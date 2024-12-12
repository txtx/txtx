use kit::hcl::structure::Block;
use kit::helpers::hcl::RawHclContent;
use kit::types::commands::{CommandInstance, CommandInstanceType};
use kit::types::diagnostics::Diagnostic;
use kit::types::embedded_runbooks::{
    EmbeddedRunbookInputSpecification, EmbeddedRunbookInstanceSpecification,
    EmbeddedRunbookLocation, EmbeddedRunbookStatefulExecutionContext,
    EmbeddedRunbookStaticExecutionContext, EmbeddedRunbookStaticWorkspaceContext, SignerName,
};
use kit::types::stores::ValueStore;
use kit::types::{commands::CommandExecutionResult, embedded_runbooks::EmbeddedRunbookInstance};
use kit::types::{ConstructDid, ConstructId, PackageId, RunbookId};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::std::commands;
use kit::types::package::Package;

use super::runtime_context::AddonsContext;
use super::{RunbookExecutionContext, RunbookExecutionMode, RunbookWorkspaceContext};

fn search_in_blocks(blocks: &VecDeque<Block>, ident: &str, name: &str) -> Option<Block> {
    for block in blocks {
        if block.ident.to_string().eq(ident) {
            if let Some(block_name) = block.labels.get(0) {
                if block_name.to_string().eq(name) {
                    return Some(block.clone());
                }
            }
        }
    }
    None
}

pub struct ExecutableEmbeddedRunbookInstance {
    pub runbook: EmbeddedRunbookInstance,
    pub context: EmbeddableRunbookContext,
}

impl ExecutableEmbeddedRunbookInstance {
    pub fn new(
        runbook_instance: EmbeddedRunbookInstance,
        signers_context: EmbeddedRunbookStatefulExecutionContext,
        top_level_inputs: &ValueStore,
    ) -> Result<Self, Diagnostic> {
        let context =
            EmbeddableRunbookContext::new(&runbook_instance, &signers_context, top_level_inputs)?;
        Ok(Self { runbook: runbook_instance, context })
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddableRunbookContext {
    /// The execution context contains all the data related to the execution of the runbook
    pub execution_context: RunbookExecutionContext,
    /// The workspace context keeps track of packages and constructs reachable
    pub workspace_context: RunbookWorkspaceContext,
    /// The set of environment variables used during the execution
    pub inputs: ValueStore,
    /// The evaluated inputs to this flow
    pub evaluated_inputs: ValueStore,
}

impl EmbeddableRunbookContext {
    pub fn new(
        runbook_instance: &EmbeddedRunbookInstance,
        signers_context: &EmbeddedRunbookStatefulExecutionContext,
        top_level_inputs: &ValueStore,
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

        for (key, value) in top_level_inputs.iter() {
            let construct_did = workspace_context.index_top_level_input(key, value);
            let mut result = CommandExecutionResult::new();
            result.outputs.insert("value".into(), value.clone());
            execution_context.commands_execution_results.insert(construct_did, result);
        }

        Ok(Self {
            execution_context,
            workspace_context,
            inputs: top_level_inputs.clone(),
            evaluated_inputs: ValueStore::tmp(),
        })
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
        embedded_runbook_location: EmbeddedRunbookLocation,
        embedded_runbook_name: &str,
        package_id: &PackageId,
        block: &Block,
        addons_context: &AddonsContext,
    ) -> Result<EmbeddedRunbookInstance, Diagnostic> {
        let bytes = embedded_runbook_location.get_content().await.map_err(|e| {
            Diagnostic::error_from_string(format!("error reading embedded runbook content: {}", e))
        })?;

        let publishable_runbook_instance = serde_json::from_slice::<
            TopLevelPublishableEmbeddedRunbookInstance,
        >(&bytes)
        .map_err(|e| {
            Diagnostic::error_from_string(format!(
                "error deserializing embedded runbook instance: {}",
                e
            ))
        })?;

        publishable_runbook_instance.into_embedded_runbook_instance(
            embedded_runbook_name,
            package_id,
            block,
            addons_context,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopLevelPublishableEmbeddedRunbookInstance {
    pub specification: PublishableEmbeddedRunbookInstanceSpecification,
    pub hcl: RawHclContent,
}
impl TopLevelPublishableEmbeddedRunbookInstance {
    pub fn into_embedded_runbook_instance(
        self,
        instance_name: &str,
        package_id: &PackageId,
        block: &Block,
        addons_context: &AddonsContext,
    ) -> Result<EmbeddedRunbookInstance, Diagnostic> {
        let embedded_runbook_blocks = self.hcl.into_blocks()?;
        Ok(EmbeddedRunbookInstance {
            name: instance_name.into(),
            package_id: package_id.clone(),
            specification: self.specification.into_embedded_runbook_instance_specification(
                addons_context,
                &embedded_runbook_blocks,
            )?,
            block: block.clone(),
            hcl: self.hcl,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishableEmbeddedRunbookInstance {
    pub hcl: RawHclContent,
    pub instance_name: String,
    pub package_id: PackageId,
    pub specification: PublishableEmbeddedRunbookInstanceSpecification,
}

impl PublishableEmbeddedRunbookInstance {
    pub fn into_embedded_runbook_instance(
        self,
        addons_context: &AddonsContext,
        blocks: &VecDeque<Block>,
    ) -> Result<EmbeddedRunbookInstance, Diagnostic> {
        let block = search_in_blocks(blocks, "runbook", &self.instance_name).ok_or(
            Diagnostic::error_from_string(format!(
                "block not found for embedded runbook instance: {}",
                self.instance_name
            )),
        )?;
        let embedded_runbook_blocks = self.hcl.into_blocks()?;
        Ok(EmbeddedRunbookInstance {
            name: self.instance_name,
            package_id: self.package_id,
            specification: self.specification.into_embedded_runbook_instance_specification(
                addons_context,
                &embedded_runbook_blocks,
            )?,
            block: block.clone(),
            hcl: self.hcl,
        })
    }
    pub fn from_embedded_runbook_instance(instance: &EmbeddedRunbookInstance) -> Self {
        Self {
            hcl: instance.hcl.clone(),
            instance_name: instance.name.clone(),
            package_id: instance.package_id.clone(),
            specification: PublishableEmbeddedRunbookInstanceSpecification::from_embedded_runbook_instance_specification(&instance.specification),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishableEmbeddedRunbookInstanceSpecification {
    pub runbook_id: RunbookId,
    pub description: Option<String>,
    pub inputs: Vec<EmbeddedRunbookInputSpecification>,
    pub static_execution_context: PublishableExecutionContext,
    pub static_workspace_context: PublishableWorkspaceContext,
}

impl PublishableEmbeddedRunbookInstanceSpecification {
    pub fn into_embedded_runbook_instance_specification(
        self,
        addons_context: &AddonsContext,
        blocks: &VecDeque<Block>,
    ) -> Result<EmbeddedRunbookInstanceSpecification, Diagnostic> {
        Ok(EmbeddedRunbookInstanceSpecification {
            runbook_id: self.runbook_id,
            description: self.description,
            inputs: self.inputs,
            static_execution_context: self
                .static_execution_context
                .into_static_execution_context(addons_context, blocks)?,
            static_workspace_context: self.static_workspace_context.into_workspace_context(),
        })
    }
    pub fn from_embedded_runbook_instance_specification(
        specification: &EmbeddedRunbookInstanceSpecification,
    ) -> Self {
        Self {
            runbook_id: specification.runbook_id.clone(),
            description: specification.description.clone(),
            inputs: specification.inputs.clone(),
            static_execution_context: PublishableExecutionContext::from_static_execution_context(
                &specification.static_execution_context,
            ),
            static_workspace_context: PublishableWorkspaceContext::from_static_workspace_context(
                &specification.static_workspace_context,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishableExecutionContext {
    /// Map of embedded runbooks
    pub embedded_runbooks: HashMap<ConstructDid, PublishableEmbeddedRunbookInstance>,
    /// Map of executable commands (input, output, action)
    pub commands_instances: HashMap<ConstructDid, PublishableCommandInstance>,
    /// Commands dependencies
    pub commands_dependencies: HashMap<ConstructDid, Vec<ConstructDid>>,
    /// Constructs depending on a given Construct performing signing.
    pub signers_downstream_dependencies: Vec<(SignerName, Vec<ConstructDid>)>,
    /// Constructs depending on a given Construct being signed.
    pub signed_commands_upstream_dependencies: HashMap<ConstructDid, Vec<ConstructDid>>,
    /// Constructs depending on a given Construct being signed.
    pub signed_commands: HashSet<ConstructDid>,
    /// Commands execution order
    pub order_for_commands_execution: Vec<ConstructDid>,
    /// Signing commands initialization order
    pub order_for_signers_initialization: Vec<ConstructDid>,
    /// Published evaluated inputs
    pub evaluated_inputs: ValueStore,
}

impl PublishableExecutionContext {
    pub fn into_static_execution_context(
        self,
        addons_context: &AddonsContext,
        blocks: &VecDeque<Block>,
    ) -> Result<EmbeddedRunbookStaticExecutionContext, Diagnostic> {
        let mut embedded_runbooks = HashMap::new();
        for (did, instance) in self.embedded_runbooks {
            embedded_runbooks
                .insert(did, instance.into_embedded_runbook_instance(addons_context, blocks)?);
        }
        let mut commands_instances = HashMap::new();
        for (did, instance) in self.commands_instances {
            commands_instances
                .insert(did, instance.into_command_instance(&addons_context, &blocks).unwrap());
        }
        Ok(EmbeddedRunbookStaticExecutionContext {
            embedded_runbooks,
            commands_instances,
            commands_dependencies: self.commands_dependencies,
            signers_downstream_dependencies: self.signers_downstream_dependencies,
            signed_commands_upstream_dependencies: self.signed_commands_upstream_dependencies,
            signed_commands: self.signed_commands,
            order_for_commands_execution: self.order_for_commands_execution,
            order_for_signers_initialization: self.order_for_signers_initialization,
            evaluated_inputs: self.evaluated_inputs,
        })
    }

    pub fn from_static_execution_context(
        static_execution_context: &EmbeddedRunbookStaticExecutionContext,
    ) -> Self {
        Self {
            embedded_runbooks: static_execution_context
                .embedded_runbooks
                .iter()
                .map(|(c, i)| {
                    (
                        c.clone(),
                        PublishableEmbeddedRunbookInstance::from_embedded_runbook_instance(i),
                    )
                })
                .collect(),
            commands_instances: static_execution_context
                .commands_instances
                .iter()
                .map(|(c, i)| (c.clone(), PublishableCommandInstance::from_command_instance(i)))
                .collect(),
            commands_dependencies: static_execution_context.commands_dependencies.clone(),
            signers_downstream_dependencies: static_execution_context
                .signers_downstream_dependencies
                .clone(),
            signed_commands_upstream_dependencies: static_execution_context
                .signed_commands_upstream_dependencies
                .clone(),
            signed_commands: static_execution_context.signed_commands.clone(),
            order_for_commands_execution: static_execution_context
                .order_for_commands_execution
                .clone(),
            order_for_signers_initialization: static_execution_context
                .order_for_signers_initialization
                .clone(),
            evaluated_inputs: static_execution_context.evaluated_inputs.clone(),
        }
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishableWorkspaceContext {
    /// Map of packages. A package is either a standalone .tx file, or a directory enclosing multiple .tx files
    #[serde_as(as = "Vec<(_, _)>")]
    pub packages: HashMap<PackageId, Package>,
    /// Map of constructs. A construct refers to root level objects (input, action, output, signer, import, ...)
    pub constructs: HashMap<ConstructDid, ConstructId>,
}
impl PublishableWorkspaceContext {
    pub fn into_workspace_context(self) -> EmbeddedRunbookStaticWorkspaceContext {
        EmbeddedRunbookStaticWorkspaceContext {
            packages: self.packages,
            constructs: self.constructs,
        }
    }
    pub fn from_static_workspace_context(
        static_workspace_context: &EmbeddedRunbookStaticWorkspaceContext,
    ) -> Self {
        Self {
            packages: static_workspace_context.packages.clone(),
            constructs: static_workspace_context.constructs.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishableCommandInstance {
    pub package_id: PackageId,
    pub namespace: String,
    pub typing: CommandInstanceType,
    pub name: String,
}

impl PublishableCommandInstance {
    pub fn into_command_instance(
        self,
        addons_context: &AddonsContext,
        blocks: &VecDeque<Block>,
    ) -> Result<CommandInstance, Diagnostic> {
        let block = search_in_blocks(&blocks, self.typing.to_ident(), &self.name).ok_or(
            Diagnostic::error_from_string(format!(
                "block not found for command instance: {} {}",
                self.typing.to_ident(),
                self.name
            )),
        )?;
        let command_instance = match self.typing {
            CommandInstanceType::Variable => CommandInstance {
                specification: commands::new_variable_specification(),
                name: self.name.clone(),
                block: block.clone(),
                package_id: self.package_id.clone(),
                namespace: self.namespace.clone(),
                typing: CommandInstanceType::Variable,
            },
            CommandInstanceType::Output => CommandInstance {
                specification: commands::new_output_specification(),
                name: self.name.clone(),
                block: block.clone(),
                package_id: self.package_id.clone(),
                namespace: self.namespace.clone(),
                typing: CommandInstanceType::Output,
            },
            CommandInstanceType::Action(command_id) => addons_context
                .create_action_instance(
                    &self.namespace,
                    &command_id,
                    &self.name,
                    &self.package_id,
                    &block,
                    &self.package_id.package_location,
                )
                .map_err(|diag| {
                    Diagnostic::error_from_string(format!(
                        "invalid embedded runbook action: {}",
                        diag.message
                    ))
                })?,
            CommandInstanceType::Prompt => todo!(),
            CommandInstanceType::Module => CommandInstance {
                specification: commands::new_module_specification(),
                name: self.name.clone(),
                block: block.clone(),
                package_id: self.package_id.clone(),
                namespace: self.namespace.clone(),
                typing: CommandInstanceType::Module,
            },
            CommandInstanceType::Addon => todo!(),
        };
        Ok(command_instance)
    }

    pub fn from_command_instance(command_instance: &CommandInstance) -> Self {
        Self {
            package_id: command_instance.package_id.clone(),
            namespace: command_instance.namespace.clone(),
            typing: command_instance.typing.clone(),
            name: command_instance.name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use kit::helpers::fs::FileLocation;
    use kit::types::embedded_runbooks::EmbeddedRunbookValueInputSpecification;
    use kit::types::types::Type;
    use kit::types::Did;

    use super::*;
    use crate::kit::types::stores::ValueStore;
    use crate::kit::types::PackageId;
    use crate::kit::types::RunbookId;

    #[test]
    fn make_publishable() {
        let hcl_str = r#"
        variable "my_var" {
            value = input.my_input
        }

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
        };
        let my_output_inst = PublishableCommandInstance {
            package_id: package_id.clone(),
            namespace: "std".to_string(),
            typing: CommandInstanceType::Output,
            name: my_output_name.to_string(),
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

        let inst = TopLevelPublishableEmbeddedRunbookInstance {
            specification: PublishableEmbeddedRunbookInstanceSpecification {
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
            },
            hcl: RawHclContent::from_string(hcl_str.to_string()),
        };
        let str = serde_json::to_string_pretty(&inst).unwrap();
        println!("{}", str);
    }
}
