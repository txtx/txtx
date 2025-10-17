//! Unified types for source location tracking and reference collection
//!
//! This module provides shared types used across the runbook collector,
//! validation system, and LSP implementation to track source locations
//! and references in txtx files.
//!
//! @c4-component SourceLocationMapper
//! @c4-container Runbook Core
//! @c4-description Shared location tracking and span-to-position mapping
//! @c4-technology Rust
//! @c4-responsibility Track source locations (file, line, column) across the codebase
//! @c4-responsibility Convert byte offsets to line/column positions
//! @c4-responsibility Provide context about where references appear in HCL structure
//! @c4-relationship "Used by" "Runbook Collector"
//! @c4-relationship "Used by" "HCL Validator"
//! @c4-relationship "Used by" "Variable Extractor"

use std::ops::Range;

/// Represents a specific location in a source file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// The file path
    pub file: String,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(file: String, line: usize, column: usize) -> Self {
        Self { file, line, column }
    }

    /// Create a location at the start of a file (1, 1)
    pub fn at_start(file: String) -> Self {
        Self { file, line: 1, column: 1 }
    }

    /// Create a location without file context
    pub fn without_file(line: usize, column: usize) -> Self {
        Self {
            file: String::new(),
            line,
            column,
        }
    }
}

/// Maps source spans (byte offsets) to line/column positions
pub struct SourceMapper<'a> {
    source: &'a str,
}

impl<'a> SourceMapper<'a> {
    /// Create a new source mapper for the given source text
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }

    /// Convert a span (byte range) to a source location
    pub fn span_to_location(&self, span: &Range<usize>, file: String) -> SourceLocation {
        let (line, column) = self.span_to_position(span);
        SourceLocation::new(file, line, column)
    }

    /// Convert a span to line and column (1-based)
    pub fn span_to_position(&self, span: &Range<usize>) -> (usize, usize) {
        let start = span.start;
        let mut line = 1;
        let mut col = 1;

        for (i, ch) in self.source.char_indices() {
            if i >= start {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        (line, col)
    }

    /// Convert an optional span to a location, returning a default if None
    pub fn optional_span_to_location(
        &self,
        span: Option<&Range<usize>>,
        file: String,
    ) -> SourceLocation {
        match span {
            Some(s) => self.span_to_location(s, file),
            None => SourceLocation::at_start(file),
        }
    }

    /// Convert an optional span to position, returning (1, 1) if None
    pub fn optional_span_to_position(&self, span: Option<&Range<usize>>) -> (usize, usize) {
        span.map(|s| self.span_to_position(s)).unwrap_or((1, 1))
    }
}

/// Context of where a reference or definition appears in the HCL structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockContext {
    /// Inside an action block
    Action(String),
    /// Inside a variable block
    Variable(String),
    /// Inside a signer block
    Signer(String),
    /// Inside an output block
    Output(String),
    /// Inside a flow block
    Flow(String),
    /// Inside an addon block
    Addon(String),
    /// Unknown or top-level context
    Unknown,
}

impl BlockContext {
    /// Extract the name from the context if available
    pub fn name(&self) -> Option<&str> {
        match self {
            BlockContext::Action(name)
            | BlockContext::Variable(name)
            | BlockContext::Signer(name)
            | BlockContext::Output(name)
            | BlockContext::Flow(name)
            | BlockContext::Addon(name) => Some(name),
            BlockContext::Unknown => None,
        }
    }

    /// Get the block type as a string
    pub fn block_type(&self) -> &str {
        use crate::types::ConstructType;

        match self {
            BlockContext::Action(_) => ConstructType::ACTION,
            BlockContext::Variable(_) => ConstructType::VARIABLE,
            BlockContext::Signer(_) => ConstructType::SIGNER,
            BlockContext::Output(_) => ConstructType::OUTPUT,
            BlockContext::Flow(_) => ConstructType::FLOW,
            BlockContext::Addon(_) => ConstructType::ADDON,
            BlockContext::Unknown => "unknown",
        }
    }
}

/// Type of reference being tracked
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceType {
    /// Reference to an input (input.*)
    Input,
    /// Reference to a variable (var.* or variable.*)
    Variable,
    /// Reference to an action (action.*)
    Action,
    /// Reference to a signer (signer.*)
    Signer,
    /// Reference to a flow input (flow.*)
    FlowInput,
    /// Reference to an output (output.*)
    Output,
}

/// A reference to an input, variable, or other construct in the runbook
#[derive(Debug, Clone)]
pub struct InputReference {
    /// The name being referenced (e.g., "api_key" in input.api_key)
    pub name: String,
    /// Full path as it appears (e.g., "input.api_key")
    pub full_path: String,
    /// Location where the reference appears
    pub location: SourceLocation,
    /// Context where the reference is used
    pub context: BlockContext,
    /// Type of reference
    pub reference_type: ReferenceType,
}

impl InputReference {
    /// Create a new input reference
    pub fn new(
        name: String,
        full_path: String,
        location: SourceLocation,
        context: BlockContext,
        reference_type: ReferenceType,
    ) -> Self {
        Self {
            name,
            full_path,
            location,
            context,
            reference_type,
        }
    }

    /// Create an input reference (input.*)
    pub fn input(name: String, location: SourceLocation, context: BlockContext) -> Self {
        let full_path = format!("input.{}", name);
        Self::new(name, full_path, location, context, ReferenceType::Input)
    }

    /// Create a variable reference (var.* or variable.*)
    pub fn variable(name: String, location: SourceLocation, context: BlockContext) -> Self {
        let full_path = format!("var.{}", name);
        Self::new(name, full_path, location, context, ReferenceType::Variable)
    }

    /// Create a flow input reference (flow.*)
    pub fn flow_input(name: String, location: SourceLocation, context: BlockContext) -> Self {
        let full_path = format!("flow.{}", name);
        Self::new(name, full_path, location, context, ReferenceType::FlowInput)
    }

    /// Create an action reference (action.*)
    pub fn action(name: String, location: SourceLocation, context: BlockContext) -> Self {
        let full_path = format!("action.{}", name);
        Self::new(name, full_path, location, context, ReferenceType::Action)
    }

    /// Create a signer reference (signer.*)
    pub fn signer(name: String, location: SourceLocation, context: BlockContext) -> Self {
        let full_path = format!("signer.{}", name);
        Self::new(name, full_path, location, context, ReferenceType::Signer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location_new() {
        let loc = SourceLocation::new("test.tx".to_string(), 10, 5);
        assert_eq!(loc.file, "test.tx");
        assert_eq!(loc.line, 10);
        assert_eq!(loc.column, 5);
    }

    #[test]
    fn test_source_location_at_start() {
        let loc = SourceLocation::at_start("test.tx".to_string());
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 1);
    }

    #[test]
    fn test_source_mapper_simple() {
        let source = "hello world";
        let mapper = SourceMapper::new(source);

        let (line, col) = mapper.span_to_position(&(0..5));
        assert_eq!(line, 1);
        assert_eq!(col, 1);

        let (line, col) = mapper.span_to_position(&(6..11));
        assert_eq!(line, 1);
        assert_eq!(col, 7);
    }

    #[test]
    fn test_source_mapper_multiline() {
        let source = "line 1\nline 2\nline 3";
        let mapper = SourceMapper::new(source);

        // Start of line 1
        let (line, col) = mapper.span_to_position(&(0..1));
        assert_eq!(line, 1);
        assert_eq!(col, 1);

        // Start of line 2 (after first \n at position 6)
        let (line, col) = mapper.span_to_position(&(7..8));
        assert_eq!(line, 2);
        assert_eq!(col, 1);

        // Start of line 3 (after second \n at position 13)
        let (line, col) = mapper.span_to_position(&(14..15));
        assert_eq!(line, 3);
        assert_eq!(col, 1);
    }

    #[test]
    fn test_source_mapper_newline_boundary() {
        let source = "abc\ndefg";
        let mapper = SourceMapper::new(source);

        // Just before newline
        let (line, col) = mapper.span_to_position(&(3..4));
        assert_eq!(line, 1);
        assert_eq!(col, 4);

        // Just after newline
        let (line, col) = mapper.span_to_position(&(4..5));
        assert_eq!(line, 2);
        assert_eq!(col, 1);
    }

    #[test]
    fn test_source_mapper_optional_none() {
        let source = "test";
        let mapper = SourceMapper::new(source);

        let loc = mapper.optional_span_to_location(None, "test.tx".to_string());
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 1);
    }

    #[test]
    fn test_block_context_name() {
        use crate::types::ConstructType;

        let ctx = BlockContext::Action("deploy".to_string());
        assert_eq!(ctx.name(), Some("deploy"));
        assert_eq!(ctx.block_type(), ConstructType::ACTION);

        let ctx = BlockContext::Unknown;
        assert_eq!(ctx.name(), None);
        assert_eq!(ctx.block_type(), "unknown");
    }

    #[test]
    fn test_input_reference_constructors() {
        let loc = SourceLocation::new("test.tx".to_string(), 5, 10);
        let ctx = BlockContext::Action("deploy".to_string());

        let input_ref = InputReference::input("api_key".to_string(), loc.clone(), ctx.clone());
        assert_eq!(input_ref.name, "api_key");
        assert_eq!(input_ref.full_path, "input.api_key");
        assert_eq!(input_ref.reference_type, ReferenceType::Input);

        let var_ref = InputReference::variable("my_var".to_string(), loc.clone(), ctx.clone());
        assert_eq!(var_ref.full_path, "var.my_var");
        assert_eq!(var_ref.reference_type, ReferenceType::Variable);

        let flow_ref = InputReference::flow_input("chain_id".to_string(), loc.clone(), ctx);
        assert_eq!(flow_ref.full_path, "flow.chain_id");
        assert_eq!(flow_ref.reference_type, ReferenceType::FlowInput);
    }

    #[test]
    fn test_block_context_equality() {
        let ctx1 = BlockContext::Action("deploy".to_string());
        let ctx2 = BlockContext::Action("deploy".to_string());
        let ctx3 = BlockContext::Action("other".to_string());

        assert_eq!(ctx1, ctx2);
        assert_ne!(ctx1, ctx3);
    }
}
