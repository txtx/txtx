//! File boundary tracking for multi-file runbook validation
//!
//! When validating multi-file runbooks, we concatenate all source files
//! into a single string. This module provides utilities to track which
//! lines in the combined content belong to which original files, enabling
//! accurate error reporting.
//!
//! # Architecture Pattern: Normalization Strategy
//! Multi-file runbooks are normalized to single-file by:
//! 1. Concatenating all files with boundary tracking
//! 2. Running the SAME validation pipeline as single-file
//! 3. Mapping error locations back to source files
//!
//! This eliminates code duplication - one validation pipeline handles both cases.
//!
//! # C4 Architecture Annotations
//! @c4-component FileBoundaryMapper
//! @c4-container Validation Core
//! @c4-description Normalizes multi-file runbooks to single-file for validation
//! @c4-description Maps validation errors back to original source file locations
//! @c4-technology Rust
//! @c4-responsibility Track which lines in concatenated content belong to which files
//! @c4-responsibility Map error line numbers back to original source files
//! @c4-pattern Normalization Strategy (multi-file → single-file)

/// Tracks file boundaries in a combined/concatenated source file
///
/// @c4-component FileBoundaryMapper
#[derive(Debug, Clone)]
pub struct FileBoundaryMap {
    boundaries: Vec<FileBoundary>,
}

#[derive(Debug, Clone)]
struct FileBoundary {
    file_path: String,
    start_line: usize,
    line_count: usize,
}

impl FileBoundaryMap {
    /// Create a new empty boundary map
    pub fn new() -> Self {
        Self { boundaries: Vec::new() }
    }

    /// Add a file to the boundary map
    ///
    /// # Arguments
    /// * `file_path` - The path/name of the file
    /// * `line_count` - Number of lines in the file
    ///
    /// Files should be added in the same order they appear in the combined content.
    pub fn add_file(&mut self, file_path: String, line_count: usize) {
        let start_line = if let Some(last) = self.boundaries.last() {
            // Next file starts after the previous file
            // Empty files (line_count=0) still occupy at least 1 line in the concatenated content
            // +1 accounts for the newline separator we add between files
            let effective_line_count = last.line_count.max(1);
            last.start_line + effective_line_count + 1
        } else {
            // First file starts at line 1
            1
        };

        self.boundaries.push(FileBoundary {
            file_path,
            start_line,
            line_count,
        });
    }

    /// Map a line number in the combined content to its original file and line
    ///
    /// # Arguments
    /// * `combined_line` - Line number in the combined content (1-indexed)
    ///
    /// # Returns
    /// A tuple of (file_path, original_line_number)
    /// If the line can't be mapped, returns ("unknown", combined_line)
    pub fn map_line(&self, combined_line: usize) -> (String, usize) {
        for boundary in &self.boundaries {
            let end_line = boundary.start_line + boundary.line_count;

            if combined_line >= boundary.start_line && combined_line < end_line {
                // Found the file containing this line
                let original_line = combined_line - boundary.start_line + 1;
                return (boundary.file_path.clone(), original_line);
            }
        }

        // Line not found in any file (shouldn't happen in normal use)
        ("unknown".to_string(), combined_line)
    }

    /// Get the number of files tracked
    pub fn file_count(&self) -> usize {
        self.boundaries.len()
    }
}

impl Default for FileBoundaryMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_boundary_map() {
        let map = FileBoundaryMap::new();
        assert_eq!(map.file_count(), 0);

        // Mapping with no files should return unknown
        let (file, line) = map.map_line(1);
        assert_eq!(file, "unknown");
        assert_eq!(line, 1);
    }

    #[test]
    fn test_single_file() {
        let mut map = FileBoundaryMap::new();
        map.add_file("test.tx".to_string(), 5);

        assert_eq!(map.file_count(), 1);

        // Lines 1-5 should map to test.tx
        let (file, line) = map.map_line(1);
        assert_eq!(file, "test.tx");
        assert_eq!(line, 1);

        let (file, line) = map.map_line(5);
        assert_eq!(file, "test.tx");
        assert_eq!(line, 5);

        // Line 6 is past the file (separator line)
        let (file, line) = map.map_line(6);
        assert_eq!(file, "unknown");
        assert_eq!(line, 6);
    }

    #[test]
    fn test_multiple_files() {
        let mut map = FileBoundaryMap::new();
        map.add_file("flows.tx".to_string(), 3);
        map.add_file("deploy.tx".to_string(), 5);

        assert_eq!(map.file_count(), 2);

        // File 1: lines 1-3
        let (file, line) = map.map_line(1);
        assert_eq!(file, "flows.tx");
        assert_eq!(line, 1);

        let (file, line) = map.map_line(3);
        assert_eq!(file, "flows.tx");
        assert_eq!(line, 3);

        // Line 4 is separator
        let (file, line) = map.map_line(4);
        assert_eq!(file, "unknown");

        // File 2: lines 5-9 (start_line = 3 + 1 + 1 = 5)
        let (file, line) = map.map_line(5);
        assert_eq!(file, "deploy.tx");
        assert_eq!(line, 1);

        let (file, line) = map.map_line(9);
        assert_eq!(file, "deploy.tx");
        assert_eq!(line, 5);
    }

    #[test]
    fn test_three_files() {
        let mut map = FileBoundaryMap::new();
        map.add_file("flows.tx".to_string(), 3);
        map.add_file("variables.tx".to_string(), 2);
        map.add_file("deploy.tx".to_string(), 4);

        // flows.tx: lines 1-3
        // separator: line 4
        // variables.tx: lines 5-6
        // separator: line 7
        // deploy.tx: lines 8-11

        let (file, line) = map.map_line(2);
        assert_eq!(file, "flows.tx");
        assert_eq!(line, 2);

        let (file, line) = map.map_line(6);
        assert_eq!(file, "variables.tx");
        assert_eq!(line, 2);

        let (file, line) = map.map_line(10);
        assert_eq!(file, "deploy.tx");
        assert_eq!(line, 3);
    }

    #[test]
    fn test_empty_file_in_sequence() {
        let mut map = FileBoundaryMap::new();
        map.add_file("first.tx".to_string(), 2);
        map.add_file("empty.tx".to_string(), 0);
        map.add_file("third.tx".to_string(), 3);

        // first.tx: lines 1-2
        // separator: line 3
        // empty.tx: line 4 (start but no content)
        // separator: line 5
        // third.tx: lines 6-8

        let (file, line) = map.map_line(2);
        assert_eq!(file, "first.tx");
        assert_eq!(line, 2);

        // Empty file has no lines that map to it
        let (file, _) = map.map_line(4);
        assert_eq!(file, "unknown");

        let (file, line) = map.map_line(6);
        assert_eq!(file, "third.tx");
        assert_eq!(line, 1);
    }
}
