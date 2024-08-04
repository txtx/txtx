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
use types::commands::CommandInputsEvaluationResult;
use types::commands::CommandInstance;
use types::diagnostics::Diagnostic;
use types::ConstructDid;
use types::Did;
use types::ValueStore;
pub use uuid;
pub extern crate crossbeam_channel as channel;
pub use futures;
pub use indexmap;

pub use dotenvy_macro::dotenv;
pub use hcl_edit as hcl;
use std::{collections::HashMap, fmt::Debug};
use types::{
    commands::{CommandId, PreCommandSpecification},
    functions::FunctionSpecification,
    wallets::WalletSpecification,
};

pub use reqwest;
pub use serde;
pub use sha2;

pub mod helpers;
pub mod types;

lazy_static! {
    pub static ref DEFAULT_ADDON_DOCUMENTATION_TEMPLATE: String =
        include_str!("doc/default_addon_template.mdx").to_string();
    pub static ref DEFAULT_ADDON_FUNCTIONS_TEMPLATE: String =
        include_str!("doc/default_addon_functions_template.mdx").to_string();
    pub static ref DEFAULT_ADDON_ACTIONS_TEMPLATE: String =
        include_str!("doc/default_addon_actions_template.mdx").to_string();
    pub static ref DEFAULT_ADDON_WALLETS_TEMPLATE: String =
        include_str!("doc/default_addon_wallets_template.mdx").to_string();
    pub static ref DEFAULT_ADDON_OVERVIEW_TEMPLATE: String =
        include_str!("doc/default_addon_overview_template.mdx").to_string();
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
    fn get_functions(self: &Self) -> Vec<FunctionSpecification> {
        vec![]
    }
    ///
    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        vec![]
    }
    ///
    fn get_wallets(&self) -> Vec<WalletSpecification> {
        vec![]
    }
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
    ///
    fn get_domain_specific_commands_inputs_dependencies<'a>(
        self: &Self,
        _commands_instances: &'a Vec<(
            ConstructDid,
            &'a CommandInstance,
            Option<&'a CommandInputsEvaluationResult>,
        )>,
    ) -> Result<HashMap<ConstructDid, Vec<ConstructDid>>, Diagnostic> {
        Ok(HashMap::new())
    }
}

#[derive(Debug, Clone)]
pub struct AddonDefaults {
    pub store: ValueStore,
}

impl AddonDefaults {
    pub fn new(key: &str) -> AddonDefaults {
        AddonDefaults {
            store: ValueStore::new(key, &Did::zero()),
        }
    }
}
