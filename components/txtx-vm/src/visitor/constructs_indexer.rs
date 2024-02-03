use std::collections::{HashSet, VecDeque};

use crate::{
    errors::{ConstructErrors, DiscoveryError},
    types::{Manual, PreConstructData},
};
use txtx_ext_kit::types::diagnostics::{Diagnostic, DiagnosticLevel, DiagnosticSpan};
use txtx_ext_kit::{
    hcl::{self, structure::BlockLabel, Span},
    helpers::{
        fs::{get_txtx_files_paths, FileLocation},
        hcl::visit_required_string_literal_attribute,
    },
};

pub fn run_constructs_indexer(manual: &mut Manual) -> Result<bool, String> {
    let mut has_errored = false;

    let Some(source_tree) = manual.source_tree.take() else {
        return Ok(has_errored);
    };

    let mut sources = VecDeque::new();
    // todo(lgalabru): basing files_visited on path is fragile, we should hash file contents instead
    let mut files_visited = HashSet::new();
    for (location, (module_name, raw_content)) in source_tree.files.iter() {
        files_visited.insert(location);
        sources.push_back((location.clone(), module_name.clone(), raw_content.clone()));
    }

    while let Some((location, package_name, raw_content)) = sources.pop_front() {
        let content =
            hcl::parser::parse_body(&raw_content).map_err(|e: hcl::parser::Error| e.to_string())?;

        let package_location = location.get_parent_location()?;

        for block in content.into_blocks() {
            let span = block.span().ok_or("unable to retrieve span".to_string())?;
            match block.ident.value().as_str() {
                "import" => {
                    // imports are the only constructs that we need to process in this step
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

                    let path = visit_required_string_literal_attribute("path", &block).unwrap(); // todo(lgalabru)
                    println!("Loading {} at path ({path})", name.to_string());

                    // todo(lgalabru): revisit this approach, filesystem access needs to be abstracted.
                    let mut imported_package_location = location.get_parent_location().unwrap();
                    imported_package_location.append_path(&path).unwrap();

                    match std::fs::read_dir(imported_package_location.to_string()) {
                        Ok(_) => {
                            let files =
                                get_txtx_files_paths(&imported_package_location.to_string())
                                    .map_err(|e| {
                                        format!("unable to read directory: {}", e.to_string())
                                    })?;
                            for file_path in files.into_iter() {
                                let file_location = FileLocation::from_path(file_path);
                                if !files_visited.contains(&file_location) {
                                    let raw_content = file_location.read_content_as_utf8()?;
                                    let module_name = name.to_string();
                                    sources.push_back((file_location, module_name, raw_content));
                                }
                            }
                        }
                        Err(_) => {
                            if !files_visited.contains(&imported_package_location) {
                                let raw_content = location.read_content_as_utf8()?;
                                let module_name = name.to_string();
                                sources.push_back((
                                    imported_package_location.clone(),
                                    module_name,
                                    raw_content,
                                ));
                            }
                        }
                    }

                    println!(
                        "PACKAGE {} ({:?}) will be imported by {}",
                        package_name, location, package_name
                    );

                    manual.index_construct(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Import(block),
                        span,
                        &package_name,
                        &package_location,
                    );
                }
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
                    manual.index_construct(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Variable(block),
                        span,
                        &package_name,
                        &package_location,
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
                    manual.index_construct(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Module(block),
                        span,
                        &package_name,
                        &package_location,
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
                    manual.index_construct(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Output(block),
                        span,
                        &package_name,
                        &package_location,
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
                    manual.index_construct(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Ext(block),
                        span,
                        &package_name,
                        &package_location,
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
