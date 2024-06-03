use std::collections::{HashMap, VecDeque};
use std::future;
use std::str::FromStr;

use libsecp256k1::sign;
use txtx_addon_kit::types::commands::{CommandExecutionContext, CommandInputsEvaluationResult, CommandSpecification};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::wallets::{
    WalletActivabilityFutureResult, WalletActivateFutureResult, WalletImplementation,
    WalletInstance, WalletSignFutureResult, WalletSpecification, WalletsState,
};
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::{channel, AddonDefaults};

lazy_static! {
    pub static ref STACKS_MULTISIG: WalletSpecification = define_wallet! {
        StacksConnect => {
          name: "Stacks Multisig",
          matcher: "multisig",
          documentation: "Coming soon",
          inputs: [
            signers: {
              documentation: "Coming soon",
                typing: Type::array(Type::string()),
                optional: false,
                interpolable: true
            },
            exepcted_address: {
              documentation: "Coming soon",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            exepcted_public_key: {
                documentation: "Coming soon",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
              }
          ],
          outputs: [
              public_key: {
                documentation: "Coming soon",
                typing: Type::array(Type::buffer())
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
        // Coming soon
    "#},
      }
    };
}

pub struct StacksConnect;
impl WalletImplementation for StacksConnect {
    fn check_instantiability(
        _ctx: &WalletSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_activability(
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &WalletSpecification,
        args: &ValueStore,
        wallet_state: ValueStore,
        mut wallets: WalletsState,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> WalletActivabilityFutureResult {
        // Loop over the signers
        // Ensuring that they are all correctly activable.
        // When they are, collect the public keys
        // and build the stacks address + Check the balance
        unimplemented!()
    }

    fn activate(
        _uuid: &ConstructUuid,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        wallet_state: ValueStore,
        wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult {
        unimplemented!()
    }

    fn check_signability(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        wallet_state: ValueStore,
        wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)> {
        unimplemented!()
    }

    fn sign(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        wallet_state: ValueStore,
        wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
    ) -> WalletSignFutureResult {
        unimplemented!()
    }
}
