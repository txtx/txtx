//! HCL AST-based parsing for LSP operations.
//!
//! This module provides LSP-specific helpers for working with the hcl-edit AST,
//! replacing regex-based parsing with proper AST traversal.
//!
//! ## Key Features
//!
//! - Convert HCL spans to LSP positions and ranges
//! - Extract references at cursor positions
//! - Find all occurrences of references using visitor pattern
//! - Support for all txtx reference types (input, variable, action, signer, etc.)

use lsp_types::{Position, Range};
use std::str::FromStr;
use txtx_addon_kit::hcl::{
    expr::{Expression, Traversal, TraversalOperator},
    structure::{Block, BlockLabel, Body},
    visit::{visit_block, visit_expr, Visit},
    Span,
};

/// Reference types in txtx runbooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reference {
    /// Input reference: `input.name`
    Input(String),
    /// Variable reference: `variable.name` or `var.name`
    Variable(String),
    /// Action reference: `action.name`
    Action(String),
    /// Signer reference: `signer.name`
    Signer(String),
    /// Output reference: `output.name`
    Output(String),
    /// Flow reference by name: `flow("name")` (not commonly used)
    Flow(String),
    /// Flow field reference: `flow.field_name` - references a field in any flow
    FlowField(String),
}

impl Reference {
    /// Get the reference name regardless of type.
    pub fn name(&self) -> &str {
        match self {
            Reference::Input(name)
            | Reference::Variable(name)
            | Reference::Action(name)
            | Reference::Signer(name)
            | Reference::Output(name)
            | Reference::Flow(name)
            | Reference::FlowField(name) => name,
        }
    }

    /// Get the reference type as a string.
    pub fn type_name(&self) -> &'static str {
        match self {
            Reference::Input(_) => "input",
            Reference::Variable(_) => "variable",
            Reference::Action(_) => "action",
            Reference::Signer(_) => "signer",
            Reference::Output(_) => "output",
            Reference::Flow(_) => "flow",
            Reference::FlowField(_) => "flow_field",
        }
    }

    /// Determine if this reference type is workspace-scoped or runbook-scoped.
    ///
    /// Workspace-scoped references (Input, Signer) can reference definitions
    /// from any runbook in the workspace. Runbook-scoped references (Variable,
    /// Flow, Action, Output) can only reference definitions within the same runbook.
    pub fn is_workspace_scoped(&self) -> bool {
        matches!(self, Reference::Input(_) | Reference::Signer(_))
    }

    /// Check if this reference matches a block definition.
    ///
    /// Returns true if the block type and name match this reference.
    /// For example, `Reference::Variable("my_var")` matches a block with
    /// `block_type = "variable"` and `name = "my_var"`.
    fn matches_block(&self, name: &str, block_type: &str) -> bool {
        match (self, block_type) {
            (Reference::Variable(n), "variable") |
            (Reference::Action(n), "action") |
            (Reference::Signer(n), "signer") |
            (Reference::Output(n), "output") |
            (Reference::Flow(n), "flow") => n == name,
            _ => false,
        }
    }
}

/// Convert byte offset in source to line/column position.
///
/// Returns 0-indexed line and character positions suitable for LSP.
fn byte_offset_to_position(source: &str, offset: usize) -> Position {
    let (line, character) = source[..offset.min(source.len())]
        .char_indices()
        .fold((0, 0), |(line, col), (_, ch)| {
            if ch == '\n' {
                (line + 1, 0)
            } else {
                (line, col + 1)
            }
        });

    Position {
        line: line as u32,
        character: character as u32,
    }
}

/// Convert hcl-edit span (byte range) to LSP Position.
pub fn span_to_lsp_position(source: &str, span: &std::ops::Range<usize>) -> Position {
    byte_offset_to_position(source, span.start)
}

/// Convert hcl-edit span (byte range) to LSP Range.
pub fn span_to_lsp_range(source: &str, span: &std::ops::Range<usize>) -> Range {
    Range {
        start: byte_offset_to_position(source, span.start),
        end: byte_offset_to_position(source, span.end),
    }
}

/// Convert LSP Position to byte offset in source.
fn position_to_byte_offset(source: &str, position: Position) -> Option<usize> {
    let mut current_line = 0u32;
    let mut current_col = 0u32;

    for (byte_idx, ch) in source.char_indices() {
        if current_line == position.line && current_col == position.character {
            return Some(byte_idx);
        }

        if ch == '\n' {
            current_line += 1;
            current_col = 0;
        } else {
            current_col += 1;
        }
    }

    // Handle position at end of file
    if current_line == position.line && current_col == position.character {
        Some(source.len())
    } else {
        None
    }
}

/// Extract reference at cursor position using AST (strict mode).
///
/// This function parses the source and finds the AST node at the given position,
/// then determines what reference the cursor is on.
///
/// **Strict mode**: Only returns a reference if the cursor is precisely on:
/// - The identifier part of a traversal (e.g., `name` in `variable.name`)
/// - A block label (e.g., `"name"` in `variable "name"`)
///
/// Use `extract_reference_at_position_lenient()` for more forgiving cursor detection.
pub fn extract_reference_at_position(
    source: &str,
    position: Position,
) -> Option<(Reference, Range)> {
    let body = Body::from_str(source).ok()?;
    let byte_offset = position_to_byte_offset(source, position)?;

    let mut finder = ReferenceFinder {
        source,
        target_offset: byte_offset,
        found: None,
    };

    finder.visit_body(&body);
    finder.found
}

/// Extract reference at cursor position with lenient matching (AST + regex fallback).
///
/// This function tries AST-based extraction first, then falls back to regex patterns
/// for cases where the cursor is on the namespace prefix (e.g., `variable` in `variable.name`).
///
/// **Lenient mode**: Returns a reference if the cursor is anywhere on:
/// - The full traversal expression (e.g., anywhere on `variable.name`)
/// - A block label definition
/// - Incomplete/malformed HCL that AST can't parse
///
/// This is the recommended function for LSP handlers where UX requires forgiving cursor detection.
pub fn extract_reference_at_position_lenient(
    source: &str,
    position: Position,
) -> Option<(Reference, Range)> {
    // Try strict AST-based extraction first
    if let Some(result) = extract_reference_at_position(source, position) {
        return Some(result);
    }

    // Fallback to pattern matching
    let line = source.lines().nth(position.line as usize)?;

    find_definition_reference(source, line, position)
        .or_else(|| find_traversal_reference(line, position))
}

/// Pattern definitions for block definitions (variable "name", action "name", etc.)
static DEFINITION_PATTERNS: &[(&str, fn(&str) -> Reference)] = &[
    (r#"variable\s+"([^"]+)""#, |s| Reference::Variable(s.to_string())),
    (r#"action\s+"([^"]+)""#, |s| Reference::Action(s.to_string())),
    (r#"signer\s+"([^"]+)""#, |s| Reference::Signer(s.to_string())),
    (r#"output\s+"([^"]+)""#, |s| Reference::Output(s.to_string())),
    (r#"flow\s+"([^"]+)""#, |s| Reference::Flow(s.to_string())),
];

/// Pattern definitions for traversal expressions (input.name, variable.name, etc.)
static TRAVERSAL_PATTERNS: &[(&str, fn(&str) -> Reference)] = &[
    (r"input\.(\w+)", |s| Reference::Input(s.to_string())),
    (r"variable\.(\w+)", |s| Reference::Variable(s.to_string())),
    (r"var\.(\w+)", |s| Reference::Variable(s.to_string())),
    (r"action\.(\w+)", |s| Reference::Action(s.to_string())),
    (r"signer\.(\w+)", |s| Reference::Signer(s.to_string())),
    (r"output\.(\w+)", |s| Reference::Output(s.to_string())),
    (r"flow\.(\w+)", |s| Reference::Flow(s.to_string())),
];

/// Find reference in block definition (e.g., variable "my_var" { ... })
fn find_definition_reference(
    source: &str,
    line: &str,
    position: Position,
) -> Option<(Reference, Range)> {
    use regex::Regex;
    use std::sync::OnceLock;

    // Compile regexes once
    static COMPILED: OnceLock<Vec<(Regex, fn(&str) -> Reference)>> = OnceLock::new();
    let compiled = COMPILED.get_or_init(|| {
        DEFINITION_PATTERNS
            .iter()
            .filter_map(|(pattern, ctor)| {
                Regex::new(pattern).ok().map(|re| (re, *ctor))
            })
            .collect()
    });

    compiled.iter().find_map(|(re, constructor)| {
        re.captures(line).and_then(|capture| {
            let name_match = capture.get(1)?;
            let char_range = (name_match.start() as u32)..(name_match.end() as u32);

            if char_range.contains(&position.character) {
                let reference = constructor(name_match.as_str());
                let byte_range = char_to_byte_range(line, &char_range);
                let range = span_to_lsp_range(source, &byte_range);
                Some((reference, range))
            } else {
                None
            }
        })
    })
}

/// Find reference in traversal expression (e.g., input.my_var)
fn find_traversal_reference(
    line: &str,
    position: Position,
) -> Option<(Reference, Range)> {
    use regex::Regex;
    use std::sync::OnceLock;

    static COMPILED: OnceLock<Vec<(Regex, fn(&str) -> Reference)>> = OnceLock::new();
    let compiled = COMPILED.get_or_init(|| {
        TRAVERSAL_PATTERNS
            .iter()
            .filter_map(|(pattern, ctor)| {
                Regex::new(pattern).ok().map(|re| (re, *ctor))
            })
            .collect()
    });

    compiled.iter().find_map(|(re, constructor)| {
        re.captures(line).and_then(|capture| {
            let full_match = capture.get(0)?;
            let full_range = (full_match.start() as u32)..(full_match.end() as u32);

            if full_range.contains(&position.character) {
                let name_match = capture.get(1)?;
                let reference = constructor(name_match.as_str());
                // Return identifier span only (not full traversal)
                let range = Range {
                    start: Position {
                        line: position.line,
                        character: name_match.start() as u32
                    },
                    end: Position {
                        line: position.line,
                        character: name_match.end() as u32
                    },
                };
                Some((reference, range))
            } else {
                None
            }
        })
    })
}

/// Convert character range to byte range in a line
fn char_to_byte_range(line: &str, char_range: &std::ops::Range<u32>) -> std::ops::Range<usize> {
    let byte_start = line.chars().take(char_range.start as usize).map(|c| c.len_utf8()).sum();
    let byte_end = line.chars().take(char_range.end as usize).map(|c| c.len_utf8()).sum();
    byte_start..byte_end
}

/// Visitor that finds references at a specific byte offset.
struct ReferenceFinder<'a> {
    source: &'a str,
    target_offset: usize,
    found: Option<(Reference, Range)>,
}

impl<'a> ReferenceFinder<'a> {
    fn span_contains(&self, span: &std::ops::Range<usize>) -> bool {
        span.contains(&self.target_offset)
    }

    fn check_block_label(&mut self, block: &Block) {
        let Some(BlockLabel::String(name_str)) = block.labels.first() else {
            return;
        };

        let Some(span) = name_str.span().filter(|s| self.span_contains(s)) else {
            return;
        };

        use Reference::*;
        let reference = match block.ident.as_str() {
            "variable" => Variable,
            "action" => Action,
            "signer" => Signer,
            "output" => Output,
            "flow" => Flow,
            _ => return,
        }(name_str.as_str().to_string());

        self.found = Some((reference, span_to_lsp_range(self.source, &span)));
    }
}

impl<'a> Visit for ReferenceFinder<'a> {
    fn visit_block(&mut self, block: &Block) {
        if self.found.is_some() {
            return; // Stop immediately - don't even check labels
        }

        // Check if cursor is on block label (definition)
        self.check_block_label(block);

        if self.found.is_none() {
            visit_block(self, block);
        }
    }

    fn visit_expr(&mut self, expr: &Expression) {
        if self.found.is_some() {
            return;
        }

        // Check if this is a traversal expression (e.g., input.foo, variable.bar)
        if let Expression::Traversal(traversal) = expr {
            if let Some(span) = traversal.span().filter(|s| self.span_contains(s)) {
                self.found = extract_reference_from_traversal(self.source, traversal);
            }
        }

        if self.found.is_none() {
            visit_expr(self, expr);
        }
    }
}

/// Extract reference information from a Traversal expression.
///
/// Handles patterns like:
/// - `input.name` -> Input("name"), returns span of full "input.name"
/// - `variable.name` or `var.name` -> Variable("name"), returns span of full "variable.name"
/// - `action.name` -> Action("name"), returns span of full "action.name"
///
/// Returns the full traversal span (namespace + identifier) for better cursor detection context.
fn extract_reference_from_traversal(
    source: &str,
    traversal: &Traversal,
) -> Option<(Reference, Range)> {
    // Extract the root variable name
    let root = traversal.expr.as_variable()?.as_str();

    // Extract the first attribute access
    let first_attr = traversal
        .operators
        .first()
        .and_then(|op| match op.value() {
            TraversalOperator::GetAttr(ident) => Some(ident.as_str()),
            _ => None,
        })?;

    // Determine reference type from root
    let reference = match root {
        "input" => Reference::Input(first_attr.to_string()),
        "variable" | "var" => Reference::Variable(first_attr.to_string()),
        "action" => Reference::Action(first_attr.to_string()),
        "signer" => Reference::Signer(first_attr.to_string()),
        "output" => Reference::Output(first_attr.to_string()),
        // Flow field access: flow.chain_id, flow.api_url, etc.
        // This represents accessing a field from any flow (not a specific flow by name)
        "flow" => Reference::FlowField(first_attr.to_string()),
        _ => return None,
    };

    // Return just the identifier span (not including namespace/dot) for precise editing
    // This ensures rename operations only replace the name part, not the prefix
    let first_op = traversal.operators.first()?;
    let ident_span = match first_op.value() {
        TraversalOperator::GetAttr(ident) => ident.span()?,
        _ => return None,
    };
    let range = span_to_lsp_range(source, &ident_span);

    Some((reference, range))
}

/// Find all occurrences of a reference in the source using visitor pattern.
pub fn find_all_occurrences(source: &str, reference: &Reference) -> Vec<Range> {
    let Ok(body) = Body::from_str(source) else {
        return Vec::new();
    };

    let mut finder = OccurrenceFinder {
        source,
        reference,
        occurrences: Vec::new(),
    };

    finder.visit_body(&body);
    finder.occurrences
}

/// Visitor that collects all occurrences of a specific reference.
struct OccurrenceFinder<'a> {
    source: &'a str,
    reference: &'a Reference,
    occurrences: Vec<Range>,
}

impl<'a> Visit for OccurrenceFinder<'a> {
    fn visit_block(&mut self, block: &Block) {
        let Some(BlockLabel::String(name_str)) = block.labels.first() else {
            visit_block(self, block);
            return;
        };

        if self.reference.matches_block(name_str.as_str(), block.ident.as_str()) {
            if let Some(span) = name_str.span() {
                self.occurrences.push(span_to_lsp_range(self.source, &span));
            }
        }

        visit_block(self, block);
    }

    fn visit_expr(&mut self, expr: &Expression) {
        if let Expression::Traversal(traversal) = expr {
            if let Some((found_ref, range)) = extract_reference_from_traversal(self.source, traversal) {
                if found_ref == *self.reference {
                    self.occurrences.push(range);
                }
            }
        }

        visit_expr(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_offset_to_position() {
        let source = "line 0\nline 1\nline 2";

        // Start of file
        assert_eq!(byte_offset_to_position(source, 0), Position { line: 0, character: 0 });

        // Middle of first line
        assert_eq!(byte_offset_to_position(source, 3), Position { line: 0, character: 3 });

        // Start of second line
        assert_eq!(byte_offset_to_position(source, 7), Position { line: 1, character: 0 });

        // Start of third line
        assert_eq!(byte_offset_to_position(source, 14), Position { line: 2, character: 0 });
    }

    #[test]
    fn test_position_to_byte_offset() {
        let source = "line 0\nline 1\nline 2";

        assert_eq!(position_to_byte_offset(source, Position { line: 0, character: 0 }), Some(0));
        assert_eq!(position_to_byte_offset(source, Position { line: 0, character: 3 }), Some(3));
        assert_eq!(position_to_byte_offset(source, Position { line: 1, character: 0 }), Some(7));
        assert_eq!(position_to_byte_offset(source, Position { line: 2, character: 0 }), Some(14));
    }

    #[test]
    fn test_extract_input_reference() {
        let source = r#"
action "test" "evm::call" {
    chain_id = input.network_id
}
"#;
        // Position on "network_id" part
        let position = Position { line: 2, character: 22 };

        let result = extract_reference_at_position(source, position);
        assert!(result.is_some());

        let (reference, _range) = result.unwrap();
        assert_eq!(reference, Reference::Input("network_id".to_string()));
    }

    #[test]
    fn test_extract_variable_reference() {
        let source = r#"
action "test" "evm::call" {
    count = variable.my_count
}
"#;
        let position = Position { line: 2, character: 23 };

        let result = extract_reference_at_position(source, position);
        assert!(result.is_some());

        let (reference, _range) = result.unwrap();
        assert_eq!(reference, Reference::Variable("my_count".to_string()));
    }

    #[test]
    fn test_extract_from_definition() {
        let source = r#"variable "my_var" { value = 10 }"#;
        let position = Position { line: 0, character: 11 }; // On "my_var"

        let result = extract_reference_at_position(source, position);
        assert!(result.is_some());

        let (reference, _range) = result.unwrap();
        assert_eq!(reference, Reference::Variable("my_var".to_string()));
    }

    #[test]
    fn test_find_all_variable_occurrences() {
        let source = r#"
variable "count" { value = 10 }
action "test" "evm::call" {
    num = variable.count
    total = var.count + 5
}
"#;
        let reference = Reference::Variable("count".to_string());
        let occurrences = find_all_occurrences(source, &reference);

        // Should find: definition + 2 references
        assert_eq!(occurrences.len(), 3, "Expected 3 occurrences, found {}", occurrences.len());
    }

    #[test]
    fn test_find_all_input_occurrences() {
        let source = r#"
action "test1" "evm::call" {
    chain = input.network_id
}
action "test2" "evm::call" {
    chain = input.network_id
}
"#;
        let reference = Reference::Input("network_id".to_string());
        let occurrences = find_all_occurrences(source, &reference);

        // Should find 2 references (no definition for inputs)
        assert_eq!(occurrences.len(), 2);
    }

    #[test]
    fn test_extract_cursor_on_namespace_prefix() {
        // Test that lenient mode finds references with cursor anywhere on "variable.my_var"
        let source = "value = variable.my_var + 1";

        // Cursor on 'v' in 'variable' (start of traversal) - lenient mode needed
        let pos1 = Position { line: 0, character: 8 };
        let result1 = extract_reference_at_position_lenient(source, pos1);
        assert!(result1.is_some(), "Should find reference with cursor at start: {:?}", result1);

        // Cursor on 'b' in 'variable' (middle of prefix) - lenient mode needed
        let pos2 = Position { line: 0, character: 12 };
        let result2 = extract_reference_at_position_lenient(source, pos2);
        assert!(result2.is_some(), "Should find reference with cursor on prefix: {:?}", result2);

        // Cursor on '.' (dot) - lenient mode needed
        let pos3 = Position { line: 0, character: 16 };
        let result3 = extract_reference_at_position_lenient(source, pos3);
        assert!(result3.is_some(), "Should find reference with cursor on dot: {:?}", result3);

        // Cursor on 'm' in 'my_var' (identifier) - both modes work, lenient calls strict
        let pos4 = Position { line: 0, character: 17 };
        let result4 = extract_reference_at_position_lenient(source, pos4);
        assert!(result4.is_some(), "Should find reference with cursor on identifier: {:?}", result4);
    }
}
