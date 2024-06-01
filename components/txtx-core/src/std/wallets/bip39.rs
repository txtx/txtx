use kit::types::frontend::BlockEvent;
use txtx_addon_kit::types::commands::{CommandExecutionContext, CommandExecutionFutureResult};
use txtx_addon_kit::types::frontend::ActionItemRequest;
use txtx_addon_kit::types::wallets::{
    WalletActivabilityFutureResult, WalletImplementation, WalletSpecification,
};
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::{channel, AddonDefaults};

lazy_static! {
    pub static ref STD_BIP39: WalletSpecification = define_wallet! {
        StacksConnect => {
          name: "BIP39 Wallet",
          matcher: "bip39",
          documentation: "Coming soon",
          inputs: [
            mnemonic: {
              documentation: "Coming soon",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            derivation_path: {
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
        _args: &ValueStore,
        _state: &mut ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> WalletActivabilityFutureResult {
        unimplemented!()
    }

    fn activate(
        _uuid: &ConstructUuid,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _state: &mut ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        unimplemented!()
    }

    fn check_signability(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _state: &mut ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Vec<ActionItemRequest>, Diagnostic> {
        unimplemented!()
    }

    fn sign(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _state: &ValueStore,
        _defaults: &AddonDefaults,
    ) -> CommandExecutionFutureResult {
        unimplemented!()
    }

    fn check_public_key_expectations(
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _public_key_bytes: &Vec<u8>,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Option<String>, Diagnostic> {
        unimplemented!()
    }
}
