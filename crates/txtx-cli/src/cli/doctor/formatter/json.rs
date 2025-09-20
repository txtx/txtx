use serde_json::json;
use txtx_core::validation::ValidationResult;

/// Display results in JSON format
pub fn display(result: &ValidationResult) {
    let output = json!({
        "errors": result.errors.iter().map(|e| {
            json!({
                "file": e.file,
                "line": e.line,
                "column": e.column,
                "level": "error",
                "message": e.message,
                "context": e.context,
                "documentation": e.documentation_link,
            })
        }).collect::<Vec<_>>(),
        "warnings": result.warnings.iter().map(|w| {
            json!({
                "file": w.file,
                "line": w.line,
                "column": w.column,
                "level": "warning",
                "message": w.message,
                "suggestion": w.suggestion,
            })
        }).collect::<Vec<_>>(),
        "suggestions": result.suggestions.iter().map(|s| {
            json!({
                "message": s.message,
                "example": s.example,
            })
        }).collect::<Vec<_>>(),
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string()));
}
