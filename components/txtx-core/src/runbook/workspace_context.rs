use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use crate::std::commands;
use crate::types::{Package, PreConstructData};
use kit::hcl::expr::{Expression, TraversalOperator};
use kit::hcl::structure::{Block, BlockLabel};
use kit::helpers::fs::{get_txtx_files_paths, FileLocation};
use kit::helpers::hcl::visit_required_string_literal_attribute;
use kit::types::commands::{CommandId, CommandInstance, CommandInstanceType};
use kit::types::diagnostics::{Diagnostic, DiagnosticLevel};
use kit::types::types::Value;
use kit::types::wallets::WalletInstance;
use kit::types::{ConstructDid, ConstructId, Did, PackageDid, PackageId, RunbookId};
use kit::AddonDefaults;
use txtx_addon_kit::hcl;

use super::{RunbookExecutionContext, RunbookGraphContext, RunbookSources, RuntimeContext};

pub enum ConstructInstanceType {
    Executable(CommandInstance),
    Signing(WalletInstance),
    Import,
}

#[derive(better_debug::BetterDebug, Clone)]
pub struct RunbookWorkspaceContext {
    /// Id of the Runbook
    pub runbook_id: RunbookId,
    /// Map of packages. A package is either a standalone .tx file, or a directory enclosing multiple .tx files
    pub packages: HashMap<PackageId, Package>,
    /// Map of constructs. A construct refers to root level objects (input, action, output, wallet, import, ...)
    pub constructs: HashMap<ConstructDid, ConstructId>,
    /// Lookup: Retrieve a construct did, given an environment name (mainnet, testnet, etc)
    pub environment_variables_did_lookup: BTreeMap<String, ConstructDid>,
    /// Lookup: Retrieve a construct did, given an environment name (mainnet, testnet, etc)
    pub environment_variables_values: BTreeMap<ConstructDid, Value>,

    pub addons_defaults: HashMap<(PackageDid, String), AddonDefaults>,

    std_defaults: AddonDefaults,
}

impl RunbookWorkspaceContext {
    pub fn new(runbook_id: RunbookId) -> Self {
        Self {
            runbook_id,
            packages: HashMap::new(),
            constructs: HashMap::new(),
            environment_variables_did_lookup: BTreeMap::new(),
            environment_variables_values: BTreeMap::new(),
            addons_defaults: HashMap::new(),
            std_defaults: AddonDefaults::new("std"),
        }
    }

    pub fn get_addon_defaults(&self, key: &(PackageDid, String)) -> &AddonDefaults {
        self.addons_defaults.get(key).unwrap_or(&self.std_defaults)
    }

    pub fn build_from_sources(
        &mut self,
        runbook_sources: &RunbookSources,
        runtime_context: &RuntimeContext,
        graph_context: &mut RunbookGraphContext,
        execution_context: &mut RunbookExecutionContext,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        let mut sources = VecDeque::new();
        // todo(lgalabru): basing files_visited on path is fragile, we should hash file contents instead
        let mut files_visited = HashSet::new();
        for (location, (module_name, raw_content)) in runbook_sources.tree.iter() {
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
            let package_id = PackageId {
                runbook_id: self.runbook_id.clone(),
                package_location: package_location.clone(),
                package_name: package_name.clone(),
            };
            self.index_package(&package_id);
            graph_context.index_package(&package_id);

            let mut blocks = content
                .into_blocks()
                .into_iter()
                .collect::<VecDeque<Block>>();
            while let Some(block) = blocks.pop_front() {
                match block.ident.value().as_str() {
                    "import" => {
                        // imports are the only constructs that we need to process in this step
                        let Some(BlockLabel::String(name)) = block.labels.first() else {
                            diagnostics.push(Diagnostic {
                                location: Some(location.clone()),
                                span: None,
                                message: "import name missing".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            });
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
                                        sources.push_back((
                                            file_location,
                                            module_name,
                                            raw_content,
                                        ));
                                    }
                                }
                            }
                            Err(_) => {
                                if !files_visited.contains(&imported_package_location) {
                                    let raw_content =
                                        location.read_content_as_utf8().map_err(|e| {
                                            vec![diagnosed_error!("{}", e.to_string())
                                                .location(&location)]
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

                        let _ = self.index_construct(
                            name.to_string(),
                            location.clone(),
                            PreConstructData::Import(block.clone()),
                            &package_id,
                            graph_context,
                            execution_context,
                        );
                    }
                    "input" => {
                        let Some(BlockLabel::String(name)) = block.labels.first() else {
                            diagnostics.push(Diagnostic {
                                location: Some(location.clone()),
                                span: None,
                                message: "variable name missing".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            });
                            continue;
                        };
                        let _ = self.index_construct(
                            name.to_string(),
                            location.clone(),
                            PreConstructData::Input(block.clone()),
                            &package_id,
                            graph_context,
                            execution_context,
                        );
                    }
                    "module" => {
                        let Some(BlockLabel::String(name)) = block.labels.first() else {
                            diagnostics.push(Diagnostic {
                                location: Some(location.clone()),
                                span: None,
                                message: "module name missing".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            });
                            continue;
                        };
                        let _ = self.index_construct(
                            name.to_string(),
                            location.clone(),
                            PreConstructData::Module(block.clone()),
                            &package_id,
                            graph_context,
                            execution_context,
                        );
                    }
                    "output" => {
                        let Some(BlockLabel::String(name)) = block.labels.first() else {
                            diagnostics.push(Diagnostic {
                                location: Some(location.clone()),
                                span: None,
                                message: "output name missing".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            });
                            continue;
                        };
                        let _ = self.index_construct(
                            name.to_string(),
                            location.clone(),
                            PreConstructData::Output(block.clone()),
                            &package_id,
                            graph_context,
                            execution_context,
                        );
                    }
                    "action" => {
                        let (Some(command_name), Some(namespaced_action)) =
                            (block.labels.get(0), block.labels.get(1))
                        else {
                            diagnostics.push(Diagnostic {
                                location: Some(location.clone()),
                                span: None,
                                message: "action syntax invalid".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            });
                            continue;
                        };

                        let Some((namespace, command_id)) = namespaced_action.split_once("::")
                        else {
                            todo!("return diagnostic")
                        };

                        match runtime_context.addons_context.create_action_instance(
                            namespace,
                            command_id,
                            command_name.as_str(),
                            &package_id,
                            &block,
                            &location,
                        ) {
                            Ok(command_instance) => {
                                let _ = self.index_construct(
                                    command_name.to_string(),
                                    location.clone(),
                                    PreConstructData::Action(command_instance),
                                    &package_id,
                                    graph_context,
                                    execution_context,
                                );
                            }
                            Err(diagnostic) => {
                                diagnostics.push(diagnostic);
                                continue;
                            }
                        };
                    }
                    "wallet" => {
                        let (Some(wallet_name), Some(namespaced_wallet_cmd)) =
                            (block.labels.get(0), block.labels.get(1))
                        else {
                            diagnostics.push(Diagnostic {
                                location: Some(location.clone()),
                                span: None,
                                message: "action syntax invalid".to_string(),
                                level: DiagnosticLevel::Error,
                                documentation: None,
                                example: None,
                                parent_diagnostic: None,
                            });
                            continue;
                        };
                        match runtime_context
                            .addons_context
                            .create_signing_command_instance(
                                &namespaced_wallet_cmd.as_str(),
                                wallet_name.as_str(),
                                &package_id,
                                &block,
                                &location,
                            ) {
                            Ok(wallet_instance) => {
                                let _ = self.index_construct(
                                    wallet_name.to_string(),
                                    location.clone(),
                                    PreConstructData::Wallet(wallet_instance),
                                    &package_id,
                                    graph_context,
                                    execution_context,
                                );
                            }
                            Err(diagnostic) => {
                                diagnostics.push(diagnostic);
                                continue;
                            }
                        }
                    }
                    "runtime" => {}
                    _ => {
                        diagnostics.push(Diagnostic {
                            location: Some(location.clone()),
                            span: None,
                            message: "construct unknown".to_string(),
                            level: DiagnosticLevel::Error,
                            documentation: None,
                            example: None,
                            parent_diagnostic: None,
                        });
                    }
                }
            }
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }

    pub fn index_environment_variable(&mut self, key: &String, value: &Value) -> ConstructDid {
        let construct_did = ConstructDid(Did::from_components(vec![
            "runbook_input".as_bytes(),
            key.as_bytes(),
        ]));
        self.environment_variables_values
            .insert(construct_did.clone(), value.clone());
        self.environment_variables_did_lookup
            .insert(key.clone(), construct_did.clone());
        construct_did
    }

    fn index_package(&mut self, package_id: &PackageId) {
        loop {
            if let Some(_) = self.packages.get(&package_id) {
                break;
            }
            let package = Package::new(package_id);
            self.packages.insert(package_id.clone(), package);
            continue;
        }
    }

    fn index_construct(
        &mut self,
        construct_name: String,
        construct_location: FileLocation,
        construct_data: PreConstructData,
        package_id: &PackageId,
        graph_context: &mut RunbookGraphContext,
        execution_context: &mut RunbookExecutionContext,
    ) -> ConstructId {
        let package = self
            .packages
            .get_mut(&package_id)
            .expect("internal error: unable to retrieve package");
        let construct_id = ConstructId {
            package_id: package_id.clone(),
            construct_type: construct_data.construct_type().into(),
            construct_location,
            construct_name: construct_name.clone(),
        };
        let construct_did = construct_id.did();
        self.constructs
            .insert(construct_did.clone(), construct_id.clone());
        let construct_instance_type = match construct_data {
            PreConstructData::Module(block) => {
                // if construct_name.eq("runbook") && self.runbook_metadata_construct_did.is_none() {
                //     self.runbook_metadata_construct_did = Some(construct_did.clone());
                // }
                package.modules_dids.insert(construct_did.clone());
                package
                    .modules_did_lookup
                    .insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Executable(CommandInstance {
                    specification: commands::new_module_specification(),
                    name: construct_name.clone(),
                    block: block.clone(),
                    package_id: package_id.clone(),
                    namespace: construct_name.clone(),
                    typing: CommandInstanceType::Module,
                })
            }
            PreConstructData::Input(block) => {
                package.variables_dids.insert(construct_did.clone());
                package
                    .inputs_did_lookup
                    .insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Executable(CommandInstance {
                    specification: commands::new_input_specification(),
                    name: construct_name.clone(),
                    block: block.clone(),
                    package_id: package_id.clone(),
                    namespace: construct_name.clone(),
                    typing: CommandInstanceType::Input,
                })
            }
            PreConstructData::Addon(block) => {
                package.commands_dids.insert(construct_did.clone());
                package
                    .addons_did_lookup
                    .insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Executable(CommandInstance {
                    specification: commands::new_runtime_setting(),
                    name: construct_name.clone(),
                    block: block.clone(),
                    package_id: package_id.clone(),
                    namespace: construct_name.clone(),
                    typing: CommandInstanceType::Addon,
                })
            }
            PreConstructData::Output(block) => {
                package.outputs_dids.insert(construct_did.clone());
                package
                    .outputs_did_lookup
                    .insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Executable(CommandInstance {
                    specification: commands::new_output_specification(),
                    name: construct_name.clone(),
                    block: block.clone(),
                    package_id: package_id.clone(),
                    namespace: construct_name.clone(),
                    typing: CommandInstanceType::Output,
                })
            }
            PreConstructData::Import(_) => {
                package.imports_dids.insert(construct_did.clone());
                package
                    .imports_did_lookup
                    .insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Import
            }
            PreConstructData::Action(command_instance) => {
                package.commands_dids.insert(construct_did.clone());
                package.addons_did_lookup.insert(
                    CommandId::Action(construct_name).to_string(),
                    construct_did.clone(),
                );
                ConstructInstanceType::Executable(command_instance)
            }
            PreConstructData::Wallet(wallet_instance) => {
                package.signing_commands_dids.insert(construct_did.clone());
                package
                    .signing_commands_did_lookup
                    .insert(construct_name, construct_did.clone());
                ConstructInstanceType::Signing(wallet_instance)
            }
            PreConstructData::Root => unreachable!(),
        };

        let construct_did = construct_id.did();
        graph_context.index_construct(&construct_did);
        match construct_instance_type {
            ConstructInstanceType::Executable(instance) => {
                execution_context
                    .commands_instances
                    .insert(construct_did.clone(), instance);
            }
            ConstructInstanceType::Signing(instance) => {
                execution_context
                    .signing_commands_instances
                    .insert(construct_did.clone(), instance);
            }
            ConstructInstanceType::Import => {}
        }

        construct_id
    }

    /// Expects `expression` to be a traversal and `package_did_source` to be indexed in the runbook's `packages`.
    /// Iterates over the operators of `expression` to see if any of the blocks it references are cached as a
    /// `module`, `output`, `input`, `action`, or `prompt` in the package.
    ///
    pub fn try_resolve_construct_reference_in_expression(
        &self,
        source_package_id: &PackageId,
        expression: &Expression,
    ) -> Result<Option<(ConstructDid, VecDeque<String>, VecDeque<Value>)>, String> {
        let Some(traversal) = expression.as_traversal() else {
            return Ok(None);
        };

        let Some(root) = traversal.expr.as_variable() else {
            return Ok(None);
        };

        let mut subpath = VecDeque::new();

        let mut components = VecDeque::new();
        components.push_front(root.to_string());

        for op in traversal.operators.iter() {
            if let TraversalOperator::GetAttr(value) = op.value() {
                components.push_back(value.to_string());
            }
            if let TraversalOperator::Index(expr) = op.value() {
                match expr {
                    Expression::Number(value) => {
                        subpath.push_back(Value::int(value.as_i64().unwrap()));
                    }
                    Expression::String(value) => {
                        subpath.push_back(Value::string(value.to_string()));
                    }
                    Expression::Bool(value) => {
                        subpath.push_back(Value::bool(**value));
                    }
                    _ => unimplemented!(),
                }
            }
        }

        let mut is_root = true;

        while let Some(component) = components.pop_front() {
            // Look for modules
            if is_root {
                // Look for env variables
                if component.eq_ignore_ascii_case("env") {
                    let Some(env_variable_name) = components.pop_front() else {
                        continue;
                    };

                    if let Some(construct_did) = self
                        .environment_variables_did_lookup
                        .get(&env_variable_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                let Some(current_package) = self.packages.get(source_package_id) else {
                    return Ok(None);
                };

                if component.eq_ignore_ascii_case("module") {
                    is_root = false;
                    let Some(module_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) =
                        current_package.modules_did_lookup.get(&module_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for outputs
                if component.eq_ignore_ascii_case("output") {
                    is_root = false;
                    let Some(output_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) =
                        current_package.outputs_did_lookup.get(&output_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for inputs
                if component.eq_ignore_ascii_case("input") {
                    is_root = false;
                    let Some(input_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) = current_package.inputs_did_lookup.get(&input_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for actions
                if component.eq_ignore_ascii_case("action") {
                    is_root = false;
                    let Some(action_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) = current_package
                        .addons_did_lookup
                        .get(&CommandId::Action(action_name).to_string())
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for wallets
                if component.eq_ignore_ascii_case("wallet") {
                    is_root = false;
                    let Some(wallet_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) = current_package
                        .signing_commands_did_lookup
                        .get(&wallet_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }
            }
            let Some(mut current_package) = self.packages.get(source_package_id) else {
                return Ok(None);
            };

            let imported_package = current_package
                .imports_did_lookup
                .get(&component.to_string())
                .and_then(|c| self.constructs.get(c))
                .and_then(|c| Some(&c.package_id))
                .and_then(|p| self.packages.get(&p));

            if let Some(imported_package) = imported_package {
                current_package = imported_package;
                continue;
            }
        }
        Ok(None)
    }
}
