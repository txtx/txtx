use std::collections::HashMap;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::SignerKey;
use txtx_addon_kit::types::cloud_interface::CloudServiceContext;
use txtx_addon_kit::types::signers::SignerActionsFutureResult;
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::{
    commands::{
        CommandExecutionFutureResult, CommandImplementation, CommandSpecification,
        PreCommandSpecification,
    },
    diagnostics::Diagnostic,
    frontend::BlockEvent,
    signers::{SignerInstance, SignerSignFutureResult, SignersState},
    types::Type,
    ConstructDid,
};
use txtx_addon_kit::uuid::Uuid;

use crate::{constants::TRANSACTION_PAYLOAD_BYTES, typing::STACKS_CV_PRINCIPAL};

use super::encode_stx_transfer;
use super::get_signer_did;
use super::{
    broadcast_transaction::BroadcastStacksTransaction, sign_transaction::SignStacksTransaction,
};

lazy_static! {
    pub static ref SEND_STX_TRANSFER: PreCommandSpecification = define_command! {
        SendStxTransfer => {
            name: "Send STX Transfer Transaction",
            matcher: "send_stx",
            documentation: "The `stacks::send_stx` action encodes a STX transfer transaction, signs the transaction using the specified signer, and broadcasts the signed transaction to the network.",
            implements_signing_capability: true,
            implements_background_task_capability: true,
            inputs: [
                amount: {
                    documentation: "The amount to send, in microSTX (1 STX = 10^6 µSTX).",
                    typing: Type::addon(STACKS_CV_PRINCIPAL),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                recipient: {
                    documentation: "The Stacks address of the recipient.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                network_id: {
                    documentation: indoc!{r#"The network id. Valid values are `"mainnet"`, `"testnet"` or `"devnet"`."#},
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                rpc_api_url: {
                    documentation: "The URL to use when making API requests.",
                    typing: Type::string(),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                rpc_api_auth_token: {
                    documentation: "The HTTP authentication token to include in the headers when making API requests.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                signer: {
                    documentation: "A reference to a signer construct, which will be used to sign the transaction payload.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                confirmations: {
                    documentation: "Once the transaction is included on a block, the number of blocks to await before the transaction is considered successful and Runbook execution continues. The default is 1.",
                    typing: Type::integer(),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                nonce: {
                    documentation: "The account nonce of the signer. This value will be retrieved from the network if omitted.",
                    typing: Type::integer(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                fee: {
                    documentation: "The transaction fee. This value will automatically be estimated if omitted.",
                    typing: Type::integer(),
                    optional: true,
                    tainting: false,
                    internal: false
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
            action "stx_transfer" "stacks::send_stx" {
                description = "Send µSTX to Bob."
                recipient = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM"
                amount = 1000000
                memo = "0x10394390"
                signer = signer.alice
            }            
            output "transfer_tx_id" {
                value = action.stx_transfer.tx_id
            }
            // > transfer_tx_id: 0x1020321039120390239103193012909424854753848509019302931023849320
        "#},
      }
    };
}

pub struct SendStxTransfer;
impl CommandImplementation for SendStxTransfer {
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
        values: &ValueStore,
        supervision_context: &RunbookSupervisionContext,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerActionsFutureResult {
        let signer_did = get_signer_did(values).unwrap();
        let signer_state = signers.pop_signer_state(&signer_did).unwrap();
        // Extract network_id
        let network_id: String = match values.get_expected_string("network_id") {
            Ok(value) => value.to_owned(),
            Err(diag) => return Err((signers, signer_state, diag)),
        };
        let recipient = match values.get_expected_value("recipient") {
            Ok(value) => value,
            Err(diag) => return Err((signers, signer_state, diag)),
        };
        let amount = match values.get_expected_uint("amount") {
            Ok(value) => value,
            Err(diag) => return Err((signers, signer_state, diag)),
        };
        let memo = values.get_value("memo");

        let bytes = match encode_stx_transfer(spec, recipient, amount, &memo, &network_id) {
            Ok(value) => value,
            Err(diag) => return Err((signers, signer_state, diag)),
        };
        signers.push_signer_state(signer_state);

        let mut values = values.clone();
        values.insert(TRANSACTION_PAYLOAD_BYTES, bytes);

        SignStacksTransaction::check_signed_executability(
            construct_did,
            instance_name,
            spec,
            &values,
            supervision_context,
            signers_instances,
            signers,
        )
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        values: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        signers: SignersState,
    ) -> SignerSignFutureResult {
        let network_id: String = values.get_expected_string("network_id").unwrap().to_owned();

        let recipient = values.get_expected_value("recipient").unwrap();
        let memo = values.get_value("memo");
        let amount = values.get_expected_uint("amount").unwrap();

        let bytes = encode_stx_transfer(spec, recipient, amount, &memo, &network_id).unwrap();
        let progress_tx = progress_tx.clone();
        let args = values.clone();
        let signers_instances = signers_instances.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();

        let mut values = args.clone();
        values.insert(TRANSACTION_PAYLOAD_BYTES, bytes);

        let future = async move {
            let run_signing_future = SignStacksTransaction::run_signed_execution(
                &construct_did,
                &spec,
                &values,
                &progress_tx,
                &signers_instances,
                signers,
            );
            let (signers, signer_state, mut res_signing) = match run_signing_future {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(err) => return Err(err),
                },
                Err(err) => return Err(err),
            };

            values.insert(
                SignerKey::SignedTransactionBytes.as_ref(),
                res_signing.outputs.get(SignerKey::SignedTransactionBytes.as_ref()).unwrap().clone(),
            );
            let mut res = match BroadcastStacksTransaction::run_execution(
                &construct_did,
                &spec,
                &values,
                &progress_tx,
            ) {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(diag) => return Err((signers, signer_state, diag)),
                },
                Err(data) => return Err((signers, signer_state, data)),
            };

            res_signing.append(&mut res);

            Ok((signers, signer_state, res_signing))
        };
        Ok(Box::pin(future))
    }

    fn build_background_task(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        inputs: &ValueStore,
        outputs: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
        cloud_service_context: &Option<CloudServiceContext>,
    ) -> CommandExecutionFutureResult {
        BroadcastStacksTransaction::build_background_task(
            &construct_did,
            &spec,
            &inputs,
            &outputs,
            &progress_tx,
            &background_tasks_uuid,
            &supervision_context,
            &cloud_service_context,
        )
    }
}
