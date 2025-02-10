use lsp_types::Diagnostic as LspDiagnostic;
use lsp_types::Url;
use lsp_types::{DiagnosticSeverity, Position, Range};
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::types::diagnostics::{
    Diagnostic as TxtxDiagnostic, DiagnosticLevel as TxtxLevel,
};

#[allow(unused_macros)]
#[cfg(feature = "wasm")]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[cfg(feature = "wasm")]
pub(crate) use log;

pub fn txtx_diagnostics_to_lsp_type(diagnostics: &Vec<TxtxDiagnostic>) -> Vec<LspDiagnostic> {
    let mut dst = vec![];
    for d in diagnostics {
        dst.push(txtx_diagnostic_to_lsp_type(d));
    }
    dst
}

pub fn txtx_diagnostic_to_lsp_type(diagnostic: &TxtxDiagnostic) -> LspDiagnostic {
    let range = match &diagnostic.span {
        None => Range::default(),
        Some(span) => Range {
            start: Position { line: span.line_start - 1, character: span.column_start - 1 },
            end: Position { line: span.line_end - 1, character: span.column_end },
        },
    };
    // TODO(lgalabru): add hint for contracts not found errors
    LspDiagnostic {
        range,
        severity: match diagnostic.level {
            TxtxLevel::Error => Some(DiagnosticSeverity::ERROR),
            TxtxLevel::Warning => Some(DiagnosticSeverity::WARNING),
            TxtxLevel::Note => Some(DiagnosticSeverity::INFORMATION),
        },
        code: None,
        code_description: None,
        source: Some("txtx".to_string()),
        message: diagnostic.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

pub fn get_manifest_location(text_document_uri: &Url) -> Option<FileLocation> {
    let file_location = text_document_uri.to_string();
    if !file_location.ends_with("txtx.yml") {
        return None;
    }
    FileLocation::try_parse(&file_location, None)
}

pub fn get_runbook_location(text_document_uri: &Url) -> Option<FileLocation> {
    let file_location = text_document_uri.to_string();
    if !file_location.ends_with(".tx") {
        return None;
    }
    FileLocation::try_parse(&file_location, None)
}
