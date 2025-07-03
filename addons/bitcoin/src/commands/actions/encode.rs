use txtx_addon_kit::types::commands::{CommandExecutionFutureResult, PreCommandSpecification};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::Type,
};

use crate::typing::{BitcoinValue, BITCOIN_OPCODE};

lazy_static! {
    pub static ref ENCODE_SCRIPT: PreCommandSpecification = define_command! {
        CallReadonlyStacksFunction => {
            name: "Encode Bitcoin Script",
            matcher: "encode_script",
            documentation: "The `btc::encode_script` action takes a series of Bitcoin instructions and encodes them as Bitcoin Script.",
            implements_signing_capability: false,
            implements_background_task_capability: false,
            inputs: [
                instructions: {
                    documentation: "A series of Bitcoin instructions.",
                    typing: Type::array(Type::addon(BITCOIN_OPCODE)),
                    optional: false,
                    tainting: true,
                    internal: false
                }
            ],
            outputs: [
                value: {
                    documentation: "The encoded Bitcoin Script.",
                    typing: Type::string()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
                action "my_script" "btc::encode_script" {
                    instructions = [
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
        _values: &ValueStore,
        _supervision_context: &RunbookSupervisionContext,
        _auth_context: &txtx_addon_kit::types::AuthorizationContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none()) // todo
    }

    #[cfg(not(feature = "wasm"))]
    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        use crate::constants::INSTRUCTIONS;

        let args = args.clone();

        let instructions = args
            .get_expected_array(INSTRUCTIONS)?
            .into_iter()
            .map(|op| {
                let t = op.try_get_buffer_bytes_result().map_err(|e| {
                    diagnosed_error!("bitcoin instructions should be encoded as bytes: {e}")
                })?;

                t.ok_or(diagnosed_error!("bitcoin instructions should be encoded as bytes"))
            })
            .collect::<Result<Vec<Vec<u8>>, Diagnostic>>()?;
        let script_bytes = instructions.into_iter().flatten().collect::<Vec<u8>>();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            result.outputs.insert("value".into(), BitcoinValue::script(script_bytes));

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
