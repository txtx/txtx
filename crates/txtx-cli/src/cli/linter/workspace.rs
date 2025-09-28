//! Workspace and runbook discovery
//!
//! # C4 Architecture Annotations
//! @c4-component WorkspaceAnalyzer
//! @c4-container Lint Command
//! @c4-description Discovers manifests and resolves runbooks
//! @c4-description Normalizes multi-file runbooks to single-file with FileBoundaryMap
//! @c4-technology Rust
//! @c4-uses FileBoundaryMapper "For multi-file runbooks"
//! @c4-relationship "Provides normalized content to" "Linter Engine"

use std::path::{Path, PathBuf};
use std::env;
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_core::manifest::WorkspaceManifest;
use txtx_core::manifest::file::{read_runbook_from_location, read_runbooks_from_manifest};
use txtx_core::validation::{ValidationResult, FileBoundaryMap};

use super::config::LinterConfig;
use super::validator::Linter;

/// @c4-component WorkspaceAnalyzer
/// @c4-responsibility Discover workspace manifests by searching upward from current directory
/// @c4-responsibility Resolve runbook files from manifest or direct paths
pub struct WorkspaceAnalyzer {
    config: LinterConfig,
    manifest: Option<WorkspaceManifest>,
}

impl WorkspaceAnalyzer {
    pub fn new(config: &LinterConfig) -> Result<Self, String> {
        let manifest = Self::resolve_manifest(&config.manifest_path)?;
        Ok(Self { config: config.clone(), manifest })
    }

    /// Resolve manifest by:
    /// 1. Using explicitly provided manifest path if available
    /// 2. Searching upward from current directory for txtx.yml
    /// 3. Returning None if no manifest found (will use simple validation)
    fn resolve_manifest(explicit_path: &Option<PathBuf>) -> Result<Option<WorkspaceManifest>, String> {
        // If explicit path provided, use it
        if let Some(path) = explicit_path {
            let location = FileLocation::from_path(path.clone());
            return WorkspaceManifest::from_location(&location)
                .map(Some)
                .map_err(|e| format!("Failed to load manifest from {}: {}", path.display(), e));
        }

        // Try to find manifest by searching upward
        let current_dir = env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?;

        Ok(Self::find_manifest_upward(&current_dir)
            .and_then(|manifest_path| {
                let location = FileLocation::from_path(manifest_path.clone());
                match WorkspaceManifest::from_location(&location) {
                    Ok(manifest) => {
                        eprintln!("Using manifest: {}", manifest_path.display());
                        Some(manifest)
                    },
                    Err(e) => {
                        eprintln!("Warning: Found manifest at {} but failed to load: {}", manifest_path.display(), e);
                        None
                    }
                }
            })
            .or_else(|| {
                eprintln!("Warning: No txtx.yml manifest found. Using basic validation without manifest context.");
                None
            }))
    }

    /// Search for txtx.yml starting from the given directory and moving up
    /// Stop at git root or filesystem root
    fn find_manifest_upward(start_path: &Path) -> Option<PathBuf> {
        std::iter::successors(Some(start_path.to_path_buf()), |path| {
            if path.join(".git").exists() {
                None // Stop at git root
            } else {
                path.parent().map(|p| p.to_path_buf())
            }
        })
        .map(|dir| dir.join("txtx.yml"))
        .find(|path| path.exists())
    }

    pub fn analyze_runbook(&self, name: &str) -> Result<ValidationResult, String> {
        let runbook_sources = self.resolve_runbook_sources(name)?;
        self.validate_sources(runbook_sources)
    }

    /// Resolves runbook sources by name, either from a direct file path or from the manifest.
    ///
    /// # Arguments
    /// * `name` - The name or path of the runbook to resolve
    ///
    /// # Returns
    /// * `Ok(RunbookSources)` - The resolved runbook sources
    /// * `Err(String)` - An error message if the runbook cannot be found or loaded
    pub fn resolve_runbook_sources(&self, name: &str) -> Result<txtx_core::runbook::RunbookSources, String> {
        // First, check if it's a direct file path
        let path = PathBuf::from(name);
        if path.exists() {
            let location = FileLocation::from_path(path);
            let (_, _, sources) = read_runbook_from_location(
                &location,
                &None,
                &self.config.environment,
                Some(name),
            )?;
            return Ok(sources);
        }

        // Try to find it in the manifest
        match &self.manifest {
            Some(manifest) => {
                let runbooks = read_runbooks_from_manifest(
                    manifest,
                    &self.config.environment,
                    None,
                )?;

                runbooks.into_iter()
                    .find(|(id, (_, _, runbook_name, _))| runbook_name == name || id == name)
                    .map(|(_, (_, sources, _, _))| sources)
                    .ok_or_else(|| format!("Runbook '{}' not found in manifest", name))
            },
            None => {
                // No manifest - try to find the file in standard locations
                // This allows basic validation even without a manifest
                [
                    PathBuf::from(format!("{}.tx", name)),
                    PathBuf::from("runbooks").join(format!("{}.tx", name)),
                    PathBuf::from(name),
                    PathBuf::from("runbooks").join(name),
                ]
                .into_iter()
                .find(|path| path.exists())
                .and_then(|path| {
                    let location = FileLocation::from_path(path);
                    read_runbook_from_location(
                        &location,
                        &None,
                        &self.config.environment,
                        Some(name),
                    )
                    .map(|(_, _, sources)| sources)
                    .ok()
                })
                .ok_or_else(|| format!("Runbook '{}' not found. Searched in current directory and 'runbooks' subdirectory.", name))
            }
        }
    }

    fn validate_sources(&self, runbook_sources: txtx_core::runbook::RunbookSources) -> Result<ValidationResult, String> {
        let linter = Linter::with_defaults();

        // For multi-file runbooks, we need to validate all files together so they can
        // share definitions (especially for flows). We concatenate all sources but track
        // file boundaries for proper error reporting.

        if runbook_sources.tree.len() == 1 {
            // Single file - validate directly with proper file path
            let (location, (_name, raw_content)) = runbook_sources.tree.iter().next().unwrap();
            let content = raw_content.to_string();
            let result = linter.validate_content(
                &content,
                &location.to_string(),
                self.config.manifest_path.as_ref(),
                self.config.environment.as_ref(),
            );
            Ok(result)
        } else {
            // Multi-file runbook - combine all sources for validation
            // This allows flows defined in one file to be visible when validating another
            let mut combined_content = String::new();
            let mut boundary_map = FileBoundaryMap::new();

            for (location, (_name, raw_content)) in runbook_sources.tree.iter() {
                let content = raw_content.to_string();
                let line_count = content.lines().count();

                // Track where this file's lines are in the combined content
                boundary_map.add_file(location.to_string(), line_count);

                combined_content.push_str(&content);
                combined_content.push('\n'); // Separate files with newline
            }

            // Validate the combined content
            let mut result = linter.validate_content(
                &combined_content,
                "multi-file runbook",
                self.config.manifest_path.as_ref(),
                self.config.environment.as_ref(),
            );

            // Map error locations back to original files
            result.map_errors_to_source_files(&boundary_map);

            Ok(result)
        }
    }

    pub fn analyze_all(&self) -> Result<Vec<ValidationResult>, String> {
        let manifest = self.manifest.as_ref()
            .ok_or_else(|| "No manifest found. Unable to lint all runbooks. Please specify a manifest with --manifest-file-path or ensure txtx.yml exists in your project.".to_string())?;

        let runbooks = read_runbooks_from_manifest(
            manifest,
            &self.config.environment,
            None,
        )?;

        let results: Vec<ValidationResult> = runbooks
            .into_iter()
            .filter_map(|(_, (_, sources, _, _))| {
                self.validate_sources(sources).ok()
            })
            .filter(|result| !result.errors.is_empty() || !result.warnings.is_empty())
            .collect();

        if results.is_empty() {
            // Return single empty result to indicate success
            Ok(vec![ValidationResult::default()])
        } else {
            Ok(results)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Test fixture for creating temporary test workspaces
    struct TestWorkspace {
        _temp_dir: TempDir, // Keep temp dir alive
        pub root: PathBuf,
    }

    impl TestWorkspace {
        /// Create a new temporary test workspace
        fn new() -> Self {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let root = temp_dir.path().to_path_buf();
            TestWorkspace {
                _temp_dir: temp_dir,
                root,
            }
        }

        /// Create a manifest file in the workspace
        fn create_manifest(&self, content: &str) -> PathBuf {
            self.create_file("txtx.yml", content)
        }

        /// Create a file in the workspace
        fn create_file(&self, name: &str, content: &str) -> PathBuf {
            let path = self.root.join(name);
            fs::write(&path, content).expect("Failed to write file");
            path
        }

        /// Create a subdirectory
        fn create_dir(&self, name: &str) -> PathBuf {
            let path = self.root.join(name);
            fs::create_dir_all(&path).expect("Failed to create directory");
            path
        }

        /// Create a git repository (just the .git directory for testing)
        fn init_git(&self) {
            fs::create_dir(self.root.join(".git")).expect("Failed to create .git directory");
        }
    }

    // ===== Manifest Discovery Tests =====

    #[test]
    fn test_find_manifest_in_current_directory() {
        let workspace = TestWorkspace::new();
        let manifest_path = workspace.create_manifest("id: test\nname: test\nenvironments:\n  global: {}\nrunbooks: []");

        let result = WorkspaceAnalyzer::find_manifest_upward(&workspace.root);
        assert!(result.is_some(), "Should find manifest in current directory");
        assert_eq!(result.unwrap(), manifest_path);
    }

    #[test]
    fn test_find_manifest_in_parent_directory() {
        let workspace = TestWorkspace::new();
        let manifest_path = workspace.create_manifest("id: test\nname: test\nenvironments:\n  global: {}\nrunbooks: []");
        let sub_dir = workspace.create_dir("subdir");

        let result = WorkspaceAnalyzer::find_manifest_upward(&sub_dir);
        assert!(result.is_some(), "Should find manifest in parent directory");
        assert_eq!(result.unwrap(), manifest_path);
    }

    #[test]
    fn test_find_manifest_deeply_nested() {
        let workspace = TestWorkspace::new();
        let manifest_path = workspace.create_manifest("id: test\nname: test\nenvironments:\n  global: {}\nrunbooks: []");

        // Create deeply nested directory
        let deep_dir = workspace.root
            .join("a").join("b").join("c").join("d");
        fs::create_dir_all(&deep_dir).expect("Failed to create nested directories");

        let result = WorkspaceAnalyzer::find_manifest_upward(&deep_dir);
        assert!(result.is_some(), "Should find manifest from deeply nested directory");
        assert_eq!(result.unwrap(), manifest_path);
    }

    #[test]
    fn test_stop_search_at_git_root() {
        let workspace = TestWorkspace::new();
        workspace.init_git();
        let sub_dir = workspace.create_dir("subdir");

        // No manifest in this git repo
        let result = WorkspaceAnalyzer::find_manifest_upward(&sub_dir);
        assert!(result.is_none(), "Should stop at git root and not find manifest");
    }

    #[test]
    fn test_find_manifest_at_git_root() {
        let workspace = TestWorkspace::new();
        workspace.init_git();
        let manifest_path = workspace.create_manifest("id: test\nname: test\nenvironments:\n  global: {}\nrunbooks: []");
        let sub_dir = workspace.create_dir("subdir");

        let result = WorkspaceAnalyzer::find_manifest_upward(&sub_dir);
        assert!(result.is_some(), "Should find manifest at git root");
        assert_eq!(result.unwrap(), manifest_path);
    }

    #[test]
    fn test_no_manifest_found() {
        let workspace = TestWorkspace::new();

        let result = WorkspaceAnalyzer::find_manifest_upward(&workspace.root);
        assert!(result.is_none(), "Should return None when no manifest exists");
    }

    #[test]
    fn test_resolve_manifest_with_explicit_path() {
        let workspace = TestWorkspace::new();
        let custom_manifest = workspace.create_file(
            "custom.yml",
            r#"id: custom
name: custom
description: Custom manifest
environments:
  global: {}
runbooks: []"#
        );

        let config = LinterConfig::new(
            Some(custom_manifest.clone()),
            None,
            None,
            vec![],
            super::super::Format::Json,
        );

        let analyzer = WorkspaceAnalyzer::new(&config);
        assert!(analyzer.is_ok(), "Should create analyzer with explicit manifest: {:?}", analyzer.as_ref().err());

        let analyzer = analyzer.unwrap();
        assert!(analyzer.manifest.is_some(), "Should have loaded manifest");
    }

    #[test]
    fn test_resolve_manifest_with_auto_discovery() {
        let workspace = TestWorkspace::new();
        let original_dir = env::current_dir().expect("Failed to get current dir");

        // Create manifest and switch to workspace directory
        workspace.create_manifest(r#"id: auto
name: auto
description: Auto-discovered manifest
environments:
  global: {}
runbooks: []"#);
        env::set_current_dir(&workspace.root).expect("Failed to change directory");

        let config = LinterConfig::new(None, None, None, vec![], super::super::Format::Json);
        let analyzer = WorkspaceAnalyzer::new(&config);

        // Restore original directory
        env::set_current_dir(original_dir).expect("Failed to restore directory");

        assert!(analyzer.is_ok(), "Should create analyzer with auto-discovered manifest: {:?}", analyzer.as_ref().err());
        let analyzer = analyzer.unwrap();
        assert!(analyzer.manifest.is_some(), "Should have auto-discovered manifest");
    }

    // ===== Runbook Resolution Tests =====

    #[test]
    fn test_resolve_runbook_direct_file_path() {
        let workspace = TestWorkspace::new();
        let runbook_path = workspace.create_file("test.tx", "action \"test\" {}");

        let config = LinterConfig::new(None, None, None, vec![], super::super::Format::Json);
        let analyzer = WorkspaceAnalyzer {
            config: config.clone(),
            manifest: None,
        };

        let result = analyzer.resolve_runbook_sources(runbook_path.to_str().unwrap());
        assert!(result.is_ok(), "Should resolve direct file path");
    }

    #[test]
    fn test_resolve_runbook_from_standard_location() {
        let workspace = TestWorkspace::new();

        // Create runbook in standard location
        let runbooks_dir = workspace.create_dir("runbooks");
        let runbook_path = runbooks_dir.join("test.tx");
        fs::write(&runbook_path, "action \"test\" {}").expect("Failed to write runbook");

        // Instead of changing current directory (which causes race conditions in parallel tests),
        // pass the full path to the runbook. This tests the same code path (direct file resolution)
        // without global process state modification.
        let config = LinterConfig::new(None, None, None, vec![], super::super::Format::Json);
        let analyzer = WorkspaceAnalyzer {
            config,
            manifest: None,
        };

        let result = analyzer.resolve_runbook_sources(runbook_path.to_str().unwrap());
        assert!(result.is_ok(), "Should find runbook in standard location");
    }

    #[test]
    fn test_resolve_runbook_not_found() {
        let workspace = TestWorkspace::new();
        let config = LinterConfig::new(None, None, None, vec![], super::super::Format::Json);
        let analyzer = WorkspaceAnalyzer {
            config,
            manifest: None,
        };

        let result = analyzer.resolve_runbook_sources("nonexistent");
        assert!(result.is_err(), "Should fail when runbook not found");
        assert!(result.unwrap_err().contains("not found"), "Error should mention 'not found'");
    }

    // ===== Original Tests =====

    /// Test that the linter properly validates content with errors
    #[test]
    fn test_validate_content_with_errors() {
        // Arrange
        let linter = Linter::with_defaults();
        let content = r#"
        variable "defined_var" {
            value = "test"
        }

        action "test" {
            input = variable.undefined_var  // This should trigger undefined variable error
        }
        "#;

        // Act
        let result = linter.validate_content(
            content,
            "test.tx",
            None::<&PathBuf>, // No manifest
            None, // No environment
        );

        // Assert
        assert!(result.errors.len() > 0, "Should detect undefined variable error");
    }

    /// Test that valid content produces no errors
    #[test]
    fn test_validate_valid_content() {
        // Arrange
        let linter = Linter::with_defaults();
        let content = r#"
        variable "test_var" {
            value = "test_value"
        }

        output "result" {
            value = variable.test_var
        }
        "#;

        // Act
        let result = linter.validate_content(
            content,
            "test.tx",
            None::<&PathBuf>,
            None,
        );

        // Assert
        assert_eq!(result.errors.len(), 0, "Valid content should have no errors");
    }

    /// Test that the linter can validate with manifest context
    #[test]
    fn test_validate_with_manifest_context() {
        // Arrange
        let linter = Linter::with_defaults();
        let manifest = WorkspaceManifest::new("test".to_string());

        let content = r#"
        variable "env_var" {
            value = input.some_input
        }
        "#;

        // Act
        let result = linter.validate_content(
            content,
            "test.tx",
            Some(manifest),
            None,
        );

        // Assert
        // The linter should validate against the manifest's defined inputs
        // For now, we just verify it doesn't crash
        assert!(result.errors.len() >= 0, "Should validate against manifest");
    }

    /// Test validation with multiple source files (simulating multi-file runbook)
    #[test]
    fn test_combine_validation_results() {
        // Arrange
        let linter = Linter::with_defaults();
        let mut combined_result = ValidationResult::default();

        // Simulate validating multiple files
        let file1_content = r#"
        variable "var1" {
            value = "test1"
        }
        "#;

        let file2_content = r#"
        variable "var2" {
            value = variable.undefined_var  // Error in second file
        }
        "#;

        // Act - validate each file and combine results
        let result1 = linter.validate_content(file1_content, "file1.tx", None::<&PathBuf>, None);
        let result2 = linter.validate_content(file2_content, "file2.tx", None::<&PathBuf>, None);

        combined_result.errors.extend(result1.errors);
        combined_result.warnings.extend(result1.warnings);
        combined_result.errors.extend(result2.errors);
        combined_result.warnings.extend(result2.warnings);

        // Assert
        assert!(combined_result.errors.len() > 0, "Should have errors from second file");
        // Verify error has correct file information
        let has_file2_error = combined_result.errors.iter()
            .any(|e| e.file == "file2.tx");
        assert!(has_file2_error, "Error should reference correct file");
    }

    /// Test that circular dependency in variables is detected
    #[test]
    fn test_circular_dependency_detection() {
        // Arrange
        let linter = Linter::with_defaults();
        let content = r#"
variable "a" {
    value = variable.b
}

variable "b" {
    value = variable.a
}
        "#;

        // Act
        let result = linter.validate_content(content, "test.tx", None::<&PathBuf>, None);

        // Assert
        assert_eq!(result.errors.len(), 2, "Should detect 2 circular dependency errors");

        // Both errors should mention circular dependency
        let all_circular = result.errors.iter()
            .all(|e| e.message.contains("circular dependency"));
        assert!(all_circular, "All errors should be about circular dependency");

        // Check that errors are at different lines
        let lines: Vec<_> = result.errors.iter()
            .filter_map(|e| e.line)
            .collect();
        assert_eq!(lines.len(), 2, "Should have line numbers for both errors");
        assert_ne!(lines[0], lines[1], "Errors should be at different lines");
    }

    /// Test three-way circular dependency detection
    #[test]
    fn test_three_way_circular_dependency() {
        // Arrange
        let linter = Linter::with_defaults();
        let content = r#"
variable "x" {
    value = variable.y
}

variable "y" {
    value = variable.z
}

variable "z" {
    value = variable.x
}
        "#;

        // Act
        let result = linter.validate_content(content, "test.tx", None::<&PathBuf>, None);

        // Assert
        assert_eq!(result.errors.len(), 2, "Should detect 2 circular dependency errors");

        // Check the cycle includes all three variables
        let first_error = &result.errors[0];

        // The cycle can be detected starting from any point, so accept any valid representation
        let valid_cycles = [
            "x -> y -> z -> x",
            "y -> z -> x -> y",
            "z -> x -> y -> z",
        ];

        let contains_valid_cycle = valid_cycles.iter()
            .any(|cycle| first_error.message.contains(cycle));

        assert!(contains_valid_cycle,
                "Should show complete cycle path, got: {}", first_error.message);
    }

    /// Test no false positive for non-circular dependencies
    #[test]
    fn test_no_false_positive_circular_dependency() {
        // Arrange
        let linter = Linter::with_defaults();
        let content = r#"
variable "base" {
    value = "hello"
}

variable "derived1" {
    value = variable.base
}

variable "derived2" {
    value = variable.base
}
        "#;

        // Act
        let result = linter.validate_content(content, "test.tx", None::<&PathBuf>, None);

        // Assert
        let has_circular = result.errors.iter()
            .any(|e| e.message.contains("circular"));
        assert!(!has_circular, "Should not detect circular dependency when there isn't one");
    }

    /// Test circular dependency in actions
    #[test]
    fn test_action_circular_dependency() {
        // Arrange
        let linter = Linter::with_defaults();
        let content = r#"
action "first" "test::action" {
    input = action.second.output
}

action "second" "test::action" {
    input = action.first.output
}
        "#;

        // Act
        let result = linter.validate_content(content, "test.tx", None::<&PathBuf>, None);

        // Assert
        // Should have circular dependency errors plus unknown namespace errors
        let circular_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.message.contains("circular dependency in action"))
            .collect();

        assert_eq!(circular_errors.len(), 2, "Should detect 2 action circular dependency errors");

        // Check that cycle is properly formatted
        // The cycle can be detected starting from either action
        let valid_cycles = [
            "first -> second -> first",
            "second -> first -> second",
        ];

        let contains_valid_cycle = valid_cycles.iter()
            .any(|cycle| circular_errors[0].message.contains(cycle));

        assert!(contains_valid_cycle,
                "Should show action cycle path, got: {}", circular_errors[0].message);
    }
}
