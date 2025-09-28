//! LSP utility functions

pub mod environment;
pub mod file_scanner;

use lsp_server::{RequestId, Response};
use lsp_types::*;
use serde::de::DeserializeOwned;

/// Cast an LSP request to a specific type
#[allow(dead_code)]
pub fn cast_request<R>(
    req: lsp_server::Request,
) -> Result<(RequestId, R::Params), (RequestId, serde_json::Error)>
where
    R: lsp_types::request::Request,
    R::Params: DeserializeOwned,
{
    match serde_json::from_value::<R::Params>(req.params) {
        Ok(params) => Ok((req.id, params)),
        Err(e) => Err((req.id, e)),
    }
}

/// Create an error response for invalid requests
#[allow(dead_code)]
pub fn create_error_response(id: RequestId, message: &str) -> Response {
    Response {
        id,
        result: None,
        error: Some(lsp_server::ResponseError {
            code: lsp_server::ErrorCode::InvalidRequest as i32,
            message: message.to_string(),
            data: None,
        }),
    }
}

/// Convert a position in text to a byte offset
#[allow(dead_code)]
pub fn position_to_offset(text: &str, position: Position) -> Option<usize> {
    let mut line_num = 0;
    let mut char_num = 0;

    for (idx, ch) in text.char_indices() {
        if line_num == position.line as usize && char_num == position.character as usize {
            return Some(idx);
        }

        if ch == '\n' {
            line_num += 1;
            char_num = 0;
        } else {
            char_num += 1;
        }
    }

    // Handle position at end of file
    if line_num == position.line as usize && char_num == position.character as usize {
        Some(text.len())
    } else {
        None
    }
}

/// Convert a byte offset to a position in text
#[allow(dead_code)]
pub fn offset_to_position(text: &str, offset: usize) -> Position {
    let mut line = 0;
    let mut character = 0;

    for (idx, ch) in text.char_indices() {
        if idx >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    Position { line, character }
}

/// Create a diagnostic from a simple error message
#[allow(dead_code)]
pub fn simple_diagnostic(
    range: Range,
    message: String,
    severity: DiagnosticSeverity,
) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(severity),
        code: None,
        code_description: None,
        source: Some("txtx".to_string()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }
}
