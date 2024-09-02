use txtx_addon_kit::types::commands::{CommandExecutionFutureResult, PreCommandSpecification};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::Type,
};
use txtx_addon_kit::types::{ConstructDid, ValueStore};
use txtx_addon_kit::AddonDefaults;

use crate::typing::{BitcoinValue, BITCOIN_OPCODE};

lazy_static! {
    pub static ref ENCODE: PreCommandSpecification = define_command! {
        CallReadonlyStacksFunction => {
            name: "Encode Bitcoin Opcodes to Script",
            matcher: "encode",
            documentation: "The `btc::encode` action takes a series of Bitcoin opcodes and encodes them as Bitcoin Script.",
            implements_signing_capability: false,
            implements_background_task_capability: false,
            inputs: [
                opcodes: {
                    documentation: "A series of Bitcoin opcodes.",
                    typing: Type::array(Type::addon(BITCOIN_OPCODE)),
                    optional: false,
                    interpolable: true
                }
            ],
            outputs: [
                value: {
                    documentation: "The encoded Bitcoin Script.",
                    typing: Type::string()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
                action "my_script" "btc::encode" {
                    opcodes = [
                        btc::op_dup(),
                        btc::op_hash160(),
                        btc::op_pushdata(20, "55ae51684c43435da751ac8d2173b2652eb64105"),
                        btc::op_equalverify(),
                        btc::op_checksig()
                    ]
                }
                output "encoded_script" {
                    value = action.my_script
                }
                // > encoded_script: 0x76a91455ae51684c43435da751ac8d2173b2652eb6410588ac
            "#},
        }
    };
}

pub struct CallReadonlyStacksFunction;
impl CommandImplementation for CallReadonlyStacksFunction {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none()) // todo
    }

    #[cfg(not(feature = "wasm"))]
    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let args = args.clone();

        let opcodes = args
            .get_expected_array("opcodes")?
            .into_iter()
            .map(|op| {
                op.try_get_buffer_bytes()
                    .and_then(|b| Some(b.clone()))
                    .ok_or(diagnosed_error!("bitcoin opcodes should be encoded as bytes"))
            })
            .collect::<Result<Vec<Vec<u8>>, Diagnostic>>()?;
        let script_bytes = opcodes.into_iter().flatten().collect::<Vec<u8>>();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            result.outputs.insert("value".into(), BitcoinValue::script(script_bytes));

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
