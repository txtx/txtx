use kit::helpers::hcl::RawHclContent;
use kit::types::commands::{CommandInstance, CommandInstanceType};
use kit::types::diagnostics::Diagnostic;
use kit::types::embedded_runbooks::EmbeddedRunbookInstance;
use kit::types::embedded_runbooks::{
    EmbeddedRunbookInputSpecification, EmbeddedRunbookInstanceSpecification,
    EmbeddedRunbookStaticExecutionContext, EmbeddedRunbookStaticWorkspaceContext, SignerName,
};
use kit::types::stores::ValueStore;
use kit::types::AddonInstance;
use kit::types::{ConstructDid, ConstructId, PackageId, RunbookId};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::{HashMap, HashSet};

use crate::runbook::runtime_context::AddonsContext;
use crate::std::commands;
use kit::types::package::Package;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishableEmbeddedRunbookSpecification {
    pub runbook_id: RunbookId,
    pub description: Option<String>,
    pub inputs: Vec<EmbeddedRunbookInputSpecification>,
    pub static_execution_context: PublishableExecutionContext,
    pub static_workspace_context: PublishableWorkspaceContext,
}

impl PublishableEmbeddedRunbookSpecification {
    pub fn into_embedded_runbook_instance_specification(
        self,
        addons_context: &mut AddonsContext,
    ) -> Result<EmbeddedRunbookInstanceSpecification, Diagnostic> {
        Ok(EmbeddedRunbookInstanceSpecification {
            runbook_id: self.runbook_id,
            description: self.description,
            inputs: self.inputs,
            static_execution_context: self
                .static_execution_context
                .into_static_execution_context(addons_context)?,
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
    /// Map of addon instances (addon "evm" { ... })
    pub addon_instances: HashMap<ConstructDid, PublishableAddonInstance>,
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
        addons_context: &mut AddonsContext,
    ) -> Result<EmbeddedRunbookStaticExecutionContext, Diagnostic> {
        let mut addon_instances = HashMap::new();
        for (did, instance) in self.addon_instances {
            addon_instances.insert(did, instance.into_addon_instance()?);
        }
        let mut embedded_runbooks = HashMap::new();
        for (did, instance) in self.embedded_runbooks {
            embedded_runbooks.insert(did, instance.into_embedded_runbook_instance(addons_context)?);
        }
        let mut commands_instances = HashMap::new();
        for (did, instance) in self.commands_instances {
            commands_instances.insert(did, instance.into_command_instance(addons_context)?);
        }
        Ok(EmbeddedRunbookStaticExecutionContext {
            addon_instances,
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
            addon_instances: static_execution_context
                .addon_instances
                .iter()
                .map(|(c, i)| (c.clone(), PublishableAddonInstance::from_addon_instance(i)))
                .collect(),
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
pub struct PublishableAddonInstance {
    pub package_id: PackageId,
    pub addon_id: String,
    pub hcl: RawHclContent,
}
impl PublishableAddonInstance {
    pub fn into_addon_instance(self) -> Result<AddonInstance, Diagnostic> {
        let block = self.hcl.into_block_instance().map_err(|e| {
            Diagnostic::error_from_string(format!(
                "unable to parse hcl content for embedded runbook instance: {}",
                e.message
            ))
        })?;
        Ok(AddonInstance { package_id: self.package_id, addon_id: self.addon_id, block })
    }

    pub fn from_addon_instance(addon_instance: &AddonInstance) -> Self {
        Self {
            package_id: addon_instance.package_id.clone(),
            addon_id: addon_instance.addon_id.clone(),
            hcl: RawHclContent::from_block(&addon_instance.block),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// An instance of an embedded runbook, stripped of all sensitive information, and Serializable/Derserializable
/// An example embedded runbook instance:
/// ```hcl
/// runbook "my_runbook" {
///     location = "path/to/runbook.tx"
/// }
/// ```
/// The [PublishableEmbeddedRunbookInstance] would contain the spec for the embedded runbook `runbook.tx`,
/// and recursively include any runbooks, addons, and commands that are used in the embedded runbook.
pub struct PublishableEmbeddedRunbookInstance {
    pub instance_name: String,
    pub package_id: PackageId,
    pub specification: PublishableEmbeddedRunbookSpecification,
    pub hcl: RawHclContent,
}

impl PublishableEmbeddedRunbookInstance {
    pub fn into_embedded_runbook_instance(
        self,
        addons_context: &mut AddonsContext,
    ) -> Result<EmbeddedRunbookInstance, Diagnostic> {
        let block = self.hcl.into_block_instance().map_err(|e| {
            Diagnostic::error_from_string(format!(
                "unable to parse hcl content for embedded runbook instance: {}",
                e.message
            ))
        })?;
        Ok(EmbeddedRunbookInstance {
            name: self.instance_name,
            package_id: self.package_id,
            specification: self
                .specification
                .into_embedded_runbook_instance_specification(addons_context)?,
            block,
        })
    }
    pub fn from_embedded_runbook_instance(instance: &EmbeddedRunbookInstance) -> Self {
        Self {
            instance_name: instance.name.clone(),
            package_id: instance.package_id.clone(),
            specification: PublishableEmbeddedRunbookSpecification::from_embedded_runbook_instance_specification(&instance.specification),
            hcl: RawHclContent::from_block(&instance.block),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishableCommandInstance {
    pub package_id: PackageId,
    pub namespace: String,
    pub typing: CommandInstanceType,
    pub name: String,
    pub hcl: RawHclContent,
}

impl PublishableCommandInstance {
    pub fn into_command_instance(
        self,
        addons_context: &mut AddonsContext,
    ) -> Result<CommandInstance, Diagnostic> {
        let block = self.hcl.into_block_instance().map_err(|e| {
            Diagnostic::error_from_string(format!(
                "unable to parse hcl content for embedded runbook instance: {}",
                e.message
            ))
        })?;
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
            CommandInstanceType::Action(command_id) => {
                addons_context
                    .register_if_already_registered(&self.package_id.did(), &self.namespace, true)
                    .map_err(|diag| {
                        Diagnostic::error_from_string(format!(
                            "unable to register addon '{}' for embedded runbook action: {}",
                            self.namespace, diag.message
                        ))
                    })?;

                addons_context
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
                    })?
            }
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
            hcl: RawHclContent::from_block(&command_instance.block),
        }
    }
}
