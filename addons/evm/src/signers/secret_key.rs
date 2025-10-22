use alloy::consensus::Transaction;
use alloy::network::{EthereumWallet, TransactionBuilder};
use alloy::primitives::Address;
use alloy_rpc_types::TransactionRequest;
use std::collections::HashMap;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::SignerKey;
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemStatus, ProvideSignedTransactionRequest, ReviewInputRequest,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::signers::{
    return_synchronous_result, CheckSignabilityOk, SignerActionErr, SignerActionsFutureResult,
    SignerActivateFutureResult, SignerInstance, SignerSignFutureResult,
};
use txtx_addon_kit::types::signers::{SignerImplementation, SignerSpecification};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::AuthorizationContext;
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
    CHAIN_ID, FORMATTED_TRANSACTION, NAMESPACE, RPC_API_URL, SECRET_KEY_WALLET_UNSIGNED_TRANSACTION_BYTES,
    TX_HASH,
};
use txtx_addon_kit::constants::ActionItemKey;
use crate::rpc::EvmWalletRpc;
use crate::typing::EvmValue;
use txtx_addon_kit::types::signers::return_synchronous_actions;

use crate::constants::PUBLIC_KEYS;

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
        "#}
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
        construct_did: &ConstructDid,
        instance_name: &str,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
        auth_ctx: &txtx_addon_kit::types::AuthorizationContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        use txtx_addon_kit::constants::DocumentationKey;

        use crate::{
            codec::crypto::{mnemonic_to_secret_key_signer, secret_key_to_secret_key_signer},
            constants::CHECKED_ADDRESS,
        };

        let mut actions = Actions::none();

        if signer_state.get_value(PUBLIC_KEYS).is_some() {
            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        }
        let description = values.get_string(DocumentationKey::Description).map(|d| d.to_string());
        let markdown = values
            .get_markdown(auth_ctx)
            .map_err(|d| (signers.clone(), signer_state.clone(), d))?;

        let expected_signer =
            if let Ok(secret_key_bytes) = values.get_expected_buffer_bytes("secret_key") {
                secret_key_to_secret_key_signer(&secret_key_bytes)
                    .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?
            } else {
                let mnemonic = values
                    .get_expected_string("mnemonic")
                    .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
                let derivation_path = values.get_string("derivation_path");
                let is_encrypted = values.get_bool("is_encrypted");
                let password = values.get_string("password");
                mnemonic_to_secret_key_signer(mnemonic, derivation_path, is_encrypted, password)
                    .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?
            };

        let expected_address: Address = expected_signer.address();

        if supervision_context.review_input_values {
            if let Ok(_) = signer_state.get_expected_string(CHECKED_ADDRESS) {
                signer_state.insert(
                    "signer_field_bytes",
                    EvmValue::signer_field_bytes(expected_signer.to_field_bytes().to_vec()),
                );
                signer_state.insert("signer_address", Value::string(expected_address.to_string()));
                signer_state.insert(CHECKED_ADDRESS, Value::string(expected_address.to_string()));
            } else {
                actions.push_sub_group(
                    None,
                    vec![ReviewInputRequest::new("", &Value::string(expected_address.to_string()))
                        .to_action_type()
                        .to_request(instance_name, ActionItemKey::CheckAddress)
                        .with_construct_did(construct_did)
                        .with_some_description(description.clone())
                        .with_meta_description(&format!("Check {} expected address", instance_name))
                        .with_some_markdown(markdown.clone())],
                );
            }
        } else {
            signer_state.insert(CHECKED_ADDRESS, Value::string(expected_address.to_string()));
            signer_state.insert(
                "signer_field_bytes",
                EvmValue::signer_field_bytes(expected_signer.to_field_bytes().to_vec()),
            );
            signer_state.insert("signer_address", Value::string(expected_address.to_string()));
        }
        let future = async move { Ok((signers, signer_state, actions)) };
        Ok(Box::pin(future))
    }

    fn activate(
        _construct_id: &ConstructDid,
        _spec: &SignerSpecification,
        _values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        let mut result = CommandExecutionResult::new();
        let address = signer_state
            .get_expected_value("signer_address")
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
        result.outputs.insert("address".into(), address.clone());
        return_synchronous_result(Ok((signers, signer_state, result)))
    }

    fn check_signability(
        construct_did: &ConstructDid,
        title: &str,
        description: &Option<String>,
        meta_description: &Option<String>,
        markdown: &Option<String>,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
        _auth_ctx: &AuthorizationContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        let actions = if supervision_context.review_input_values {
            let construct_did_str = &construct_did.to_string();
            if let Some(_) = signer_state.get_scoped_value(&construct_did_str, SignerKey::SignatureApproved) {
                return Ok((signers, signer_state, Actions::none()));
            }

            let chain_id = values
                .get_expected_uint(CHAIN_ID)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

            let status = ActionItemStatus::Todo;

            let formatted_payload =
                signer_state.get_scoped_value(&construct_did_str, FORMATTED_TRANSACTION);

            let request = ProvideSignedTransactionRequest::new(
                &signer_state.uuid,
                &payload,
                NAMESPACE,
                &chain_id.to_string(),
            )
            .check_expectation_action_uuid(construct_did)
            .only_approval_needed()
            .formatted_payload(formatted_payload)
            .to_action_type()
            .to_request(title, ActionItemKey::ProvideSignedTransaction)
            .with_construct_did(construct_did)
            .with_some_description(description.clone())
            .with_some_meta_description(meta_description.clone())
            .with_some_markdown(markdown.clone())
            .with_status(status);

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
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> SignerSignFutureResult {
        let caller_uuid = caller_uuid.clone();
        let values = values.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            let rpc_api_url = values
                .get_expected_string(RPC_API_URL)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

            let signer_field_bytes = signer_state
                .get_expected_buffer_bytes("signer_field_bytes")
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

            let payload_bytes = signer_state
                .get_expected_scoped_buffer_bytes(
                    &caller_uuid.to_string(),
                    SECRET_KEY_WALLET_UNSIGNED_TRANSACTION_BYTES,
                )
                .unwrap();

            let secret_key_signer = field_bytes_to_secret_key_signer(&signer_field_bytes)
                .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

            let eth_signer = EthereumWallet::from(secret_key_signer);

            let mut tx: TransactionRequest = serde_json::from_slice(&payload_bytes).unwrap();
            if tx.to.is_none() {
                // there's no to address on the tx, which is invalid unless it's set as "create"
                tx.set_create();
            }

            let tx_envelope = tx.build(&eth_signer).await.map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("failed to build transaction envelope: {e}"),
                )
            })?;
            let tx_nonce = tx_envelope.nonce();
            let tx_chain_id = tx_envelope.chain_id().unwrap();

            let rpc = EvmWalletRpc::new(&rpc_api_url, eth_signer)
                .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;
            let tx_hash = rpc.sign_and_send_tx(tx_envelope).await.map_err(|e| {
                (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e.to_string()))
            })?;

            result.outputs.insert(SignerKey::TxHash.to_string(), EvmValue::tx_hash(tx_hash.to_vec()));
            set_signer_nonce(&mut signer_state, tx_chain_id, tx_nonce);
            Ok((signers, signer_state, result))
        };
        Ok(Box::pin(future))
    }
}
