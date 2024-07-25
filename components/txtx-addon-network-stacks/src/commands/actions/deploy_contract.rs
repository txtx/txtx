use std::collections::HashMap;

use txtx_addon_kit::channel;
use txtx_addon_kit::{
    types::{
        commands::{
            CommandExecutionFutureResult, CommandImplementation, CommandSpecification,
            PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        frontend::BlockEvent,
        types::{RunbookSupervisionContext, Type, Value},
        wallets::{
            SigningCommandsState, WalletActionsFutureResult, WalletInstance, WalletSignFutureResult,
        },
        ConstructDid, ValueStore,
    },
    uuid::Uuid,
    AddonDefaults,
};

use crate::{
    constants::{
        SIGNED_TRANSACTION_BYTES, TRANSACTION_PAYLOAD_BYTES, TRANSACTION_POST_CONDITIONS_BYTES,
    },
    typing::STACKS_POST_CONDITION,
};

use super::encode_contract_deployment;
use super::{
    broadcast_transaction::BroadcastStacksTransaction, get_signing_construct_did,
    sign_transaction::SignStacksTransaction,
};

lazy_static! {
    pub static ref DEPLOY_STACKS_CONTRACT: PreCommandSpecification = define_command! {
      StacksDeployContract => {
        name: "Stacks Contract Deployment",
        matcher: "deploy_contract",
        documentation: "The `deploy_contract` action encodes a contract deployment transaction, signs the transaction using a wallet, and broadcasts the signed transaction to the network.",
        implements_signing_capability: true,
        implements_background_task_capability: true,
        inputs: [
            source_code: {
                documentation: "The code of the contract method to deploy.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            contract_name: {
                documentation: "The name of the contract to deploy.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            clarity_version: {
                documentation: "The version of clarity to use (default: latest).",
                typing: Type::uint(),
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
            action "counter_deployment" "stacks::deploy_contract" {
                description = "Deploy counter contract."
                source_code = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.pyth-oracle-v1"
                contract_name = "verify-and-update-price-feeds"
                signer = wallet.alice
            }
            output "contract_tx_id" {
              value = action.counter_deployment.tx_id
            }
            // > contract_tx_id: 0x1020321039120390239103193012909424854753848509019302931023849320
        "#},
      }
    };
}

pub struct StacksDeployContract;
impl CommandImplementation for StacksDeployContract {
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
        let contract_source = match args.get_expected_string("source_code") {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        let contract_name = match args.get_expected_string("contract_name") {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        let clarity_version = match args.get_expected_uint("clarity_version") {
            Ok(value) => Some(value),
            Err(_diag) => None,
        };

        let empty_vec = vec![];
        let post_conditions_values = args
            .get_expected_array("post_conditions")
            .unwrap_or(&empty_vec);
        let bytes =
            match encode_contract_deployment(spec, contract_source, contract_name, clarity_version)
            {
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
        // Extract network_id
        let contract_source = args.get_expected_string("source_code").unwrap();
        let contract_name = args.get_expected_string("contract_name").unwrap();
        let clarity_version = args.get_expected_uint("clarity_version").ok();

        let empty_vec = vec![];
        let post_conditions_values = args
            .get_expected_array("post_conditions")
            .unwrap_or(&empty_vec);
        let bytes =
            encode_contract_deployment(spec, contract_source, contract_name, clarity_version)
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
