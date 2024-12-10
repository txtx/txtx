use alloy::consensus::Transaction;
use alloy::network::{EthereumWallet, TransactionBuilder};
use alloy::primitives::Address;
use alloy_rpc_types::TransactionRequest;
use std::collections::HashMap;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::SIGNATURE_APPROVED;
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemStatus, ProvideSignedTransactionRequest, ReviewInputRequest,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::signers::{
    return_synchronous_result, CheckSignabilityOk, SignerActionErr, SignerActionsFutureResult,
    SignerActivateFutureResult, SignerInstance, SignerSignFutureResult,
};
use txtx_addon_kit::types::signers::{SignerImplementation, SignerSpecification};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{
    signers::SignersState, types::RunbookSupervisionContext, ConstructDid,
};

use crate::codec::crypto::field_bytes_to_secret_key_signer;
use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION, CHAIN_ID,
    FORMATTED_TRANSACTION, RPC_API_URL, SECRET_KEY_WALLET_UNSIGNED_TRANSACTION_BYTES, TX_HASH,
};
use crate::rpc::EvmWalletRpc;
use crate::typing::EvmValue;
use txtx_addon_kit::types::signers::return_synchronous_actions;
use txtx_addon_kit::types::signers::{signer_diag_with_ctx, signer_err_fn};

use crate::constants::PUBLIC_KEYS;
use crate::signers::namespaced_err_fn;

use super::common::set_signer_nonce;

lazy_static! {
    pub static ref EVM_SECRET_KEY_SIGNER: SignerSpecification = define_signer! {
        EvmSecretKeySigner => {
          name: "EVM Secret Key Signer",
          matcher: "secret_key",
          documentation:txtx_addon_kit::indoc! {r#"The `evm::secret_key` signer can be used to synchronously sign a transaction."#},
          inputs: [
            secret_key: {
                documentation: "The secret key used to sign messages and transactions.",
                typing: Type::string(),
                optional: true,
                tainting: true,
                sensitive: true
            },
            mnemonic: {
                documentation: "The mnemonic phrase used to generate the secret key. This input will not be used if the `secret_key` input is provided.",
                typing: Type::string(),
                optional: true,
                tainting: true,
                sensitive: true
            },
            derivation_path: {
                documentation: "The derivation path used to generate the secret key. This input will not be used if the `secret_key` input is provided.",
                typing: Type::string(),
                optional: true,
                tainting: true,
                sensitive: true
            },
            is_encrypted: {
                documentation: "Coming soon",
                typing: Type::bool(),
                optional: true,
                tainting: true,
                sensitive: false
            },
            password: {
                documentation: "Coming soon",
                typing: Type::string(),
                optional: true,
                tainting: true,
                sensitive: true
            }
          ],
          outputs: [
              public_key: {
                documentation: "The public key of the account generated from the secret key.",
                typing: Type::array(Type::buffer())
              },
              address: {
                documentation: "The address generated from the secret key.",
                typing: Type::array(Type::buffer())
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
        // we can create a secret key signer by providing a mnemonic and computing the secret key
        signer "bob" "evm::secret_key" {
            mnemonic = "board list obtain sugar hour worth raven scout denial thunder horse logic fury scorpion fold genuine phrase wealth news aim below celery when cabin"
            derivation_path = "m/44'/5757'/0'/0/0"
        }
        // or we can create one by providing the secret key directly
        signer "bob_again" "evm::secret_key" {
            secret_key = "03b3e0a76b292b2c83fc0ac14ae6160d0438ebe94e14bbb5b7755153628886e08e"
        }
    "#},
      }
    };
}

pub struct EvmSecretKeySigner;
impl SignerImplementation for EvmSecretKeySigner {
    fn check_instantiability(
        _ctx: &SignerSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    #[cfg(not(feature = "wasm"))]
    fn check_activability(
        _construct_id: &ConstructDid,
        instance_name: &str,
        spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        use crate::codec::crypto::{
            mnemonic_to_secret_key_signer, secret_key_to_secret_key_signer,
        };
        let signer_err =
            signer_err_fn(signer_diag_with_ctx(spec, instance_name, namespaced_err_fn()));

        let mut actions = Actions::none();

        if signer_state.get_value(PUBLIC_KEYS).is_some() {
            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        }

        let expected_signer =
            if let Ok(secret_key_bytes) = values.get_expected_buffer_bytes("secret_key") {
                secret_key_to_secret_key_signer(&secret_key_bytes)
                    .map_err(|e| signer_err(&signers, &signer_state, e))?
            } else {
                let mnemonic = values
                    .get_expected_string("mnemonic")
                    .map_err(|e| signer_err(&signers, &signer_state, e.message))?;
                let derivation_path = values.get_string("derivation_path");
                let is_encrypted = values.get_bool("is_encrypted");
                let password = values.get_string("password");
                mnemonic_to_secret_key_signer(mnemonic, derivation_path, is_encrypted, password)
                    .map_err(|e| signer_err(&signers, &signer_state, e))?
            };

        let expected_address: Address = expected_signer.address();
        signer_state.insert(
            "signer_field_bytes",
            EvmValue::signer_field_bytes(expected_signer.to_field_bytes().to_vec()),
        );
        signer_state.insert("signer_address", Value::string(expected_address.to_string()));

        if supervision_context.review_input_values {
            actions.push_sub_group(
                None,
                vec![ActionItemRequest::new(
                    &None,
                    &format!("Check {} expected address", instance_name),
                    None,
                    ActionItemStatus::Todo,
                    ReviewInputRequest::new("", &Value::string(expected_address.to_string()))
                        .to_action_type(),
                    ACTION_ITEM_CHECK_ADDRESS,
                )],
            );
        }
        let future = async move { Ok((signers, signer_state, actions)) };
        Ok(Box::pin(future))
    }

    fn activate(
        _construct_id: &ConstructDid,
        spec: &SignerSpecification,
        _values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        let signer_did = ConstructDid(signer_state.uuid.clone());
        let signer_instance = signers_instances.get(&signer_did).unwrap();
        let signer_err =
            signer_err_fn(signer_diag_with_ctx(&spec, &signer_instance.name, namespaced_err_fn()));

        let mut result = CommandExecutionResult::new();
        let address = signer_state
            .get_expected_value("signer_address")
            .map_err(|e| signer_err(&signers, &signer_state, e.message))?;
        result.outputs.insert("address".into(), address.clone());
        return_synchronous_result(Ok((signers, signer_state, result)))
    }

    fn check_signability(
        construct_did: &ConstructDid,
        title: &str,
        description: &Option<String>,
        payload: &Value,
        spec: &SignerSpecification,
        values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        let signer_did = ConstructDid(signer_state.uuid.clone());
        let signer_instance = signers_instances.get(&signer_did).unwrap();
        let signer_err =
            signer_err_fn(signer_diag_with_ctx(spec, &signer_instance.name, namespaced_err_fn()));

        let actions = if supervision_context.review_input_values {
            let construct_did_str = &construct_did.to_string();
            if let Some(_) = signer_state.get_scoped_value(&construct_did_str, SIGNATURE_APPROVED) {
                return Ok((signers, signer_state, Actions::none()));
            }

            let chain_id = values
                .get_expected_uint(CHAIN_ID)
                .map_err(|e| signer_err(&signers, &signer_state, e.message))?;

            let status = ActionItemStatus::Todo;

            let formatted_payload = signer_state
                .get_scoped_value(&construct_did_str, FORMATTED_TRANSACTION)
                .and_then(|v| v.as_string())
                .and_then(|v| Some(v.to_string()));

            let request = ActionItemRequest::new(
                &Some(construct_did.clone()),
                title,
                description.clone(),
                status,
                ProvideSignedTransactionRequest::new(
                    &signer_state.uuid,
                    &payload,
                    "stacks",
                    &chain_id.to_string(),
                )
                .check_expectation_action_uuid(construct_did)
                .only_approval_needed()
                .formatted_payload(formatted_payload)
                .to_action_type(),
                ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION,
            );
            Actions::append_item(
                request,
                Some("Review and sign the transactions from the list below"),
                Some("Transaction Signing"),
            )
        } else {
            Actions::none()
        };
        Ok((signers, signer_state, actions))
    }

    fn sign(
        caller_uuid: &ConstructDid,
        _title: &str,
        _payload: &Value,
        spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> SignerSignFutureResult {
        let caller_uuid = caller_uuid.clone();
        let values = values.clone();
        let signers_instances = signers_instances.clone();
        let spec = spec.clone();

        let future = async move {
            let signer_did = ConstructDid(signer_state.uuid.clone());
            let signer_instance = signers_instances.get(&signer_did).unwrap();
            let signer_err = signer_err_fn(signer_diag_with_ctx(
                &spec,
                &signer_instance.name,
                namespaced_err_fn(),
            ));

            let mut result = CommandExecutionResult::new();

            let rpc_api_url = values
                .get_expected_string(RPC_API_URL)
                .map_err(|e| signer_err(&signers, &signer_state, e.message))?;

            let signer_field_bytes = signer_state
                .get_expected_buffer_bytes("signer_field_bytes")
                .map_err(|e| signer_err(&signers, &signer_state, e.message))?;

            let payload_bytes = signer_state
                .get_expected_scoped_buffer_bytes(
                    &caller_uuid.to_string(),
                    SECRET_KEY_WALLET_UNSIGNED_TRANSACTION_BYTES,
                )
                .unwrap();

            let secret_key_signer = field_bytes_to_secret_key_signer(&signer_field_bytes)
                .map_err(|e| signer_err(&signers, &signer_state, e))?;

            let eth_signer = EthereumWallet::from(secret_key_signer);

            let mut tx: TransactionRequest = serde_json::from_slice(&payload_bytes).unwrap();
            if tx.to.is_none() {
                // there's no to address on the tx, which is invalid unless it's set as "create"
                tx.set_create();
            }

            let tx_envelope = tx.build(&eth_signer).await.map_err(|e| {
                signer_err(
                    &signers,
                    &signer_state,
                    format!("failed to build transaction envelope: {e}"),
                )
            })?;
            let tx_nonce = tx_envelope.nonce();
            let tx_chain_id = tx_envelope.chain_id().unwrap();

            let rpc = EvmWalletRpc::new(&rpc_api_url, eth_signer)
                .map_err(|e| signer_err(&signers, &signer_state, e))?;
            let tx_hash = rpc
                .sign_and_send_tx(tx_envelope)
                .await
                .map_err(|e| signer_err(&signers, &signer_state, e.to_string()))?;

            result.outputs.insert(TX_HASH.to_string(), EvmValue::tx_hash(tx_hash.to_vec()));
            set_signer_nonce(&mut signer_state, tx_chain_id, tx_nonce);
            Ok((signers, signer_state, result))
        };
        Ok(Box::pin(future))
    }
}
