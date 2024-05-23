use std::sync::{Arc, Mutex};

use hcl_edit::structure::Block;
use rust_fsm::StateMachine;

use super::{
    commands::{
        CommandChecker, CommandInput, CommandInstanceStateMachine, CommandOutput, CommandRunner,
    },
    PackageUuid,
};

#[derive(Debug, Clone)]
pub struct WalletSpecification {
    pub name: String,
    pub matcher: String,
    pub documentation: String,
    pub accepts_arbitrary_inputs: bool,
    pub create_output_for_each_input: bool,
    pub update_addon_defaults: bool,
    pub example: String,
    pub default_inputs: Vec<CommandInput>,
    pub inputs: Vec<CommandInput>,
    pub outputs: Vec<CommandOutput>,
    pub signer: CommandRunner,
    pub pub_key_fetcher: CommandRunner,
    pub checker: CommandChecker,
    pub user_input_parser: CommandRunner,
}

#[derive(Debug, Clone)]
pub struct WalletInstance {
    pub specification: WalletSpecification,
    pub state: Arc<Mutex<StateMachine<CommandInstanceStateMachine>>>,
    pub name: String,
    pub block: Block,
    pub package_uuid: PackageUuid,
    pub namespace: String,
}
