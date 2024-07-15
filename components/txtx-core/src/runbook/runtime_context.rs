use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use kit::{
    hcl::structure::{Block, BlockLabel},
    helpers::fs::FileLocation,
    types::{
        commands::{CommandId, CommandInstance, CommandInstanceType, PreCommandSpecification},
        diagnostics::{Diagnostic, DiagnosticLevel},
        functions::FunctionSpecification,
        types::Value,
        wallets::{WalletInstance, WalletSpecification},
        PackageDid, PackageId, RunbookId,
    },
    Addon, AddonDefaults,
};
use serde::{Deserialize, Serialize};
use txtx_addon_kit::hcl;
use txtx_addon_network_stacks::StacksNetworkAddon;

use crate::{
    eval::{self, ExpressionEvaluationStatus},
    std::StdAddon,
};

use super::{RunbookExecutionContext, RunbookSources, RunbookWorkspaceContext};

#[derive(Debug)]
pub struct RuntimeContext {
    /// Functions accessible at runtime
    pub functions: HashMap<String, FunctionSpecification>,
    /// Addons accessible at runtime
    pub addons_context: AddonsContext,
    ///
    pub selected_env: Option<String>,
    ///
    pub environments: BTreeMap<String, BTreeMap<String, String>>,
}

impl RuntimeContext {
    pub fn new() -> RuntimeContext {
        RuntimeContext {
            functions: HashMap::new(),
            addons_context: AddonsContext::new(),
            selected_env: None,
            environments: BTreeMap::new(),
        }
    }

    pub fn build_from_sources(
        &mut self,
        runbook_sources: &RunbookSources,
        runbook_id: &RunbookId,
        runbook_workspace_context: &RunbookWorkspaceContext,
        runbook_execution_context: &RunbookExecutionContext,
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
            let mut addons_names = vec![];
            let dependencies_execution_results = HashMap::new();
            let mut addons_context = AddonsContext::new();

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
                addons_context.register(
                    &package_id.did(),
                    Box::new(StdAddon::new()),
                    false,
                    AddonDefaults::new(),
                );

                let mut blocks = content
                    .into_blocks()
                    .into_iter()
                    .collect::<VecDeque<Block>>();
                while let Some(block) = blocks.pop_front() {
                    match block.ident.value().as_str() {
                        "addon" => {
                            let Some(BlockLabel::String(name)) = block.labels.first() else {
                                diagnostics.push(Diagnostic {
                                    location: Some(location.clone()),
                                    span: None,
                                    message: "addon name missing".to_string(),
                                    level: DiagnosticLevel::Error,
                                    documentation: None,
                                    example: None,
                                    parent_diagnostic: None,
                                });
                                continue;
                            };

                            let mut blocks_iter = block.body.get_blocks("defaults");
                            let mut defaults = AddonDefaults::new();
                            while let Some(block) = blocks_iter.next() {
                                for attribute in block.body.attributes() {
                                    let eval_result = eval::eval_expression(
                                        &attribute.value,
                                        &dependencies_execution_results,
                                        &package_id,
                                        runbook_workspace_context,
                                        runbook_execution_context,
                                        self,
                                    );
                                    let value = match eval_result {
                                        Ok(ExpressionEvaluationStatus::CompleteOk(value)) => value,
                                        Err(diag) => return Err(vec![diag]),
                                        _ => unimplemented!(),
                                    };
                                    defaults.store.insert(&attribute.key.to_string(), value);
                                }
                            }
                            addons_names.push((package_id.did(), name.to_string(), defaults));
                        }
                        _ => {}
                    }
                }
            }

            // Loop over the sequence of addons identified
            for (package_did, addon_name, defaults) in addons_names.into_iter() {
                match addon_name.as_str() {
                    "stacks" => {
                        addons_context.register(
                            &package_did,
                            Box::new(StacksNetworkAddon::new()),
                            true,
                            defaults,
                        );
                    }
                    _ => {
                        diagnostics.push(diagnosed_error!("addon '{}' unknown", addon_name));
                    }
                };
            }
            self.addons_context = addons_context;

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
        (function.runner)(function, args)
    }

    pub fn set_active_environment(&mut self, selected_env: String) -> Result<(), String> {
        match self.environments.get(&selected_env) {
            Some(_) => self.selected_env = Some(selected_env),
            None => {
                return Err(format!("environment with key {} unknown", selected_env));
            }
        }
        Ok(())
    }

    pub fn get_active_environment_variables(&self) -> BTreeMap<String, String> {
        let Some(ref active_env) = self.selected_env else {
            return BTreeMap::new();
        };
        match self.environments.get(active_env) {
            Some(variables) => variables.clone(),
            None => BTreeMap::new(),
        }
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

    pub fn register(
        &mut self,
        package_did: &PackageDid,
        addon: Box<dyn Addon>,
        scope: bool,
        defaults: AddonDefaults,
    ) {
        let key = addon.get_namespace().to_string();
        if self.registered_addons.get(&key).is_some() {
            return;
        }

        // Build and register factory
        let factory = AddonConstructFactory {
            functions: addon.build_function_lookup(),
            commands: addon.build_command_lookup(),
            signing_commands: addon.build_wallet_lookup(),
            defaults,
        };
        self.registered_addons
            .insert(addon.get_namespace().to_string(), (addon, scope));
        self.addon_construct_factories
            .insert((package_did.clone(), key.clone()), factory);
    }

    pub fn consolidate_functions_to_register(&mut self) -> Vec<FunctionSpecification> {
        let mut functions = vec![];
        for (_, (addon, _)) in self.registered_addons.iter() {
            let mut addon_functions = addon.get_functions();
            functions.append(&mut addon_functions);
        }
        functions
    }

    fn get_factory(
        &mut self,
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
        &mut self,
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
        &mut self,
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
    /// Defaults registered
    pub defaults: AddonDefaults,
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
            todo!("return diagnostic: unknown command: {:?}", command_id)
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
