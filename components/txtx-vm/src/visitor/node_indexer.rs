use crate::{
    errors::{ConstructErrors, DiscoveryError},
    types::{Manual, PreConstructData},
};
use txtx_ext_kit::hcl::{self, structure::BlockLabel, Span};
use txtx_ext_kit::types::diagnostics::{Diagnostic, DiagnosticLevel, DiagnosticSpan};

pub fn run_node_indexer(manual: &mut Manual) -> Result<bool, String> {
    let mut has_errored = false;

    let Some(source_tree) = manual.source_tree.take() else {
        return Ok(has_errored);
    };

    for (location, (module_name, raw_content)) in source_tree.files.iter() {
        let content =
            hcl::parser::parse_body(raw_content).map_err(|e: hcl::parser::Error| e.to_string())?;

        let module_location = location.get_parent_location()?;
        let module_uri = (module_name.to_string(), module_location);

        for block in content.into_blocks() {
            let span = block.span().ok_or("unable to retrieve span".to_string())?;
            match block.ident.value().as_str() {
                "variable" => {
                    let Some(BlockLabel::String(name)) = block.labels.first() else {
                        manual.errors.push(ConstructErrors::Discovery(
                            DiscoveryError::VariableConstruct(Diagnostic {
                                location: location.clone(),
                                span: DiagnosticSpan {
                                    line_start: 0,
                                    line_end: 0,
                                    column_start: 0,
                                    column_end: 0,
                                },
                                message: "variable name missing".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            }),
                        ));
                        has_errored = true;
                        continue;
                    };
                    manual.index_node(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Variable(block),
                        span,
                        &module_uri,
                    );
                }
                "module" => {
                    let Some(BlockLabel::String(name)) = block.labels.first() else {
                        manual.errors.push(ConstructErrors::Discovery(
                            DiscoveryError::ModuleConstruct(Diagnostic {
                                location: location.clone(),
                                span: DiagnosticSpan {
                                    line_start: 0,
                                    line_end: 0,
                                    column_start: 0,
                                    column_end: 0,
                                },
                                message: "module name missing".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            }),
                        ));
                        has_errored = true;
                        continue;
                    };
                    manual.index_node(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Module(block),
                        span,
                        &module_uri,
                    );
                }
                "output" => {
                    let Some(BlockLabel::String(name)) = block.labels.first() else {
                        manual.errors.push(ConstructErrors::Discovery(
                            DiscoveryError::OutputConstruct(Diagnostic {
                                location: location.clone(),
                                span: DiagnosticSpan {
                                    line_start: 0,
                                    line_end: 0,
                                    column_start: 0,
                                    column_end: 0,
                                },
                                message: "output name missing".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            }),
                        ));
                        has_errored = true;
                        continue;
                    };
                    manual.index_node(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Output(block),
                        span,
                        &module_uri,
                    );
                }
                "import" => {
                    let Some(BlockLabel::String(name)) = block.labels.first() else {
                        manual.errors.push(ConstructErrors::Discovery(
                            DiscoveryError::ImportConstruct(Diagnostic {
                                location: location.clone(),
                                span: DiagnosticSpan {
                                    line_start: 0,
                                    line_end: 0,
                                    column_start: 0,
                                    column_end: 0,
                                },
                                message: "import name missing".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            }),
                        ));
                        has_errored = true;
                        continue;
                    };
                    manual.index_node(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Import(block),
                        span,
                        &module_uri,
                    );
                }
                "ext" => {
                    let Some(BlockLabel::String(name)) = block.labels.first() else {
                        manual.errors.push(ConstructErrors::Discovery(
                            DiscoveryError::ExtConstruct(Diagnostic {
                                location: location.clone(),
                                span: DiagnosticSpan {
                                    line_start: 0,
                                    line_end: 0,
                                    column_start: 0,
                                    column_end: 0,
                                },
                                message: "import name missing".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            }),
                        ));
                        has_errored = true;
                        continue;
                    };
                    manual.index_node(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Ext(block),
                        span,
                        &module_uri,
                    );
                }
                _ => {
                    manual.errors.push(ConstructErrors::Discovery(
                        DiscoveryError::UnknownConstruct(Diagnostic {
                            location: location.clone(),
                            span: DiagnosticSpan {
                                line_start: 0,
                                line_end: 0,
                                column_start: 0,
                                column_end: 0,
                            },
                            message: "construct unknown".to_string(),
                            level: DiagnosticLevel::Error,
                            documentation: None,
                            example: None,
                            parent_diagnostic: None,
                        }),
                    ));
                    has_errored = true;
                }
            }
        }
    }
    Ok(has_errored)
}
