use txtx_addon_kit::types::commands::{
    CommandExecutionContext, CommandExecutionFutureResult, CommandSpecification,
};
use txtx_addon_kit::types::frontend::{ActionItemRequest, BlockEvent};
use txtx_addon_kit::types::wallets::{
    WalletActivabilityFutureResult, WalletImplementation, WalletSpecification,
};
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
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
        _args: &ValueStore,
        _state: &mut ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> WalletActivabilityFutureResult {
        // Loop over the signers
        // Ensuring that they are all correctly activable
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
