use kit::indexmap::IndexMap;
use kit::types::cloud_interface::CloudServiceContext;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::VecDeque;
use txtx_addon_kit::types::commands::DependencyExecutionResultCache;
use txtx_addon_kit::types::stores::AddonDefaults;
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::{
    hcl::structure::{Block, BlockLabel},
    helpers::fs::FileLocation,
    types::{
        commands::{
            CommandId, CommandInputsEvaluationResult, CommandInstance, CommandInstanceType,
            PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        functions::FunctionSpecification,
        namespace::Namespace,
        signers::{SignerInstance, SignerSpecification},
        types::Value,
        AuthorizationContext, ConstructDid, ContractSourceTransform, Did, PackageDid, PackageId,
        RunbookId,
    },
    Addon,
};

use crate::eval::eval_expression;
use crate::{
    eval::{self, ExpressionEvaluationStatus},
    std::StdAddon,
};

use super::{
    RunbookExecutionContext, RunbookSources, RunbookTopLevelInputsMap, RunbookWorkspaceContext,
};

#[derive(Debug)]
pub struct RuntimeContext {
    /// Functions accessible at runtime
    pub functions: HashMap<String, FunctionSpecification>,
    /// Addons instantiated by runtime
    pub addons_context: AddonsContext,
    /// Number of threads allowed to work on the inputs_sets concurrently
    pub concurrency: u64,
    /// Authorizations settings to propagate to function execution
    pub authorization_context: AuthorizationContext,
    /// Cloud service configuration
    pub cloud_service_context: CloudServiceContext,
}

impl RuntimeContext {
    pub fn new(
        authorization_context: AuthorizationContext,
        get_addon_by_namespace: fn(&str) -> Option<Box<dyn Addon>>,
        cloud_service_context: CloudServiceContext,
    ) -> RuntimeContext {
        RuntimeContext {
            functions: HashMap::new(),
            addons_context: AddonsContext::new(get_addon_by_namespace),
            concurrency: 1,
            authorization_context,
            cloud_service_context,
        }
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
    ) -> Result<HashMap<ConstructDid, Vec<ConstructDid>>, (Diagnostic, ConstructDid)> {
        let mut consolidated_dependencies = HashMap::new();
        let mut grouped_commands: HashMap<
            String,
            Vec<(ConstructDid, &CommandInstance, Option<&CommandInputsEvaluationResult>)>,
        > = HashMap::new();
        for (did, command_instance) in runbook_execution_context.commands_instances.iter() {
            let inputs_simulation_results =
                runbook_execution_context.commands_inputs_evaluation_results.get(did);
            grouped_commands
                .entry(command_instance.namespace.to_string())
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
    /// that is available in the `get_addon_by_namespace` fn of the [RuntimeContext].
    /// If there is no match, returns [Vec<Diagnostic>].
    /// If there is a match, the addon is registered to the `addons_context`, storing the [PackageDid] as additional context.
    pub fn register_addon(
        &mut self,
        addon_id: &str,
        package_did: &PackageDid,
    ) -> Result<(), Vec<Diagnostic>> {
        self.addons_context.register(package_did, addon_id, true).map_err(|e| vec![e])
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

            // Register standard functions at the root level
            self.register_standard_functions();

            while let Some((location, package_name, raw_content)) = sources.pop_front() {
                let package_id = PackageId::from_file(&location, &runbook_id, &package_name)
                    .map_err(|e| vec![e])?;

                self.addons_context.register(&package_id.did(), "std", false).unwrap();

                let blocks =
                    raw_content.into_blocks().map_err(|diag| vec![diag.location(&location)])?;

                let _ = self
                    .register_addons_from_blocks(
                        blocks,
                        &package_id,
                        &location,
                        runbook_workspace_context,
                        runbook_execution_context,
                    )
                    .map_err(|diags| {
                        diagnostics.extend(diags);
                    });
            }

            if diagnostics.is_empty() {
                return Ok(());
            } else {
                return Err(diagnostics);
            }
        }
    }

    pub fn register_addons_from_blocks(
        &mut self,
        mut blocks: VecDeque<Block>,
        package_id: &PackageId,
        location: &FileLocation,
        runbook_workspace_context: &mut RunbookWorkspaceContext,
        runbook_execution_context: &RunbookExecutionContext,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        let dependencies_execution_results = DependencyExecutionResultCache::new();
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
                    self.register_addon(&addon_id, &package_id.did())?;

                    let existing_addon_defaults = runbook_workspace_context
                        .addons_defaults
                        .get(&(package_id.did(), Namespace::from(addon_id.clone())))
                        .cloned();
                    let addon_defaults = self
                        .generate_addon_defaults_from_block(
                            existing_addon_defaults,
                            &block,
                            &addon_id,
                            &package_id,
                            &dependencies_execution_results,
                            runbook_workspace_context,
                            runbook_execution_context,
                        )
                        .map_err(|diag| vec![diag.location(&location)])?;

                    runbook_workspace_context
                        .addons_defaults
                        .insert((package_id.did(), Namespace::from(addon_id.clone())), addon_defaults);
                }
                _ => {}
            }
        }
        if diagnostics.is_empty() {
            return Ok(());
        } else {
            return Err(diagnostics);
        }
    }

    pub fn generate_addon_defaults_from_block(
        &self,
        existing_addon_defaults: Option<AddonDefaults>,
        block: &Block,
        addon_id: &str,
        package_id: &PackageId,
        dependencies_execution_results: &DependencyExecutionResultCache,
        runbook_workspace_context: &mut RunbookWorkspaceContext,
        runbook_execution_context: &RunbookExecutionContext,
    ) -> Result<AddonDefaults, Diagnostic> {
        let mut addon_defaults = existing_addon_defaults.unwrap_or(AddonDefaults::new(addon_id));

        let map_entries = self.evaluate_hcl_map_blocks(
            block.body.blocks().collect(),
            dependencies_execution_results,
            package_id,
            runbook_workspace_context,
            runbook_execution_context,
        )?;

        for (key, value) in map_entries {
            // don't check for duplicate keys in map evaluation
            addon_defaults.insert(&key, Value::array(value));
        }

        for attribute in block.body.attributes() {
            let eval_result: Result<ExpressionEvaluationStatus, Diagnostic> = eval::eval_expression(
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
                Err(diag) => return Err(diag),
                w => unimplemented!("{:?}", w),
            };
            if addon_defaults.contains_key(&key) {
                return Err(diagnosed_error!(
                    "duplicate key '{}' in '{}' addon defaults",
                    key,
                    addon_id
                ));
            }
            addon_defaults.insert(&key, value);
        }
        Ok(addon_defaults)
    }

    /// Evaluates a list of map blocks, returning a map of the evaluated values.
    /// The following hcl:
    /// ```hcl
    /// my_map_key {
    ///     val1 = "test"
    ///     nested_map {
    ///         val2 = "test2"
    ///     }
    ///     nested_map {
    ///         val2 = "test3"
    ///     }
    /// }
    /// ```
    /// will return:
    /// ```rust,compile_fail
    /// use txtx_addon_kit::indexmap::IndexMap;
    /// use txtx_addon_kit::types::types::Value;
    /// IndexMap::from_iter([
    ///     (
    ///         "my_map_key".to_string(),
    ///         vec![
    ///             Value::object(IndexMap::from_iter([
    ///                 (
    ///                     "val1".to_string(),
    ///                     Value::string("test".to_string())
    ///                 ),
    ///                 (
    ///                     "nested_map".to_string(),
    ///                     Value::array(vec![
    ///                         Value::object(
    ///                             IndexMap::from_iter([
    ///                                 ("val2".to_string(), Value::string("test2".to_string()))
    ///                             ])
    ///                         ),
    ///                         Value::object(
    ///                             IndexMap::from_iter([
    ///                                 ("val2".to_string(), Value::string("test3".to_string()))
    ///                             ])
    ///                         )
    ///                     ])
    ///                 )
    ///             ]))
    ///         ]
    ///     )
    /// ]);
    /// ```
    ///
    fn evaluate_hcl_map_blocks(
        &self,
        blocks: Vec<&Block>,
        dependencies_execution_results: &DependencyExecutionResultCache,
        package_id: &PackageId,
        runbook_workspace_context: &RunbookWorkspaceContext,
        runbook_execution_context: &RunbookExecutionContext,
    ) -> Result<IndexMap<String, Vec<Value>>, Diagnostic> {
        let mut entries: IndexMap<String, Vec<Value>> = IndexMap::new();

        for block in blocks.iter() {
            // We'll store up all map entries as an Object
            let mut object_values = IndexMap::new();

            // Check if this map has nested maps within, and evaluate them
            let sub_entries = self.evaluate_hcl_map_blocks(
                block.body.blocks().collect(),
                dependencies_execution_results,
                package_id,
                runbook_workspace_context,
                runbook_execution_context,
            )?;

            for (sub_key, sub_values) in sub_entries {
                object_values.insert(sub_key, Value::array(sub_values));
            }

            for attribute in block.body.attributes() {
                let value = match eval_expression(
                    &attribute.value,
                    dependencies_execution_results,
                    package_id,
                    runbook_workspace_context,
                    runbook_execution_context,
                    &self,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => result,
                    Err(diag) => return Err(diag),
                    w => unimplemented!("{:?}", w),
                };
                match value.clone() {
                    Value::Object(obj) => {
                        for (k, v) in obj.into_iter() {
                            object_values.insert(k, v);
                        }
                    }
                    v => {
                        object_values.insert(attribute.key.to_string(), v);
                    }
                };
            }
            let block_ident = block.ident.to_string();
            match entries.get_mut(&block_ident) {
                Some(vals) => {
                    vals.push(Value::object(object_values));
                }
                None => {
                    entries.insert(block_ident, vec![Value::object(object_values)]);
                }
            }
        }

        Ok(entries)
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
    /// Function to get an available addon by namespace
    pub get_addon_by_namespace: fn(&str) -> Option<Box<dyn Addon>>,
}

impl AddonsContext {
    pub fn new(get_addon_by_namespace: fn(&str) -> Option<Box<dyn Addon>>) -> Self {
        Self {
            registered_addons: HashMap::new(),
            addon_construct_factories: HashMap::new(),
            get_addon_by_namespace,
        }
    }

    pub fn is_addon_registered(&self, addon_id: &str) -> bool {
        self.registered_addons.get(addon_id).is_some()
    }

    /// Registers an addon with this new package if the addon has already been registered
    /// by a different package.
    pub fn register_if_already_registered(
        &mut self,
        package_did: &PackageDid,
        addon_id: &str,
        scope: bool,
    ) -> Result<(), Diagnostic> {
        if self.is_addon_registered(&addon_id) {
            self.register(package_did, addon_id, scope)?;
            Ok(())
        } else {
            Err(diagnosed_error!("addon '{}' not registered", addon_id))
        }
    }

    pub fn register(
        &mut self,
        package_did: &PackageDid,
        addon_id: &str,
        scope: bool,
    ) -> Result<(), Diagnostic> {
        let key = (package_did.clone(), addon_id.to_string());
        let Some(addon) = (self.get_addon_by_namespace)(addon_id) else {
            return Err(diagnosed_error!("unable to find addon {}", addon_id));
        };
        if self.addon_construct_factories.contains_key(&key) {
            return Ok(());
        }

        // Build and register factory
        let factory = AddonConstructFactory {
            functions: addon.build_function_lookup(),
            commands: addon.build_command_lookup(),
            signers: addon.build_signer_lookup(),
        };
        self.registered_addons.insert(addon_id.to_string(), (addon, scope));
        self.addon_construct_factories.insert(key, factory);
        Ok(())
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
        location: &FileLocation,
    ) -> Result<CommandInstance, Diagnostic> {
        let factory = self
            .get_factory(namespace, &package_id.did())
            .map_err(|diag| diag.location(location))?;
        let command_id = CommandId::Action(command_id.to_string());
        factory.create_command_instance(&command_id, namespace, command_name, block, package_id)
    }

    pub fn create_signer_instance(
        &self,
        namespaced_action: &str,
        signer_name: &str,
        package_id: &PackageId,
        block: &Block,
        location: &FileLocation,
    ) -> Result<SignerInstance, Diagnostic> {
        let Some((namespace, signer_id)) = namespaced_action.split_once("::") else {
            todo!("return diagnostic")
        };
        let ctx = self
            .get_factory(namespace, &package_id.did())
            .map_err(|diag| diag.location(location))?;
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
                "action '{}': unknown command '{}::{}'",
                command_name,
                namespace,
                command_id.action_name(),
            ));
        };
        let typing = match command_id {
            CommandId::Action(command_id) => CommandInstanceType::Action(command_id.clone()),
        };
        match pre_command_spec {
            PreCommandSpecification::Atomic(command_spec) => {
                let command_instance = CommandInstance {
                    specification: command_spec.clone(),
                    name: command_name.to_string(),
                    block: block.clone(),
                    package_id: package_id.clone(),
                    typing,
                    namespace: Namespace::from(namespace),
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
            namespace: Namespace::from(namespace),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvironmentMetadata {
    location: String,
    name: String,
    description: Option<String>,
}
