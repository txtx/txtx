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
        types::Value,
        wallets::{WalletInstance, WalletSpecification},
        AuthorizationContext, ConstructDid, Did, PackageDid, PackageId, RunbookId, ValueStore,
    },
    Addon, AddonDefaults,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use txtx_addon_kit::hcl;

use crate::{
    eval::{self, ExpressionEvaluationStatus},
    std::StdAddon,
};

use super::{RunbookExecutionContext, RunbookInputsMap, RunbookSources, RunbookWorkspaceContext};

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
    /// Sets of inputs indicating batches inputs
    pub inputs_sets: Vec<ValueStore>,
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
            inputs_sets: vec![],
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

    pub fn generate_initial_input_sets(&self, inputs_map: &RunbookInputsMap) -> Vec<ValueStore> {
        let mut inputs_sets = vec![];
        if inputs_sets.is_empty() {
            let default_name = "default".to_string();
            let name = inputs_map.current.as_ref().unwrap_or(&default_name);

            let mut values = ValueStore::new(name, &Did::zero());

            if let Some(current_inputs) = inputs_map.values.get(&inputs_map.current) {
                for (key, value) in current_inputs.iter() {
                    values.insert(key, value.clone());
                }
            }
            inputs_sets.push(values)
        }
        inputs_sets
    }

    pub fn collect_environment_variables(
        &self,
        runbook_id: &RunbookId,
        inputs_map: &RunbookInputsMap,
        runbook_sources: &RunbookSources,
    ) -> Result<Vec<ValueStore>, Vec<Diagnostic>> {
        let mut dummy_workspace_context = RunbookWorkspaceContext::new(runbook_id.clone());
        let mut dummy_execution_context = RunbookExecutionContext::new();

        let initial_input_sets = self.generate_initial_input_sets(inputs_map);

        for (key, value) in initial_input_sets.first().unwrap().iter() {
            let construct_did = dummy_workspace_context.index_environment_variable(key, value);

            let mut result = CommandExecutionResult::new();
            result.outputs.insert("value".into(), value.clone());
            dummy_execution_context
                .commands_execution_results
                .insert(construct_did, result);
        }

        let mut sources = VecDeque::new();
        // todo(lgalabru): basing files_visited on path is fragile, we should hash file contents instead
        let mut files_visited = HashSet::new();
        for (location, (module_name, raw_content)) in runbook_sources.tree.iter() {
            files_visited.insert(location);
            sources.push_back((location.clone(), module_name.clone(), raw_content.clone()));
        }
        let dependencies_execution_results = HashMap::new();

        let mut result = CommandExecutionResult::new();
        if let Some(values) = inputs_map.values.get(&inputs_map.current) {
            values.iter().for_each(|(k, v)| {
                result.outputs.insert(k.clone(), v.clone());
            });
        }

        let mut inputs_sets = vec![];

        while let Some((location, package_name, raw_content)) = sources.pop_front() {
            let content = hcl::parser::parse_body(&raw_content).map_err(|e| {
                vec![diagnosed_error!("parsing error: {}", e.to_string()).location(&location)]
            })?;
            let package_location = location
                .get_parent_location()
                .map_err(|e| vec![diagnosed_error!("{}", e.to_string()).location(&location)])?;
            let package_id = PackageId {
                runbook_id: runbook_id.clone(),
                package_location: package_location.clone(),
                package_name: package_name.clone(),
            };

            let mut blocks = content
                .into_blocks()
                .into_iter()
                .collect::<VecDeque<Block>>();
            while let Some(block) = blocks.pop_front() {
                match block.ident.value().as_str() {
                    "runtime" => {
                        let Some(BlockLabel::String(name)) = block.labels.first() else {
                            continue;
                        };
                        if name.to_string().eq("batch") {
                            if let Some(attr) = block.body.get_attribute("inputs") {
                                let res = eval::eval_expression(
                                    &attr.value,
                                    &dependencies_execution_results,
                                    &package_id,
                                    &dummy_workspace_context,
                                    &dummy_execution_context,
                                    self,
                                )
                                .map_err(|e| vec![e])?;
                                match res {
                                    ExpressionEvaluationStatus::CompleteOk(value) => {
                                        let batches =
                                            value.as_array().ok_or(vec![diagnosed_error!(
                                                "attribute batch.inputs should be of type array"
                                            )])?;
                                        for (index, inputs_set) in batches.iter().enumerate() {
                                            let inputs = inputs_set.as_object().ok_or(vec![
                                                diagnosed_error!(
                                                "attribute batch.inputs.* should be of type object"
                                            ),
                                            ])?;
                                            // todo: hack -> we want to be able to display the batch "name" in the cli. we don't have a way
                                            // to properly set this yet, so we're defaulting to getting this evm key
                                            let name = inputs.get("evm_defaults").map(|v| {
                                                v.as_object().map(|o| o.get("chain_alias"))
                                            });

                                            let batch_name = match name {
                                                Some(Some(Some(value))) => value.to_string(),
                                                _ => format!("Batch {}", index + 1),
                                            };
                                            let mut values =
                                                ValueStore::new(&batch_name, &&Did::zero());

                                            for (key, value) in inputs.into_iter() {
                                                values.insert(key, value.clone());
                                            }
                                            inputs_sets.push(values);
                                        }
                                    }
                                    ExpressionEvaluationStatus::DependencyNotComputed
                                    | ExpressionEvaluationStatus::CompleteErr(_) => {
                                        return Err(vec![diagnosed_error!(
                                            "unable to read attribute 'concurrency'"
                                        )])
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if inputs_sets.is_empty() {
            let default_name = "default".to_string();
            let name = inputs_map.current.as_ref().unwrap_or(&default_name);
            inputs_sets.push(ValueStore::new(name, &Did::zero()))
        }

        for inputs_set in inputs_sets.iter_mut() {
            for (key, value) in inputs_map.current_inputs_set().iter() {
                if inputs_set.get_value(key).is_none() {
                    inputs_set.insert(key, value.clone());
                }
            }
        }

        Ok(inputs_sets)
    }

    pub fn collect_domain_specific_dependencies(
        &self,
        runbook_execution_context: &RunbookExecutionContext,
    ) -> Result<HashMap<ConstructDid, Vec<ConstructDid>>, Diagnostic> {
        let mut consolidated_dependencies = HashMap::new();
        let mut grouped_commands: HashMap<
            String,
            Vec<(
                ConstructDid,
                &CommandInstance,
                Option<&CommandInputsEvaluationResult>,
            )>,
        > = HashMap::new();
        for (did, command_instance) in runbook_execution_context.commands_instances.iter() {
            let inputs_simulation_results = runbook_execution_context
                .commands_inputs_simulation_results
                .get(did);
            grouped_commands
                .entry(command_instance.namespace.clone())
                .and_modify(|e: &mut _| {
                    e.push((did.clone(), command_instance, inputs_simulation_results))
                })
                .or_insert(vec![(
                    did.clone(),
                    command_instance,
                    inputs_simulation_results,
                )]);
        }
        for (addon_key, commands_instances) in grouped_commands.iter() {
            let Some((addon, _)) = self.addons_context.registered_addons.get(addon_key) else {
                continue;
            };
            let deps =
                addon.get_domain_specific_commands_inputs_dependencies(commands_instances)?;
            for (k, v) in deps.into_iter() {
                consolidated_dependencies.insert(k, v);
            }
        }
        Ok(consolidated_dependencies)
    }

    // todo: remove, this is a hack because we haven't figured out what the `runtime` construct should look like
    pub fn load_all_addons(
        &mut self,
        runbook_id: &RunbookId,
        runbook_sources: &RunbookSources,
    ) -> Result<(), Vec<Diagnostic>> {
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
                runbook_id: runbook_id.clone(),
                package_location: package_location.clone(),
                package_name: package_name.clone(),
            };
            self.addons_context
                .register(&package_id.did(), Box::new(StdAddon::new()), false);
            // register stacks
            {
                let addon_id = "stacks";
                if self
                    .available_addons
                    .iter()
                    .any(|addon| addon.get_namespace().eq(addon_id))
                {
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
                    // self.available_addons.insert(index, Box::new(addon));

                    self.addons_context.register(&package_id.did(), addon, true);
                }
            }
            // register evm
            {
                let addon_id = "evm";
                if self
                    .available_addons
                    .iter()
                    .any(|addon| addon.get_namespace().eq(addon_id))
                {
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
                    // self.available_addons.insert(index, Box::new(addon));

                    self.addons_context.register(&package_id.did(), addon, true);
                }
            }
        }
        Ok(())
    }

    pub fn build_from_sources(
        &mut self,
        runbook_workspace_context: &mut RunbookWorkspaceContext,
        runbook_id: &RunbookId,
        inputs_sets: &Vec<ValueStore>,
        runbook_sources: &RunbookSources,
        runbook_execution_context: &RunbookExecutionContext,
        _environment_selector: &Option<String>,
    ) -> Result<(), Vec<Diagnostic>> {
        {
            let mut diagnostics = vec![];

            let mut sources = VecDeque::new();
            // todo(lgalabru): basing files_visited on path is fragile, we should hash file contents instead
            let mut files_visited = HashSet::new();
            for (location, (module_name, raw_content)) in runbook_sources.tree.iter() {
                files_visited.insert(location);
                sources.push_back((location.clone(), module_name.clone(), raw_content.clone()));
            }
            let mut addons_configs = vec![];
            let dependencies_execution_results = HashMap::new();

            // Register standard functions at the root level
            let std_addon = StdAddon::new();
            for function in std_addon.get_functions().iter() {
                self.functions
                    .insert(function.name.clone(), function.clone());
            }

            self.inputs_sets = inputs_sets.clone();

            while let Some((location, package_name, raw_content)) = sources.pop_front() {
                let content = hcl::parser::parse_body(&raw_content).map_err(|e| {
                    vec![diagnosed_error!("parsing error: {}", e.to_string()).location(&location)]
                })?;
                let package_location = location
                    .get_parent_location()
                    .map_err(|e| vec![diagnosed_error!("{}", e.to_string()).location(&location)])?;
                let package_id = PackageId {
                    runbook_id: runbook_id.clone(),
                    package_location: package_location.clone(),
                    package_name: package_name.clone(),
                };
                self.addons_context
                    .register(&package_id.did(), Box::new(StdAddon::new()), false);

                let mut blocks = content
                    .into_blocks()
                    .into_iter()
                    .collect::<VecDeque<Block>>();
                while let Some(block) = blocks.pop_front() {
                    match block.ident.value().as_str() {
                        "runtime" => {
                            let Some(BlockLabel::String(name)) = block.labels.first() else {
                                diagnostics.push(
                                    Diagnostic::error_from_string("addon name missing".into())
                                        .location(&location),
                                );
                                continue;
                            };

                            // Supported constructs:
                            // "batch"
                            // "*::addon"

                            if name.to_string().eq("batch") {
                                let concurrency =
                                    if let Some(attr) = block.body.get_attribute("concurrency") {
                                        let res = eval::eval_expression(
                                            &attr.value,
                                            &dependencies_execution_results,
                                            &package_id,
                                            runbook_workspace_context,
                                            runbook_execution_context,
                                            self,
                                        )
                                        .map_err(|e| vec![e])?;
                                        match res {
                                            ExpressionEvaluationStatus::CompleteOk(value) => value
                                                .as_uint()
                                                .transpose()
                                                .map_err(|e| {
                                                    vec![diagnosed_error!(
                                                        "invalid 'concurrency' value: {e}"
                                                    )]
                                                })?
                                                .ok_or(vec![diagnosed_error!(
                                                    "unable to read attribute 'concurrency'"
                                                )])?,
                                            ExpressionEvaluationStatus::DependencyNotComputed
                                            | ExpressionEvaluationStatus::CompleteErr(_) => {
                                                return Err(vec![diagnosed_error!(
                                                    "unable to read attribute 'concurrency'"
                                                )])
                                            }
                                        }
                                    } else {
                                        1
                                    };
                                self.concurrency = concurrency;
                            }

                            if name.to_string().starts_with("addon::") {
                                let mut defaults = IndexMap::new();
                                for defaults_block in block.body.get_blocks("defaults") {
                                    for attribute in defaults_block.body.attributes() {
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
                                            Ok(ExpressionEvaluationStatus::CompleteOk(value)) => {
                                                value
                                            }
                                            Err(diag) => return Err(vec![diag]),
                                            w => unimplemented!("{:?}", w),
                                        };
                                        defaults.insert(key, value);
                                    }
                                }
                                addons_configs.push((package_id.did(), name.to_string(), defaults));
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Loop over the sequence of addons identified
            let default_key = "chain_id".to_string();
            for (package_did, addon_name, defaults_src) in addons_configs.into_iter() {
                // let addon_id = match defaults_src.get(&default_key) {
                //     Some(entry) => entry.to_string(),
                //     None => defaults_src
                //         .first()
                //         .map(|(_, v)| v.clone())
                //         .unwrap_or(Value::null())
                //         .to_string(),
                // };
                // println!("addon id: {}", addon_id);

                match addon_name.split_once("::") {
                    Some((_, addon_id)) => {
                        let mut defaults = AddonDefaults::new(&addon_id);
                        for (k, v) in defaults_src.into_iter() {
                            defaults.store.insert(&k, v);
                        }
                        let mut index = None;
                        for (i, addon) in self.available_addons.iter().enumerate() {
                            if addon.get_namespace().eq(addon_id) {
                                index = Some(i);
                                break;
                            }
                        }
                        let Some(index) = index else {
                            // todo: remove continue and reinstate error once we figure out how we want to handle runtimes and remove the load_all_addons fn
                            runbook_workspace_context
                                .addons_defaults
                                .insert((package_did.clone(), addon_id.into()), defaults);
                            continue;
                            // return Err(vec![diagnosed_error!(
                            //     "unable to find addon {}",
                            //     addon_id
                            // )]);
                        };

                        let addon = self.available_addons.remove(index);
                        runbook_workspace_context.addons_defaults.insert(
                            (package_did.clone(), addon.get_namespace().into()),
                            defaults,
                        );
                        self.addons_context.register(&package_did, addon, true);
                    }
                    _ => {
                        diagnostics.push(diagnosed_error!("addon '{}' unknown", addon_name));
                    }
                };
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
        Self {
            registered_addons: HashMap::new(),
            addon_construct_factories: HashMap::new(),
        }
    }

    pub fn register(&mut self, package_did: &PackageDid, addon: Box<dyn Addon>, scope: bool) {
        let key = addon.get_namespace().to_string();
        if self.registered_addons.get(&key).is_some() {
            return;
        }

        // Build and register factory
        let factory = AddonConstructFactory {
            functions: addon.build_function_lookup(),
            commands: addon.build_command_lookup(),
            signing_commands: addon.build_wallet_lookup(),
        };
        self.registered_addons
            .insert(addon.get_namespace().to_string(), (addon, scope));
        self.addon_construct_factories
            .insert((package_did.clone(), key.clone()), factory);
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

    pub fn create_signing_command_instance(
        &self,
        namespaced_action: &str,
        wallet_name: &str,
        package_id: &PackageId,
        block: &Block,
        _location: &FileLocation,
    ) -> Result<WalletInstance, Diagnostic> {
        let Some((namespace, wallet_id)) = namespaced_action.split_once("::") else {
            todo!("return diagnostic")
        };
        let ctx = self.get_factory(namespace, &package_id.did())?;
        ctx.create_signing_command_instance(wallet_id, namespace, wallet_name, block, package_id)
    }
}

#[derive(Debug, Clone)]
pub struct AddonConstructFactory {
    /// Functions supported by addon
    pub functions: HashMap<String, FunctionSpecification>,
    /// Commands supported by addon
    pub commands: HashMap<CommandId, PreCommandSpecification>,
    /// Signing commands supported by addon
    pub signing_commands: HashMap<String, WalletSpecification>,
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

    pub fn create_signing_command_instance(
        self: &Self,
        wallet_id: &str,
        namespace: &str,
        wallet_name: &str,
        block: &Block,
        package_id: &PackageId,
    ) -> Result<WalletInstance, Diagnostic> {
        let Some(wallet_spec) = self.signing_commands.get(wallet_id) else {
            return Err(Diagnostic::error_from_string(format!(
                "unknown wallet specification: {} ({})",
                wallet_id, wallet_name
            )));
        };
        Ok(WalletInstance {
            name: wallet_name.to_string(),
            specification: wallet_spec.clone(),
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
