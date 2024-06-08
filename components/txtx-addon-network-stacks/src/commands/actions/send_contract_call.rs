use std::collections::HashMap;
use txtx_addon_kit::channel;
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::{
    types::{
        commands::{
            CommandExecutionContext, CommandExecutionFutureResult, CommandImplementation,
            CommandSpecification, PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        frontend::{Actions, BlockEvent},
        types::Type,
        wallets::{WalletInstance, WalletSignFutureResult, WalletsState},
        ConstructUuid, ValueStore,
    },
    AddonDefaults,
};

use crate::{
    constants::{SIGNED_TRANSACTION_BYTES, TRANSACTION_PAYLOAD_BYTES},
    typing::{CLARITY_PRINCIPAL, CLARITY_VALUE},
};

use super::{
    broadcast_transaction::BroadcastStacksTransaction, encode_contract_call,
    sign_transaction::SignStacksTransaction,
};

lazy_static! {
    pub static ref SEND_CONTRACT_CALL: PreCommandSpecification = define_command! {
        SendContractCall => {
          name: "Send Contract Call Transaction",
          matcher: "send_contract_call",
          documentation: "The `send_contract_call` action encodes a contract call transaction, signs the transaction using an in-browser wallet, and broadcasts the signed transaction to the network.",
          implements_signing_capability: true,
          implements_background_task_capability: true,
          inputs: [
              contract_id: {
                  documentation: "The address and identifier of the contract to invoke.",
                  typing: Type::addon(CLARITY_PRINCIPAL.clone()),
                  optional: false,
                  interpolable: true
              },
              function_name: {
                  documentation: "The contract method to invoke.",
                  typing: Type::string(),
                  optional: false,
                  interpolable: true
              },
              function_args: {
                  documentation: "The function arguments for the contract call.",
                  typing: Type::array(Type::addon(CLARITY_VALUE.clone())),
                  optional: true,
                  interpolable: true
              },
              network_id: {
                  documentation: "The network id used to validate the transaction version.",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
              },
              signer: {
                  documentation: "Coming soon",
                  typing: Type::string(),
                  optional: false,
                  interpolable: true
              },
              confirmations: {
                documentation: "Coming soon - once the transaction is included on a block, the number of blocks to await before the transaction is considered successful.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
              },
              depends_on: {
                documentation: "Coming soon",
                typing: Type::string(),
                optional: true,
                interpolable: true
              }
          ],
          outputs: [
            signed_transaction_bytes: {
                documentation: "The signed transaction bytes.",
                typing: Type::string()
            },
            tx_id: {
              documentation: "The transaction id.",
              typing: Type::string()
            },
            result: {
              documentation: "The transaction result.",
              typing: Type::buffer()
            }
          ],
        example: txtx_addon_kit::indoc! {r#"
            action "my_ref" "stacks::send_contract_call" {
                description = "Encodes the contract call, sign, and broadcasts the set-token function."
                contract_id = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.pyth-oracle-v1"
                function_name = "verify-and-update-price-feeds"
                function_args = [
                    encode_buffer(output.bitcoin_price_feed),
                    encode_tuple({
                        "pyth-storage-contract": encode_principal("${env.pyth_deployer}.pyth-store-v1"),
                        "pyth-decoder-contract": encode_principal("${env.pyth_deployer}.pyth-pnau-decoder-v1"),
                        "wormhole-core-contract": encode_principal("${env.pyth_deployer}.wormhole-core-v1")
                    })
                ]
                signer = wallet.alice
            }            
            output "tx_id" {
            value = action.my_ref.tx_id
            }
            output "result" {
            value = action.my_ref.result
            }
            // > tx_id: 0x...
            // > result: success
  "#},
      }
    };
}

pub struct SendContractCall;
impl CommandImplementation for SendContractCall {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_signed_executability(
        uuid: &ConstructUuid,
        instance_name: &str,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        execution_context: &CommandExecutionContext,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        wallets: WalletsState,
    ) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)> {
        // Extract network_id
        let network_id: String = args.get_defaulting_string("network_id", defaults).unwrap();
        let contract_id_value = args.get_expected_value("contract_id").unwrap();
        let function_name = args.get_expected_string("function_name").unwrap();
        let function_args_values = args.get_expected_array("function_args").unwrap();
        let bytes = encode_contract_call(
            spec,
            function_name,
            function_args_values,
            &network_id,
            contract_id_value,
        )
        .unwrap();
        let mut args = args.clone();
        args.insert(TRANSACTION_PAYLOAD_BYTES, bytes);

        SignStacksTransaction::check_signed_executability(
            uuid,
            instance_name,
            spec,
            &args,
            defaults,
            execution_context,
            wallets_instances,
            wallets,
        )
    }

    fn run_signed_execution(
        uuid: &ConstructUuid,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        wallets: WalletsState,
    ) -> WalletSignFutureResult {
        let network_id: String = args.get_defaulting_string("network_id", defaults).unwrap();
        let contract_id_value = args.get_expected_value("contract_id").unwrap();
        let function_name = args.get_expected_string("function_name").unwrap();
        let function_args_values = args.get_expected_array("function_args").unwrap();
        let bytes = encode_contract_call(
            spec,
            function_name,
            function_args_values,
            &network_id,
            contract_id_value,
        )
        .unwrap();
        let progress_tx = progress_tx.clone();
        let args = args.clone();
        let wallets_instances = wallets_instances.clone();
        let defaults = defaults.clone();
        let uuid = uuid.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();

        let mut args = args.clone();
        args.insert(TRANSACTION_PAYLOAD_BYTES, bytes);

        let future = async move {
            let run_signing_future = SignStacksTransaction::run_signed_execution(
                &uuid,
                &spec,
                &args,
                &defaults,
                &progress_tx,
                &wallets_instances,
                wallets,
            );
            let (wallets, mut res_signing) = match run_signing_future {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(err) => return Err(err),
                },
                Err(err) => return Err(err),
            };

            args.insert(
                SIGNED_TRANSACTION_BYTES,
                res_signing
                    .outputs
                    .get(SIGNED_TRANSACTION_BYTES)
                    .unwrap()
                    .clone(),
            );
            let mut res = match BroadcastStacksTransaction::run_execution(
                &uuid,
                &spec,
                &args,
                &defaults,
                &progress_tx,
            ) {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(diag) => return Err((wallets, diag)),
                },
                Err(data) => return Err((wallets, data)),
            };

            res_signing.append(&mut res);
            Ok((wallets, res_signing))
        };
        Ok(Box::pin(future))
    }

    fn build_background_task(
        uuid: &ConstructUuid,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
    ) -> CommandExecutionFutureResult {
        println!("build_background_task: {:?}", args);
        BroadcastStacksTransaction::build_background_task(
            &uuid,
            &spec,
            &args,
            &defaults,
            &progress_tx,
            &background_tasks_uuid,
        )
    }
}
