use std::collections::HashMap;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::SignerKey;
use txtx_addon_kit::types::cloud_interface::CloudServiceContext;
use txtx_addon_kit::types::signers::SignerActionsFutureResult;
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::types::Value;
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

use crate::constants::TRANSACTION_POST_CONDITIONS_BYTES;
use crate::constants::TRANSACTION_POST_CONDITION_MODE_BYTES;
use crate::typing::STACKS_POST_CONDITIONS;
use crate::{
    constants::TRANSACTION_PAYLOAD_BYTES,
    typing::{STACKS_CV_GENERIC, STACKS_CV_PRINCIPAL},
};

use super::get_signer_did;
use super::{
    broadcast_transaction::BroadcastStacksTransaction, encode_contract_call,
    sign_transaction::SignStacksTransaction,
};

lazy_static! {
    pub static ref SEND_CONTRACT_CALL: PreCommandSpecification = define_command! {
        SendContractCall => {
            name: "Send Contract Call Transaction",
            matcher: "call_contract",
            documentation: "The `stacks::call_contract` action encodes a contract call transaction, signs the transaction using the specified signer, and broadcasts the signed transaction to the network.",
            implements_signing_capability: true,
            implements_background_task_capability: true,
            inputs: [
                description: {
                    documentation: "The description of the transaction.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                contract_id: {
                    documentation: "The Stacks address and contract name of the contract to invoke.",
                    typing: Type::addon(STACKS_CV_PRINCIPAL),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                function_name: {
                    documentation: "The contract method to invoke.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                function_args: {
                    documentation: "The function arguments for the contract call.",
                    typing: Type::array(Type::addon(STACKS_CV_GENERIC)),
                    optional: true,
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
                    tainting: false,
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
                },
                fee_strategy: {
                    documentation: "The strategy to use for automatically estimating fee ('low', 'medium', 'high'). Default to 'medium'.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                post_conditions: {
                    documentation: "The post conditions to include to the transaction.",
                    typing: Type::array(Type::addon(STACKS_POST_CONDITIONS)),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                post_condition_mode: {
                    documentation: "The post condition mode ('allow', 'deny'). In Allow mode other asset transfers not covered by the post-conditions are permitted. In Deny mode no other asset transfers are permitted besides those named in the post-conditions. The default is Deny mode.",
                    typing: Type::string(),
                    optional: true,
                    tainting: true,
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
                value: {
                    documentation: "The transaction id.",
                    typing: Type::string()
                },
                result: {
                    documentation: "The transaction result.",
                    typing: Type::buffer()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
                action "my_ref" "stacks::call_contract" {
                    description = "Encodes the contract call, sign, and broadcasts the set-token function."
                    contract_id = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.pyth-oracle-v1"
                    function_name = "verify-and-update-price-feeds"
                    function_args = [
                        stacks::cv_buff(output.bitcoin_price_feed),
                        stacks::cv_tuple({
                            "pyth-storage-contract": stacks::cv_principal("${input.pyth_deployer}.pyth-store-v1"),
                            "pyth-decoder-contract": stacks::cv_principal("${input.pyth_deployer}.pyth-pnau-decoder-v1"),
                            "wormhole-core-contract": stacks::cv_principal("${input.pyth_deployer}.wormhole-core-v1")
                        })
                    ]
                    signer = signer.alice
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
        values: &ValueStore,
        supervision_context: &RunbookSupervisionContext,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerActionsFutureResult {
        let signer_did = match get_signer_did(values) {
            Ok(value) => value,
            Err(diag) => return Err((signers, ValueStore::tmp(), diag)),
        };
        let signer_state = signers.pop_signer_state(&signer_did).unwrap();
        // Extract network_id
        let network_id: String = match values.get_expected_string("network_id") {
            Ok(value) => value.to_owned(),
            Err(diag) => return Err((signers, signer_state, diag)),
        };
        let contract_id_value = match values.get_expected_value("contract_id") {
            Ok(value) => value,
            Err(diag) => return Err((signers, signer_state, diag)),
        };
        let function_name = match values.get_expected_string("function_name") {
            Ok(value) => value,
            Err(diag) => return Err((signers, signer_state, diag)),
        };
        let empty_vec = vec![];
        let function_args_values = values.get_expected_array("function_args").unwrap_or(&empty_vec);
        let post_conditions_values =
            values.get_expected_array("post_conditions").unwrap_or(&empty_vec);
        let post_condition_mode = values.get_string("post_condition_mode").unwrap_or("deny");
        let bytes = match encode_contract_call(
            spec,
            function_name,
            function_args_values,
            &network_id,
            contract_id_value,
        ) {
            Ok(value) => value,
            Err(diag) => return Err((signers, signer_state, diag)),
        };
        signers.push_signer_state(signer_state);

        let mut args = values.clone();
        args.insert(TRANSACTION_PAYLOAD_BYTES, bytes);
        args.insert(
            TRANSACTION_POST_CONDITIONS_BYTES,
            Value::array(post_conditions_values.clone()),
        );
        args.insert(
            TRANSACTION_POST_CONDITION_MODE_BYTES,
            Value::string(post_condition_mode.to_string()),
        );

        SignStacksTransaction::check_signed_executability(
            construct_did,
            instance_name,
            spec,
            &args,
            supervision_context,
            signers_instances,
            signers,
        )
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        args: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        signers: SignersState,
    ) -> SignerSignFutureResult {
        let empty_vec = vec![];
        let network_id: String = args.get_expected_string("network_id").unwrap().to_owned();
        let contract_id_value = args.get_expected_value("contract_id").unwrap();
        let function_name = args.get_expected_string("function_name").unwrap();
        let function_args_values = args.get_expected_array("function_args").unwrap_or(&empty_vec);
        let post_conditions_values =
            args.get_expected_array("post_conditions").unwrap_or(&empty_vec);
        let post_condition_mode = args.get_string("post_condition_mode").unwrap_or("deny");

        let bytes = encode_contract_call(
            spec,
            function_name,
            &function_args_values,
            &network_id,
            contract_id_value,
        )
        .unwrap();
        let progress_tx = progress_tx.clone();
        let args = args.clone();
        let signers_instances = signers_instances.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();

        let mut args = args.clone();
        args.insert(TRANSACTION_PAYLOAD_BYTES, bytes);
        args.insert(
            TRANSACTION_POST_CONDITIONS_BYTES,
            Value::array(post_conditions_values.clone()),
        );
        args.insert(
            TRANSACTION_POST_CONDITION_MODE_BYTES,
            Value::string(post_condition_mode.to_string()),
        );

        let future = async move {
            let run_signing_future = SignStacksTransaction::run_signed_execution(
                &construct_did,
                &spec,
                &args,
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

            args.insert(
                SignerKey::SignedTransactionBytes,
                res_signing.outputs.get(SignerKey::SignedTransactionBytes.as_ref()).unwrap().clone(),
            );
            let mut res = match BroadcastStacksTransaction::run_execution(
                &construct_did,
                &spec,
                &args,
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
