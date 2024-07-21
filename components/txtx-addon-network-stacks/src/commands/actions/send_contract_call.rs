use std::collections::HashMap;
use txtx_addon_kit::channel;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::types::Value;
use txtx_addon_kit::types::wallets::WalletActionsFutureResult;
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::{
    types::{
        commands::{
            CommandExecutionFutureResult, CommandImplementation, CommandSpecification,
            PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        frontend::BlockEvent,
        types::Type,
        wallets::{SigningCommandsState, WalletInstance, WalletSignFutureResult},
        ConstructDid, ValueStore,
    },
    AddonDefaults,
};

use crate::constants::TRANSACTION_POST_CONDITIONS_BYTES;
use crate::typing::STACKS_POST_CONDITION;
use crate::{
    constants::{SIGNED_TRANSACTION_BYTES, TRANSACTION_PAYLOAD_BYTES},
    typing::{CLARITY_PRINCIPAL, CLARITY_VALUE},
};

use super::get_signing_construct_did;
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
                  documentation: "A reference to a wallet construct, which will be used to sign the transaction payload.",
                  typing: Type::string(),
                  optional: false,
                  interpolable: true
              },
              confirmations: {
                documentation: "Once the transaction is included on a block, the number of blocks to await before the transaction is considered successful and Runbook execution continues.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
              },
              nonce: {
                  documentation: "The account nonce of the signer. This value will be retrieved from the network if omitted.",
                  typing: Type::uint(),
                  optional: true,
                  interpolable: true
              },
              fee: {
                documentation: "The transaction fee. This value will automatically be estimated if omitted.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
              },
              post_conditions: {
                documentation: "The post conditions to include to the transaction.",
                typing: Type::array(Type::addon(STACKS_POST_CONDITION.clone())),
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
                    stacks::cv_buff(output.bitcoin_price_feed),
                    stacks::cv_tuple({
                        "pyth-storage-contract": stacks::cv_principal("${env.pyth_deployer}.pyth-store-v1"),
                        "pyth-decoder-contract": stacks::cv_principal("${env.pyth_deployer}.pyth-pnau-decoder-v1"),
                        "wormhole-core-contract": stacks::cv_principal("${env.pyth_deployer}.wormhole-core-v1")
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
        construct_did: &ConstructDid,
        instance_name: &str,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        mut wallets: SigningCommandsState,
    ) -> WalletActionsFutureResult {
        let signing_construct_did = get_signing_construct_did(args).unwrap();
        let signing_command_state = wallets
            .pop_signing_command_state(&signing_construct_did)
            .unwrap();
        // Extract network_id
        let network_id: String = match args.get_defaulting_string("network_id", defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        let contract_id_value = match args.get_expected_value("contract_id") {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        let function_name = match args.get_expected_string("function_name") {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        let function_args_values = match args.get_expected_array("function_args") {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        let empty_vec = vec![];
        let post_conditions_values = args.get_expected_array("post_conditions").unwrap_or(&empty_vec);
        let bytes = match encode_contract_call(
            spec,
            function_name,
            function_args_values,
            &network_id,
            contract_id_value,
        ) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        wallets.push_signing_command_state(signing_command_state);

        let mut args = args.clone();
        args.insert(TRANSACTION_PAYLOAD_BYTES, bytes);
        args.insert(
            TRANSACTION_POST_CONDITIONS_BYTES,
            Value::array(post_conditions_values.clone()),
        );

        SignStacksTransaction::check_signed_executability(
            construct_did,
            instance_name,
            spec,
            &args,
            defaults,
            supervision_context,
            wallets_instances,
            wallets,
        )
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        wallets: SigningCommandsState,
    ) -> WalletSignFutureResult {
        let empty_vec = vec![];
        let network_id: String = args.get_defaulting_string("network_id", defaults).unwrap();
        let contract_id_value = args.get_expected_value("contract_id").unwrap();
        let function_name = args.get_expected_string("function_name").unwrap();
        let function_args_values = args.get_expected_array("function_args").unwrap();
        let post_conditions_values = args.get_expected_array("post_conditions").unwrap_or(&empty_vec);

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
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();

        let mut args = args.clone();
        args.insert(TRANSACTION_PAYLOAD_BYTES, bytes);
        args.insert(
            TRANSACTION_POST_CONDITIONS_BYTES,
            Value::array(post_conditions_values.clone()),
        );

        let future = async move {
            let run_signing_future = SignStacksTransaction::run_signed_execution(
                &construct_did,
                &spec,
                &args,
                &defaults,
                &progress_tx,
                &wallets_instances,
                wallets,
            );
            let (wallets, signing_command_state, mut res_signing) = match run_signing_future {
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
                &construct_did,
                &spec,
                &args,
                &defaults,
                &progress_tx,
            ) {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(diag) => return Err((wallets, signing_command_state, diag)),
                },
                Err(data) => return Err((wallets, signing_command_state, data)),
            };

            res_signing.append(&mut res);

            Ok((wallets, signing_command_state, res_signing))
        };
        Ok(Box::pin(future))
    }

    fn build_background_task(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        inputs: &ValueStore,
        outputs: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        BroadcastStacksTransaction::build_background_task(
            &construct_did,
            &spec,
            &inputs,
            &outputs,
            &defaults,
            &progress_tx,
            &background_tasks_uuid,
            &supervision_context,
        )
    }
}
