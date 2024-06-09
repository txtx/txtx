use std::collections::{HashSet, VecDeque};

use crate::{
    errors::{ConstructErrors, DiscoveryError},
    types::{PreConstructData, Runbook, RuntimeContext},
};
use txtx_addon_kit::{
    hcl::structure::Block,
    types::{
        commands::CommandInstanceOrParts,
        diagnostics::{Diagnostic, DiagnosticLevel},
    },
};
use txtx_addon_kit::{
    hcl::{self, structure::BlockLabel},
    helpers::{
        fs::{get_txtx_files_paths, FileLocation},
        hcl::visit_required_string_literal_attribute,
    },
};

// todo(lgalabru): clean-up this function
pub fn run_constructs_indexing(
    runbook: &mut Runbook,
    runtime_context: &mut RuntimeContext,
) -> Result<bool, Vec<Diagnostic>> {
    let mut has_errored = false;

    let Some(source_tree) = runbook.source_tree.take() else {
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
        let content = hcl::parser::parse_body(&raw_content).map_err(|e| {
            vec![diagnosed_error!("parsing error: {}", e.to_string()).location(&location)]
        })?;
        let package_location = location
            .get_parent_location()
            .map_err(|e| vec![diagnosed_error!("{}", e.to_string()).location(&location)])?;
        let package_uuid = runbook
            .find_or_create_package_uuid(&package_name, &package_location)
            .map_err(|e| vec![diagnosed_error!("{}", e.to_string()).location(&location)])?;

        let mut blocks = content
            .into_blocks()
            .into_iter()
            .collect::<VecDeque<Block>>();
        while let Some(block) = blocks.pop_front() {
            match block.ident.value().as_str() {
                "import" => {
                    // imports are the only constructs that we need to process in this step
                    let Some(BlockLabel::String(name)) = block.labels.first() else {
                        runbook.errors.push(Diagnostic {
                                location: Some(location.clone()),
                                span: None,
                                message: "import name missing".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            });
                        has_errored = true;
                        continue;
                    };

                    let path = visit_required_string_literal_attribute("path", &block).unwrap(); // todo(lgalabru)
                    println!("Loading {} at path ({path})", name.to_string());

                    // todo(lgalabru): revisit this approach, filesystem access needs to be abstracted.
                    let mut imported_package_location =
                        location.get_parent_location().map_err(|e| {
                            vec![diagnosed_error!("{}", e.to_string()).location(&location)]
                        })?;

                    imported_package_location.append_path(&path).unwrap();

                    match std::fs::read_dir(imported_package_location.to_string()) {
                        Ok(_) => {
                            let files =
                                get_txtx_files_paths(&imported_package_location.to_string())
                                    .map_err(|e| {
                                        vec![diagnosed_error!("{}", e.to_string())
                                            .location(&imported_package_location)]
                                    })?;
                            for file_path in files.into_iter() {
                                let file_location = FileLocation::from_path(file_path);
                                if !files_visited.contains(&file_location) {
                                    let raw_content =
                                        file_location.read_content_as_utf8().map_err(|e| {
                                            vec![diagnosed_error!("{}", e.to_string())
                                                .location(&file_location)]
                                        })?;
                                    let module_name = name.to_string();
                                    sources.push_back((file_location, module_name, raw_content));
                                }
                            }
                        }
                        Err(_) => {
                            if !files_visited.contains(&imported_package_location) {
                                let raw_content = location.read_content_as_utf8().map_err(|e| {
                                    vec![diagnosed_error!("{}", e.to_string()).location(&location)]
                                })?;
                                let module_name = name.to_string();
                                sources.push_back((
                                    imported_package_location.clone(),
                                    module_name,
                                    raw_content,
                                ));
                            }
                        }
                    }

                    let _ = runbook.index_construct(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Import(block.clone()),
                        &package_uuid,
                    );
                }
                "input" => {
                    let Some(BlockLabel::String(name)) = block.labels.first() else {
                        runbook.errors.push(Diagnostic {
                                location: Some(location.clone()),
                                span: None,
                                message: "variable name missing".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            });
                        has_errored = true;
                        continue;
                    };
                    let _ = runbook.index_construct(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Input(block.clone()),
                        &package_uuid,
                    );
                }
                "module" => {
                    let Some(BlockLabel::String(name)) = block.labels.first() else {
                        runbook.errors.push(Diagnostic {
                                location: Some(location.clone()),
                                span: None,
                                message: "module name missing".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            });
                        has_errored = true;
                        continue;
                    };
                    let _ = runbook.index_construct(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Module(block.clone()),
                        &package_uuid,
                    );
                }
                "output" => {
                    let Some(BlockLabel::String(name)) = block.labels.first() else {
                        runbook.errors.push(Diagnostic {
                                location: Some(location.clone()),
                                span: None,
                                message: "output name missing".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            });
                        has_errored = true;
                        continue;
                    };
                    let _ = runbook.index_construct(
                        name.to_string(),
                        location.clone(),
                        PreConstructData::Output(block.clone()),
                        &package_uuid,
                    );
                }
                "action" => {
                    let (Some(command_name), Some(namespaced_action)) =
                        (block.labels.get(0), block.labels.get(1))
                    else {
                        runbook.errors.push(Diagnostic {
                                location: Some(location.clone()),
                                span: None,
                                message: "action syntax invalid".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            });
                        has_errored = true;
                        continue;
                    };

                    let Some((namespace, command_id)) = namespaced_action.split_once("::") else {
                        todo!("return diagnostic")
                    };

                    match runtime_context.addons_ctx.create_action_instance(
                        namespace,
                        command_id,
                        command_name.as_str(),
                        &package_uuid,
                        &block,
                        &location,
                    ) {
                        Ok(command_instance_or_parts) => match command_instance_or_parts {
                            CommandInstanceOrParts::Instance(command_instance) => {
                                let _ = runbook.index_construct(
                                    command_name.to_string(),
                                    location.clone(),
                                    PreConstructData::Action(command_instance),
                                    &package_uuid,
                                );
                            }
                            CommandInstanceOrParts::Parts(parts_blocks) => {
                                for block in parts_blocks {
                                    let parsed_block = hcl::parser::parse_body(&block).unwrap();
                                    for block in parsed_block.blocks() {
                                        blocks.push_back(block.clone());
                                    }
                                }
                            }
                        },
                        Err(diagnostic) => {
                            runbook.errors.push(diagnostic);
                            continue;
                        }
                    };
                }
                "wallet" => {
                    let (Some(wallet_name), Some(namespaced_wallet_cmd)) =
                        (block.labels.get(0), block.labels.get(1))
                    else {
                        runbook.errors.push(Diagnostic {
                                location: Some(location.clone()),
                                span: None,
                                message: "action syntax invalid".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            });
                        has_errored = true;
                        continue;
                    };
                    match runtime_context.addons_ctx.create_wallet_instance(
                        &namespaced_wallet_cmd.as_str(),
                        wallet_name.as_str(),
                        &package_uuid,
                        &block,
                        &location,
                    ) {
                        Ok(wallet_instance) => {
                            let _ = runbook.index_construct(
                                wallet_name.to_string(),
                                location.clone(),
                                PreConstructData::Wallet(wallet_instance),
                                &package_uuid,
                            );
                        }
                        Err(diagnostic) => {
                            runbook.errors.push(diagnostic);
                            has_errored = true;
                            continue;
                        }
                    }
                }
                _ => {
                    runbook.errors.push(Diagnostic {
                            location: Some(location.clone()),
                            span: None,
                            message: "construct unknown".to_string(),
                            level: DiagnosticLevel::Error,
                            documentation: None,
                            example: None,
                            parent_diagnostic: None,
                        });
                    has_errored = true;
                }
            }
        }
    }

    Ok(has_errored)
}
