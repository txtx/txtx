pub mod publishable;

use publishable::PublishableEmbeddedRunbookSpecification;
use std::collections::HashMap;
use txtx_addon_kit::hcl::structure::Block;
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::types::commands::DependencyExecutionResultCache;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::embedded_runbooks::EmbeddedRunbookStatefulExecutionContext;
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::namespace::Namespace;
use txtx_addon_kit::types::PackageId;
use txtx_addon_kit::types::{
    commands::CommandExecutionResult, embedded_runbooks::EmbeddedRunbookInstance,
};

use super::runtime_context::AddonsContext;
use super::{
    RunbookExecutionContext, RunbookExecutionMode, RunbookWorkspaceContext, RuntimeContext,
};

/// Combines the [EmbeddedRunbookInstance] with the [EmbeddingRunbookContext] to create an executable runbook instance
pub struct ExecutableEmbeddedRunbookInstance {
    pub runbook: EmbeddedRunbookInstance,
    pub context: EmbeddingRunbookContext,
}

impl ExecutableEmbeddedRunbookInstance {
    pub fn new(
        runbook_instance: EmbeddedRunbookInstance,
        signers_context: EmbeddedRunbookStatefulExecutionContext,
        top_level_inputs: &ValueStore,
        runtime_context: &RuntimeContext,
    ) -> Result<Self, Diagnostic> {
        let context = EmbeddingRunbookContext::new(
            &runbook_instance,
            &signers_context,
            top_level_inputs,
            runtime_context,
        )?;
        Ok(Self { runbook: runbook_instance, context })
    }
}

#[derive(Debug, Clone)]
/// The context of the top-level runbook (the embedding runbook) that is needed to execute an embedded runbook instance.
pub struct EmbeddingRunbookContext {
    /// The execution context contains all the data related to the execution of the runbook
    pub execution_context: RunbookExecutionContext,
    /// The workspace context keeps track of packages and constructs reachable
    pub workspace_context: RunbookWorkspaceContext,
}

impl EmbeddingRunbookContext {
    pub fn new(
        runbook_instance: &EmbeddedRunbookInstance,
        signers_context: &EmbeddedRunbookStatefulExecutionContext,
        top_level_inputs: &ValueStore,
        runtime_context: &RuntimeContext,
    ) -> Result<Self, Diagnostic> {
        let signers_downstream_dependencies = runbook_instance
            .specification
            .static_execution_context
            .signers_downstream_dependencies
            .iter()
            .map(|(signer_name, downstream)| {
                signers_context
                    .signer_did_lookup
                    .get(signer_name)
                    .ok_or(Diagnostic::error_from_string(format!(
                        "signer not found: {}",
                        signer_name
                    )))
                    .map(|signer_did| (signer_did.clone(), downstream.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut execution_context = RunbookExecutionContext {
            addon_instances: runbook_instance
                .specification
                .static_execution_context
                .addon_instances
                .clone(),
            embedded_runbooks: runbook_instance
                .specification
                .static_execution_context
                .embedded_runbooks
                .clone(),
            commands_instances: runbook_instance
                .specification
                .static_execution_context
                .commands_instances
                .clone(),
            signers_instances: signers_context.signers_instances.clone(),
            signers_state: signers_context.signers_state.clone(),
            commands_execution_results: HashMap::new(),
            commands_inputs_evaluation_results: HashMap::new(),
            commands_dependencies: runbook_instance
                .specification
                .static_execution_context
                .commands_dependencies
                .clone(),
            signers_downstream_dependencies,
            signed_commands_upstream_dependencies: runbook_instance
                .specification
                .static_execution_context
                .signed_commands_upstream_dependencies
                .clone(),
            signed_commands: runbook_instance
                .specification
                .static_execution_context
                .signed_commands
                .clone(),
            order_for_commands_execution: runbook_instance
                .specification
                .static_execution_context
                .order_for_commands_execution
                .clone(),
            order_for_signers_initialization: runbook_instance
                .specification
                .static_execution_context
                .order_for_signers_initialization
                .clone(),
            execution_mode: RunbookExecutionMode::Full,
        };

        let mut workspace_context =
            RunbookWorkspaceContext::new(runbook_instance.specification.runbook_id.clone());
        workspace_context.packages =
            runbook_instance.specification.static_workspace_context.packages.clone();

        workspace_context.constructs =
            runbook_instance.specification.static_workspace_context.constructs.clone();

        // for each package that we cloned from the embedded runbook's static context,
        // swap out the reference to the static context's signer DID with the corresponding signer for
        // this execution
        for (_, package) in workspace_context.packages.iter_mut() {
            for (signer_name, signer_did) in package.signers_did_lookup.iter_mut() {
                if let Some(new_signer_did) = signers_context.signer_did_lookup.get(signer_name) {
                    let original_did = signer_did.clone();
                    *signer_did = new_signer_did.clone();
                    let removed = package.signers_dids.remove(&original_did);
                    if removed {
                        package.signers_dids.insert(new_signer_did.clone());
                    }
                    let new_signer_id = signers_context
                        .signers_construct_id_lookup
                        .get(&new_signer_did).
                        expect("signer did found in signer did lookup, but not in signer construct id lookup");
                    let removed = workspace_context.constructs.remove(&original_did);
                    if removed.is_some() {
                        workspace_context
                            .constructs
                            .insert(new_signer_did.clone(), new_signer_id.clone());
                    };
                    execution_context.order_for_commands_execution.iter_mut().for_each(|did| {
                        if did.0.eq(&original_did.0) {
                            *did = new_signer_did.clone();
                        }
                    });
                    execution_context.order_for_signers_initialization.iter_mut().for_each(|did| {
                        if did.0.eq(&original_did.0) {
                            *did = new_signer_did.clone();
                        }
                    });
                }
            }
        }

        for (key, value) in top_level_inputs.iter() {
            let construct_did = workspace_context.index_top_level_input(key, value);
            let mut result = CommandExecutionResult::new();
            result.outputs.insert("value".into(), value.clone());
            execution_context.commands_execution_results.insert(construct_did, result);
        }

        for (_, addon_instance) in execution_context.addon_instances.iter() {
            let existing_addon_defaults = workspace_context
                .addons_defaults
                .get(&(addon_instance.package_id.did(), Namespace::from(addon_instance.addon_id.to_string())))
                .cloned();
            let defaults = runtime_context
                .generate_addon_defaults_from_block(
                    existing_addon_defaults,
                    &addon_instance.block,
                    &addon_instance.addon_id,
                    &addon_instance.package_id,
                    &DependencyExecutionResultCache::new(),
                    &mut workspace_context,
                    &execution_context,
                )
                .map_err(|e| {
                    Diagnostic::error_from_string(format!(
                    "error generating addon defaults for addon instance in embedded runbook: {}",
                    e
                ))
                })?;
            workspace_context.addons_defaults.insert(
                (addon_instance.package_id.did(), Namespace::from(&addon_instance.addon_id)),
                defaults,
            );
        }

        Ok(Self { execution_context, workspace_context })
    }

    pub fn index_top_level_inputs(&mut self, inputs_set: &ValueStore) {
        for (key, value) in inputs_set.iter() {
            let construct_did = self.workspace_context.index_top_level_input(key, value);
            let mut result = CommandExecutionResult::new();
            result.outputs.insert("value".into(), value.clone());
            self.execution_context.commands_execution_results.insert(construct_did, result);
        }
    }
}

pub struct EmbeddedRunbookInstanceBuilder {}
impl EmbeddedRunbookInstanceBuilder {
    pub async fn from_location(
        embedded_runbook_location: FileLocation,
        embedded_runbook_name: &str,
        package_id: &PackageId,
        block: &Block,
        addons_context: &mut AddonsContext,
    ) -> Result<EmbeddedRunbookInstance, Diagnostic> {
        let bytes = embedded_runbook_location.read_content().map_err(|e| {
            Diagnostic::error_from_string(format!("error reading embedded runbook content: {}", e))
        })?;

        let publishable_runbook_instance_specification = serde_json::from_slice::<
            PublishableEmbeddedRunbookSpecification,
        >(&bytes)
        .map_err(|e| {
            Diagnostic::error_from_string(format!(
                "error deserializing embedded runbook instance: {}",
                e
            ))
        })?;

        let spec = publishable_runbook_instance_specification
            .into_embedded_runbook_instance_specification(addons_context)?;

        Ok(EmbeddedRunbookInstance::new(embedded_runbook_name, block, package_id, spec))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use txtx_addon_kit::helpers::fs::FileLocation;
    use txtx_addon_kit::helpers::hcl::RawHclContent;
    use txtx_addon_kit::types::commands::CommandInstanceType;
    use txtx_addon_kit::types::embedded_runbooks::EmbeddedRunbookInputSpecification;
    use txtx_addon_kit::types::embedded_runbooks::EmbeddedRunbookValueInputSpecification;
    use txtx_addon_kit::types::package::Package;
    use txtx_addon_kit::types::stores::ValueStore;
    use txtx_addon_kit::types::types::Type;
    use txtx_addon_kit::types::ConstructDid;
    use txtx_addon_kit::types::ConstructId;
    use txtx_addon_kit::types::Did;
    use txtx_addon_kit::types::PackageId;
    use txtx_addon_kit::types::RunbookId;

    use super::publishable::*;
    use super::*;

    #[test]
    fn make_publishable() {
        let variable_hcl = r#"
        variable "my_var" {
            value = input.my_input
        }
        "#;
        let output_hcl = r#"
        output "my_output" {
            value = variable.my_var
        }
        "#;
        let my_var_name = "my_var";
        let my_output_name = "my_output";
        let my_var_id = ConstructDid(Did::from_components(vec![my_var_name.as_bytes()]));
        let my_output_id = ConstructDid(Did::from_components(vec![my_output_name.as_bytes()]));

        let package_id = PackageId {
            runbook_id: RunbookId::zero(),
            package_location: FileLocation::working_dir(),
            package_name: "my_package".to_string(),
        };

        let my_var_inst = PublishableCommandInstance {
            package_id: package_id.clone(),
            namespace: "std".to_string(),
            typing: CommandInstanceType::Variable,
            name: my_var_name.to_string(),
            hcl: RawHclContent::from_string(variable_hcl.into()),
        };
        let my_output_inst = PublishableCommandInstance {
            package_id: package_id.clone(),
            namespace: "std".to_string(),
            typing: CommandInstanceType::Output,
            name: my_output_name.to_string(),
            hcl: RawHclContent::from_string(output_hcl.into()),
        };

        let mut package = Package::new(&package_id);
        package.variables_dids.insert(my_var_id.clone());
        package.variables_did_lookup.insert(my_var_name.to_string(), my_var_id.clone());
        package.outputs_dids.insert(my_output_id.clone());
        package.outputs_did_lookup.insert(my_output_name.to_string(), my_output_id.clone());

        let my_var_construct_id = ConstructId {
            package_id: package_id.clone(),
            construct_location: package_id.package_location.clone(),
            construct_type: my_var_inst.typing.to_ident().into(),
            construct_name: my_var_name.to_string(),
        };
        let my_output_construct_id = ConstructId {
            package_id: package_id.clone(),
            construct_location: package_id.package_location.clone(),
            construct_type: my_output_inst.typing.to_ident().into(),
            construct_name: my_output_name.to_string(),
        };

        let inst = PublishableEmbeddedRunbookSpecification {
            runbook_id: RunbookId::zero(),
            description: None,
            inputs: vec![EmbeddedRunbookInputSpecification::Value(
                EmbeddedRunbookValueInputSpecification {
                    name: "my_input".to_string(),
                    documentation: "".to_string(),
                    typing: Type::String,
                },
            )],
            static_execution_context: PublishableExecutionContext {
                addon_instances: HashMap::new(),
                embedded_runbooks: HashMap::new(),
                commands_instances: HashMap::from([
                    (my_var_id.clone(), my_var_inst),
                    (my_output_id.clone(), my_output_inst),
                ]),
                signers_downstream_dependencies: vec![],
                signed_commands_upstream_dependencies: HashMap::new(),
                signed_commands: HashSet::new(),
                commands_dependencies: HashMap::from([(
                    my_output_id.clone(),
                    vec![my_var_id.clone()],
                )]),
                order_for_commands_execution: vec![my_var_id.clone(), my_output_id.clone()],
                order_for_signers_initialization: vec![],
                evaluated_inputs: ValueStore::tmp(),
            },
            static_workspace_context: PublishableWorkspaceContext {
                packages: HashMap::from([(package_id, package)]),
                constructs: HashMap::from([
                    (my_var_id.clone(), my_var_construct_id),
                    (my_output_id.clone(), my_output_construct_id),
                ]),
            },
        };
        let str = serde_json::to_string_pretty(&inst).unwrap();
        println!("{}", str);
    }
}
