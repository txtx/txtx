#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

#[macro_use]
mod macros;

pub use hex;
// pub use hiro_system_kit;
pub use indoc::formatdoc;
pub use indoc::indoc;
pub use rust_fsm as fsm;
pub use uuid;
pub extern crate crossbeam_channel as channel;
pub use futures;

use hcl::structure::Block;
pub use hcl_edit as hcl;
use std::{collections::HashMap, fmt::Debug};
use types::{
    commands::{
        CommandId, CommandInstance, CommandInstanceOrParts, CommandInstanceType,
        PreCommandSpecification,
    },
    diagnostics::Diagnostic,
    functions::FunctionSpecification,
    wallets::{WalletInstance, WalletSpecification},
    ConstructUuid, PackageUuid,
};

pub use reqwest;
pub use serde;

pub mod helpers;
pub mod types;

lazy_static! {
    pub static ref DEFAULT_ADDON_DOCUMENTATION_TEMPLATE: String =
        include_str!("doc/default_addon_template.mdx").to_string();
}

///
pub trait Addon: Debug + Sync + Send {
    ///
    fn get_name(self: &Self) -> &str;
    ///
    fn get_description(self: &Self) -> &str;
    ///
    fn get_namespace(self: &Self) -> &str;
    ///
    fn get_functions(self: &Self) -> Vec<FunctionSpecification>;
    ///
    fn get_actions(&self) -> Vec<PreCommandSpecification>;
    ///
    fn get_wallets(&self) -> Vec<WalletSpecification>;
    ///
    fn build_function_lookup(self: &Self) -> HashMap<String, FunctionSpecification> {
        let mut functions = HashMap::new();
        for function in self.get_functions().into_iter() {
            functions.insert(function.name.clone(), function);
        }
        functions
    }
    ///
    fn build_command_lookup(self: &Self) -> HashMap<CommandId, PreCommandSpecification> {
        let mut commands = HashMap::new();

        for command in self.get_actions().into_iter() {
            let matcher = match &command {
                PreCommandSpecification::Atomic(command) => command.matcher.clone(),
                PreCommandSpecification::Composite(command) => command.matcher.clone(),
            };
            commands.insert(CommandId::Action(matcher), command);
        }
        commands
    }
    ///
    fn build_wallet_lookup(self: &Self) -> HashMap<String, WalletSpecification> {
        let mut wallet_specs = HashMap::new();

        for wallet in self.get_wallets().into_iter() {
            wallet_specs.insert(wallet.matcher.clone(), wallet);
        }

        wallet_specs
    }
    fn create_context(&self) -> AddonContext {
        AddonContext {
            functions: self.build_function_lookup(),
            commands: self.build_command_lookup(),
            wallets: self.build_wallet_lookup(),
            defaults: AddonDefaults::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AddonDefaults {
    pub keys: HashMap<String, String>,
}

impl AddonDefaults {
    pub fn new() -> AddonDefaults {
        AddonDefaults {
            keys: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AddonContext {
    pub functions: HashMap<String, FunctionSpecification>,
    pub commands: HashMap<CommandId, PreCommandSpecification>,
    pub wallets: HashMap<String, WalletSpecification>,
    pub defaults: AddonDefaults,
}

impl AddonContext {
    pub fn create_command_instance(
        self: &Self,
        command_id: &CommandId,
        namespace: &str,
        command_name: &str,
        block: &Block,
        package_uuid: &PackageUuid,
    ) -> Result<CommandInstanceOrParts, Diagnostic> {
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
                    package_uuid: package_uuid.clone(),
                    typing,
                    namespace: namespace.to_string(),
                };
                Ok(CommandInstanceOrParts::Instance(command_instance))
            }
            PreCommandSpecification::Composite(composite_spec) => {
                let bodies = (composite_spec.router)(
                    &block.body.to_string(),
                    &command_name.to_string(),
                    &composite_spec.parts,
                )?;
                Ok(CommandInstanceOrParts::Parts(bodies))
            }
        }
    }

    pub fn create_wallet_instance(
        self: &Self,
        wallet_id: &str,
        namespace: &str,
        wallet_name: &str,
        block: &Block,
        package_uuid: &PackageUuid,
    ) -> Result<WalletInstance, Diagnostic> {
        let Some(wallet_spec) = self.wallets.get(wallet_id) else {
            return Err(Diagnostic::error_from_string(format!(
                "unknown wallet specification: {} ({})",
                wallet_id, wallet_name
            )));
        };
        Ok(WalletInstance {
            name: wallet_name.to_string(),
            specification: wallet_spec.clone(),
            block: block.clone(),
            package_uuid: package_uuid.clone(),
            namespace: namespace.to_string(),
        })
    }

    pub fn resolve_construct_dependencies(
        self: &Self,
        _construct_uuid: &ConstructUuid,
    ) -> Vec<ConstructUuid> {
        vec![]
    }
}
