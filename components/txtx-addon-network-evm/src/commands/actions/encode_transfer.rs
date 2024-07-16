use crate::typing::{ETH_ADDRESS, ETH_TRANSACTION};
use alloy::consensus::SignableTransaction;
use alloy::network::TransactionBuilder;
use alloy::primitives::bytes::BufMut;
use alloy::primitives::U256;
use alloy::rpc::types::TransactionRequest;
use txtx_addon_kit::types::commands::{
    return_synchronous_ok, CommandExecutionContext, CommandExecutionFutureResult,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::AddonDefaults;

use super::get_expected_address;

lazy_static! {
    pub static ref ENCODE_EVM_TRANSFER: PreCommandSpecification = define_command! {
        EncodeEVMTransfer => {
          name: "EVM Transfer",
          matcher: "encode_transfer",
          documentation: "Coming soon",
          implements_signing_capability: false,
          implements_background_task_capability: false,
          inputs: [
              to: {
                  documentation: "The address of the recipient of the transfer.",
                  typing: Type::addon(ETH_ADDRESS.clone()),
                  optional: false,
                  interpolable: true
              },
              amount: {
                  documentation: "The amount, in WEI, to transfer.",
                  typing: Type::uint(),
                  optional: false,
                  interpolable: true
              },
              chain_id: {
                  documentation: "The chain id.",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
              },
              network_id: {
                  documentation: "The network id.",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
              },
              depends_on: {
                documentation: "References another command's outputs, preventing this command from executing until the referenced command is successful.",
                typing: Type::string(),
                optional: true,
                interpolable: true
              }
          ],
          outputs: [
              bytes: {
                  documentation: "The encoded transfer transaction bytes.",
                  typing: Type::buffer()
              },
              network_id: {
                  documentation: "The network id of the encoded transaction.",
                  typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
            // Coming soon
        "#},
      }
    };
}

pub struct EncodeEVMTransfer;

impl CommandImplementation for EncodeEVMTransfer {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none()) // todo
    }

    fn run_execution(
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let mut result = CommandExecutionResult::new();

        // Extract network_id
        let network_id = args.get_defaulting_string("network_id", defaults)?;
        let chain_id = args.get_defaulting_string("chain_id", defaults)?;
        let to = args.get_expected_value("to")?;
        let amount = args.get_expected_uint("amount")?;

        let to = get_expected_address(to)
            .map_err(|e| diagnosed_error!("command 'evm::encode_transfer': {}", e))?;
        let chain_id: u64 = chain_id.parse::<u64>().map_err(|e| {
            diagnosed_error!(
                "command 'evm::encode_transfer': failed to parse chain_id: {}",
                e
            )
        })?;

        let tx = TransactionRequest::default()
            .with_to(to)
            .with_value(U256::from(amount))
            .with_nonce(0)
            .with_chain_id(chain_id)
            .with_gas_limit(21_000)
            .with_max_priority_fee_per_gas(1_000_000_000)
            .with_max_fee_per_gas(20_000_000_000);

        let tx = tx.build_unsigned().unwrap();
        let tx = tx.eip1559().unwrap();
        let mut bytes = vec![].writer();
        let bytes = bytes.get_mut();
        tx.encode_for_signing(bytes);

        result.outputs.insert(
            "bytes".to_string(),
            Value::buffer(bytes.to_vec(), ETH_TRANSACTION.clone()),
        );
        result
            .outputs
            .insert("network_id".to_string(), Value::string(network_id));
        return_synchronous_ok(result)
    }
}
