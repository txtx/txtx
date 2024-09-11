use kit::types::stores::AddonDefaults;
use kit::types::stores::ValueStore;
use kit::{
    hcl::structure::{Block, BlockLabel},
    helpers::fs::FileLocation,
    indexmap::IndexMap,
    types::{
        commands::{
            CommandExecutionResult, CommandId, CommandInputsEvaluationResult, CommandInstance,
            CommandInstanceType, PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        functions::FunctionSpecification,
        signers::{SignerInstance, SignerSpecification},
        types::Value,
        AuthorizationContext, ConstructDid, ContractSourceTransform, Did, PackageDid, PackageId,
        RunbookId,
    },
    Addon,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use txtx_addon_kit::hcl;

use crate::{
    eval::{self, ExpressionEvaluationStatus},
    std::StdAddon,
};

use super::FlowContext;
use super::{
    RunbookExecutionContext, RunbookSources, RunbookTopLevelInputsMap, RunbookWorkspaceContext,
};

#[derive(Debug)]
pub struct RuntimeContext {
    /// Functions accessible at runtime
    pub functions: HashMap<String, FunctionSpecification>,
    /// Addons instantiated by runtime
    pub addons_context: AddonsContext,
    /// Addons available for runtime
    pub available_addons: Vec<Box<dyn Addon>>,
    /// Number of threads allowed to work on the inputs_sets concurrently
    pub concurrency: u64,
    /// Authorizations settings to propagate to function execution
    pub authorization_context: AuthorizationContext,
}

impl RuntimeContext {
    pub fn new(
        available_addons: Vec<Box<dyn Addon>>,
        authorization_context: AuthorizationContext,
    ) -> RuntimeContext {
        RuntimeContext {
            functions: HashMap::new(),
            addons_context: AddonsContext::new(),
            available_addons,
            concurrency: 1,
            authorization_context,
        }
    }

    pub fn collect_available_addons(&mut self) -> Vec<Box<dyn Addon>> {
        let mut addons = vec![];
        addons.append(&mut self.available_addons);
        for (_, (addon, _)) in self.addons_context.registered_addons.drain() {
            addons.push(addon);
        }
        addons
    }

    pub fn generate_initial_input_sets(
        &self,
        inputs_map: &RunbookTopLevelInputsMap,
    ) -> Vec<ValueStore> {
        let mut inputs_sets = vec![];
        let default_name = "default".to_string();
        let name = inputs_map.current_environment.as_ref().unwrap_or(&default_name);

        let mut values = ValueStore::new(name, &Did::zero());

        if let Some(current_inputs) = inputs_map.values.get(&inputs_map.current_environment) {
            values = values.with_inputs_from_vec(current_inputs);
        }
        inputs_sets.push(values);
        inputs_sets
    }

    pub fn perform_addon_processing(
        &self,
        runbook_execution_context: &mut RunbookExecutionContext,
    ) -> Result<HashMap<ConstructDid, Vec<ConstructDid>>, Diagnostic> {
        let mut consolidated_dependencies = HashMap::new();
        let mut grouped_commands: HashMap<
            String,
            Vec<(ConstructDid, &CommandInstance, Option<&CommandInputsEvaluationResult>)>,
        > = HashMap::new();
        for (did, command_instance) in runbook_execution_context.commands_instances.iter() {
            let inputs_simulation_results =
                runbook_execution_context.commands_inputs_evaluation_results.get(did);
            grouped_commands
                .entry(command_instance.namespace.clone())
                .and_modify(|e: &mut _| {
                    e.push((did.clone(), command_instance, inputs_simulation_results))
                })
                .or_insert(vec![(did.clone(), command_instance, inputs_simulation_results)]);
        }
        let mut post_processing = vec![];
        for (addon_key, commands_instances) in grouped_commands.drain() {
            let Some((addon, _)) = self.addons_context.registered_addons.get(&addon_key) else {
                continue;
            };
            let res =
                addon.get_domain_specific_commands_inputs_dependencies(&commands_instances)?;
            for (k, v) in res.dependencies.into_iter() {
                consolidated_dependencies.insert(k, v);
            }
            post_processing.push(res.transforms);
        }

        let mut remapping_required = vec![];
        for res in post_processing.iter() {
            for (construct_did, transforms) in res.iter() {
                let Some(inputs_evaluation_results) = runbook_execution_context
                    .commands_inputs_evaluation_results
                    .get_mut(construct_did)
                else {
                    continue;
                };

                for transform in transforms.iter() {
                    match transform {
                        ContractSourceTransform::FindAndReplace(from, to) => {
                            let Ok(mut contract) =
                                inputs_evaluation_results.inputs.get_expected_object("contract")
                            else {
                                continue;
                            };
                            let mut contract_source = match contract.get_mut("contract_source") {
                                Some(Value::String(source)) => source.to_string(),
                                _ => continue,
                            };
                            contract_source = contract_source.replace(from, to);
                            contract
                                .insert("contract_source".into(), Value::string(contract_source));
                            inputs_evaluation_results
                                .inputs
                                .insert("contract", Value::object(contract));
                        }
                        ContractSourceTransform::RemapDownstreamDependencies(from, to) => {
                            remapping_required.push((from, to));
                        }
                    }
                }
            }
        }

        Ok(consolidated_dependencies)
    }

    /// Checks if the provided `addon_id` matches the namespace of a supported addon
    /// that is available in the `available_addons` of the [RuntimeContext].
    /// If there is no match, returns [Vec<Diagnostic>].
    /// If there is a match, the addon is
    ///  1. Registered to the `addons_context`, storing the [PackageDid] as additional context.
    ///  2. Removed from the set of `available_addons`
    pub fn register_addon_and_remove_from_available(
        &mut self,
        addon_id: &str,
        package_did: &PackageDid,
    ) -> Result<(), Vec<Diagnostic>> {
        if self.addons_context.is_addon_registered(addon_id) {
            return Ok(());
        }
        let mut index = None;
        for (i, addon) in self.available_addons.iter().enumerate() {
            if addon.get_namespace().eq(addon_id) {
                index = Some(i);
                break;
            }
        }
        let Some(index) = index else {
            return Err(vec![diagnosed_error!("unable to find addon {}", addon_id)]);
        };

        let addon = self.available_addons.remove(index);

        self.addons_context.register(package_did, addon, true);
        Ok(())
    }

    pub fn register_standard_functions(&mut self) {
        let std_addon = StdAddon::new();
        for function in std_addon.get_functions().iter() {
            self.functions.insert(function.name.clone(), function.clone());
        }
    }

    pub fn register_addons_from_sources(
        &mut self,
        runbook_workspace_context: &mut RunbookWorkspaceContext,
        runbook_id: &RunbookId,
        runbook_sources: &RunbookSources,
        runbook_execution_context: &RunbookExecutionContext,
        _environment_selector: &Option<String>,
    ) -> Result<(), Vec<Diagnostic>> {
        {
            let mut diagnostics = vec![];

            let mut sources = runbook_sources.to_vec_dequeue();

            let dependencies_execution_results = HashMap::new();

            // Register standard functions at the root level
            self.register_standard_functions();

            while let Some((location, package_name, raw_content)) = sources.pop_front() {
                let package_id = PackageId::from_file(&location, &runbook_id, &package_name)
                    .map_err(|e| vec![e])?;

                self.addons_context.register(&package_id.did(), Box::new(StdAddon::new()), false);

                let mut blocks =
                    raw_content.into_blocks().map_err(|diag| vec![diag.location(&location)])?;

                while let Some(block) = blocks.pop_front() {
                    // parse addon blocks to load that addon
                    match block.ident.value().as_str() {
                        "addon" => {
                            let Some(BlockLabel::String(name)) = block.labels.first() else {
                                diagnostics.push(
                                    Diagnostic::error_from_string("addon name missing".into())
                                        .location(&location),
                                );
                                continue;
                            };
                            let addon_id = name.to_string();
                            self.register_addon_and_remove_from_available(
                                &addon_id,
                                &package_id.did(),
                            )?;

                            let mut addon_defaults = AddonDefaults::new(&addon_id);
                            for attribute in block.body.attributes() {
                                let eval_result = eval::eval_expression(
                                    &attribute.value,
                                    &dependencies_execution_results,
                                    &package_id,
                                    runbook_workspace_context,
                                    runbook_execution_context,
                                    self,
                                );
                                let key = attribute.key.to_string();
                                let value = match eval_result {
                                    Ok(ExpressionEvaluationStatus::CompleteOk(value)) => value,
                                    Err(diag) => return Err(vec![diag]),
                                    w => unimplemented!("{:?}", w),
                                };
                                addon_defaults.insert(&key, value);
                            }

                            runbook_workspace_context
                                .addons_defaults
                                .insert((package_id.did(), addon_id.clone()), addon_defaults);
                        }
                        _ => {}
                    }
                }
            }

            if diagnostics.is_empty() {
                return Ok(());
            } else {
                return Err(diagnostics);
            }
        }
    }

    pub fn execute_function(
        &self,
        package_did: PackageDid,
        namespace_opt: Option<String>,
        name: &str,
        args: &Vec<Value>,
        authorization_context: &AuthorizationContext,
    ) -> Result<Value, Diagnostic> {
        let function = match namespace_opt {
            Some(namespace) => match self
                .addons_context
                .addon_construct_factories
                .get(&(package_did, namespace.clone()))
            {
                Some(addon) => match addon.functions.get(name) {
                    Some(function) => function,
                    None => {
                        return Err(diagnosed_error!(
                            "could not find function {name} in namespace {}",
                            namespace
                        ))
                    }
                },
                None => return Err(diagnosed_error!("could not find namespace {}", namespace)),
            },
            None => match self.functions.get(name) {
                Some(function) => function,
                None => {
                    return Err(diagnosed_error!("could not find function {name}"));
                }
            },
        };
        (function.runner)(function, authorization_context, args)
    }
}

#[derive(Debug)]
pub struct AddonsContext {
    pub registered_addons: HashMap<String, (Box<dyn Addon>, bool)>,
    pub addon_construct_factories: HashMap<(PackageDid, String), AddonConstructFactory>,
}

impl AddonsContext {
    pub fn new() -> Self {
        Self { registered_addons: HashMap::new(), addon_construct_factories: HashMap::new() }
    }

    pub fn is_addon_registered(&self, addon_id: &str) -> bool {
        self.registered_addons.get(addon_id).is_some()
    }

    pub fn register(&mut self, package_did: &PackageDid, addon: Box<dyn Addon>, scope: bool) {
        let key = addon.get_namespace().to_string();
        if self.is_addon_registered(&key) {
            return;
        }

        // Build and register factory
        let factory = AddonConstructFactory {
            functions: addon.build_function_lookup(),
            commands: addon.build_command_lookup(),
            signers: addon.build_signer_lookup(),
        };
        self.registered_addons.insert(addon.get_namespace().to_string(), (addon, scope));
        self.addon_construct_factories.insert((package_did.clone(), key.clone()), factory);
    }

    fn get_factory(
        &self,
        namespace: &str,
        package_did: &PackageDid,
    ) -> Result<&AddonConstructFactory, Diagnostic> {
        let key = (package_did.clone(), namespace.to_string());
        let Some(factory) = self.addon_construct_factories.get(&key) else {
            return Err(diagnosed_error!(
                "unable to instantiate construct, addon '{}' unknown",
                namespace
            ));
        };
        Ok(factory)
    }

    pub fn create_action_instance(
        &self,
        namespace: &str,
        command_id: &str,
        command_name: &str,
        package_id: &PackageId,
        block: &Block,
        _location: &FileLocation,
    ) -> Result<CommandInstance, Diagnostic> {
        let factory = self.get_factory(namespace, &package_id.did())?;
        let command_id = CommandId::Action(command_id.to_string());
        factory.create_command_instance(&command_id, namespace, command_name, block, package_id)
    }

    pub fn create_signer_instance(
        &self,
        namespaced_action: &str,
        signer_name: &str,
        package_id: &PackageId,
        block: &Block,
        _location: &FileLocation,
    ) -> Result<SignerInstance, Diagnostic> {
        let Some((namespace, signer_id)) = namespaced_action.split_once("::") else {
            todo!("return diagnostic")
        };
        let ctx = self.get_factory(namespace, &package_id.did())?;
        ctx.create_signer_instance(signer_id, namespace, signer_name, block, package_id)
    }
}

#[derive(Debug, Clone)]
pub struct AddonConstructFactory {
    /// Functions supported by addon
    pub functions: HashMap<String, FunctionSpecification>,
    /// Commands supported by addon
    pub commands: HashMap<CommandId, PreCommandSpecification>,
    /// Signing commands supported by addon
    pub signers: HashMap<String, SignerSpecification>,
}

impl AddonConstructFactory {
    pub fn create_command_instance(
        self: &Self,
        command_id: &CommandId,
        namespace: &str,
        command_name: &str,
        block: &Block,
        package_id: &PackageId,
    ) -> Result<CommandInstance, Diagnostic> {
        let Some(pre_command_spec) = self.commands.get(command_id) else {
            return Err(diagnosed_error!(
                "action '{}::{}' unknown ({})",
                namespace,
                command_id.action_name(),
                command_name
            ));
        };
        let typing = match command_id {
            CommandId::Action(_) => CommandInstanceType::Action,
        };
        match pre_command_spec {
            PreCommandSpecification::Atomic(command_spec) => {
                let command_instance = CommandInstance {
                    specification: command_spec.clone(),
                    name: command_name.to_string(),
                    block: block.clone(),
                    package_id: package_id.clone(),
                    typing,
                    namespace: namespace.to_string(),
                };
                Ok(command_instance)
            }
            PreCommandSpecification::Composite(_) => unimplemented!(),
        }
    }

    pub fn create_signer_instance(
        self: &Self,
        signer_id: &str,
        namespace: &str,
        signer_name: &str,
        block: &Block,
        package_id: &PackageId,
    ) -> Result<SignerInstance, Diagnostic> {
        let Some(signer_spec) = self.signers.get(signer_id) else {
            return Err(Diagnostic::error_from_string(format!(
                "unknown signer specification: {} ({})",
                signer_id, signer_name
            )));
        };
        Ok(SignerInstance {
            name: signer_name.to_string(),
            specification: signer_spec.clone(),
            block: block.clone(),
            package_id: package_id.clone(),
            namespace: namespace.to_string(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvironmentMetadata {
    location: String,
    name: String,
    description: Option<String>,
}
