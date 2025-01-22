use std::collections::HashMap;
use std::sync::Arc;

use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::signature::Keypair;
use solana_sdk::transaction::Transaction;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::{
    SIGNATURE_APPROVED, SIGNATURE_SKIPPABLE, SIGNED_TRANSACTION_BYTES,
};
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemStatus, ProvideSignedTransactionRequest, ReviewInputRequest,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::signers::{
    return_synchronous_result, CheckSignabilityOk, SignerActionErr, SignerActionsFutureResult,
    SignerActivateFutureResult, SignerInstance, SignerSignFutureResult, SignersState,
};
use txtx_addon_kit::types::signers::{SignerImplementation, SignerSpecification};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::AuthorizationContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};

use crate::codec::send_transaction::send_transaction;
use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION, ADDRESS, CHECKED_ADDRESS,
    CHECKED_PUBLIC_KEY, COMMITMENT_LEVEL, DO_AWAIT_CONFIRMATION, FORMATTED_TRANSACTION,
    IS_DEPLOYMENT, IS_SIGNABLE, NAMESPACE, NETWORK_ID, PARTIALLY_SIGNED_TRANSACTION_BYTES,
    PUBLIC_KEY, RPC_API_URL, SECRET_KEY, SIGNATURE, TRANSACTION_BYTES,
};
use crate::typing::{SvmValue, SVM_TEMP_AUTHORITY_SIGNED_TRANSACTION};
use txtx_addon_kit::types::signers::return_synchronous_actions;
use txtx_addon_kit::types::types::RunbookSupervisionContext;

lazy_static! {
    pub static ref SVM_SECRET_KEY: SignerSpecification = define_signer! {
        SvmSecretKey => {
            name: "Secret Key Signer",
            matcher: "secret_key",
            documentation:txtx_addon_kit::indoc! {r#"The `svm::secret_key` signer can be used to synchronously sign a transaction."#},
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
                keypair_json: {
                    documentation: "A path to a keypair.json file containing the secret key. This input will not be used if the `secret_key` or `mnemonic` inputs are provided.",
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
                    documentation: "The public key of the account generated from the secret key, mnemonic, or keypair file.",
                    typing: Type::string()
                },
                address: {
                    documentation: "The SVM address generated from the secret key, mnemonic, or keypair file. This is an alias for the `public_key` output.",
                    typing: Type::string()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
            signer "deployer" "svm::secret_key" {
                secret_key = input.secret_key
            }
        "#},
        }
    };
}

pub struct SvmSecretKey;
impl SignerImplementation for SvmSecretKey {
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
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
        auth_ctx: &AuthorizationContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        use crate::constants::{
            DEFAULT_DERIVATION_PATH, DERIVATION_PATH, IS_ENCRYPTED, KEYPAIR_JSON, MNEMONIC,
            PASSWORD, SECRET_KEY,
        };
        use solana_sdk::{signature::Keypair, signer::Signer};
        use txtx_addon_kit::crypto::secret_key_bytes_from_mnemonic;
        let mut actions = Actions::none();

        if signer_state.get_value(CHECKED_PUBLIC_KEY).is_some() {
            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        }

        let secret_key_bytes = match values.get_value(SECRET_KEY) {
            None => match values.get_string(MNEMONIC) {
                None => {
                    let keypair_json_str = values
                        .get_expected_string(KEYPAIR_JSON)
                        .map_err(|_| (signers.clone(), signer_state.clone(), diagnosed_error!("either `secret_key`, `mnemonic`, or `keypair_json` fields are required")))?;

                    let keypair_json =
                        auth_ctx.get_path_from_str(keypair_json_str).map_err(|e| {
                            (
                                signers.clone(),
                                signer_state.clone(),
                                diagnosed_error!(
                                    "invalid keypair file location ({}): {}",
                                    keypair_json_str,
                                    e
                                ),
                            )
                        })?;

                    let keypair_bytes = keypair_json.read_content().map_err(|e| {
                        (
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!(
                                "failed to read keypair file ({}): {}",
                                keypair_json_str,
                                e
                            ),
                        )
                    })?;
                    let keypair: Vec<u8> = serde_json::from_slice(&keypair_bytes).map_err(|e| {
                        (
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!(
                                "failed to deserialize keypair file ({}): {}",
                                keypair_json_str,
                                e
                            ),
                        )
                    })?;
                    keypair
                }
                Some(mnemonic) => {
                    let derivation_path =
                        values.get_string(DERIVATION_PATH).unwrap_or(DEFAULT_DERIVATION_PATH);
                    let is_encrypted = values.get_bool(IS_ENCRYPTED).unwrap_or(false);
                    let password = values.get_string(PASSWORD);
                    secret_key_bytes_from_mnemonic(
                        mnemonic,
                        derivation_path,
                        is_encrypted,
                        password,
                    )
                    .map_err(|e| {
                        (
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("invalid mnemonic: {e}"),
                        )
                    })?
                    .to_vec()
                }
            },
            Some(value) => match value
                .try_get_buffer_bytes_result()
                .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?
            {
                Some(bytes) => bytes.clone(),
                None => unreachable!(),
            },
        };

        let keypair = Keypair::from_bytes(&secret_key_bytes).map_err(|e| {
            (signers.clone(), signer_state.clone(), diagnosed_error!("invalid secret key: {e}"))
        })?;

        let expected_address = keypair.pubkey().to_string();
        let public_key = Value::string(keypair.pubkey().to_string());
        let secret_key = Value::buffer(secret_key_bytes);

        signer_state.insert(CHECKED_PUBLIC_KEY, public_key.clone());
        signer_state.insert(CHECKED_ADDRESS, Value::string(expected_address.to_string()));
        signer_state.insert(SECRET_KEY, secret_key);

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
        _construct_did: &ConstructDid,
        _spec: &SignerSpecification,
        _values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        let mut result = CommandExecutionResult::new();
        let public_key = signer_state.get_value(CHECKED_PUBLIC_KEY).unwrap();
        let address = signer_state.get_value(CHECKED_ADDRESS).unwrap();
        result.outputs.insert(ADDRESS.into(), address.clone());
        result.outputs.insert(PUBLIC_KEY.into(), public_key.clone());
        return_synchronous_result(Ok((signers, signer_state, result)))
    }

    fn check_signability(
        construct_did: &ConstructDid,
        title: &str,
        description: &Option<String>,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            TRANSACTION_BYTES,
            payload.clone(),
        );

        let actions = if supervision_context.review_input_values {
            let construct_did_str = &construct_did.to_string();
            if let Some(_) = signer_state.get_scoped_value(&construct_did_str, SIGNATURE_APPROVED) {
                return Ok((signers, signer_state, Actions::none()));
            }

            let network_id = match values.get_expected_string(NETWORK_ID) {
                Ok(value) => value,
                Err(diag) => return Err((signers, signer_state, diag)),
            };
            let signable = signer_state
                .get_scoped_value(&construct_did_str, IS_SIGNABLE)
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let status = match signable {
                true => ActionItemStatus::Todo,
                false => ActionItemStatus::Blocked,
            };
            let skippable = signer_state
                .get_scoped_value(&construct_did_str, SIGNATURE_SKIPPABLE)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let formatted_payload = signer_state
                .get_scoped_value(&construct_did_str, FORMATTED_TRANSACTION)
                .and_then(|v| v.as_string().map(|s| s.to_string()));

            let request = ActionItemRequest::new(
                &Some(construct_did.clone()),
                title,
                description.clone(),
                status,
                ProvideSignedTransactionRequest::new(
                    &signer_state.uuid,
                    &payload,
                    NAMESPACE,
                    &network_id,
                )
                .skippable(skippable)
                .check_expectation_action_uuid(construct_did)
                .formatted_payload(formatted_payload)
                .only_approval_needed()
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

    /// Signing will fail if
    ///
    /// - The transaction's [`Message`] is malformed such that the number of
    ///   required signatures recorded in its header
    ///   ([`num_required_signatures`]) is greater than the length of its
    ///   account keys ([`account_keys`]). The error is
    ///   [`SignerError::TransactionError`] where the interior
    ///   [`TransactionError`] is [`TransactionError::InvalidAccountIndex`].
    /// - Any of the provided signers in `keypairs` is not a required signer of
    ///   the message. The error is [`SignerError::KeypairPubkeyMismatch`].
    /// - Any of the signers is a [`Presigner`], and its provided signature is
    ///   incorrect. The error is [`SignerError::PresignerError`] where the
    ///   interior [`PresignerError`] is
    ///   [`PresignerError::VerificationFailure`].
    /// - The signer is a [`RemoteKeypair`] and
    ///   - It does not understand the input provided ([`SignerError::InvalidInput`]).
    ///   - The device cannot be found ([`SignerError::NoDeviceFound`]).
    ///   - The user cancels the signing ([`SignerError::UserCancel`]).
    ///   - An error was encountered connecting ([`SignerError::Connection`]).
    ///   - Some device-specific protocol error occurs ([`SignerError::Protocol`]).
    ///   - Some other error occurs ([`SignerError::Custom`]).
    fn sign(
        _caller_uuid: &ConstructDid,
        _title: &str,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> SignerSignFutureResult {
        let mut result = CommandExecutionResult::new();

        let secret_key_bytes = signer_state
            .get_expected_buffer_bytes(SECRET_KEY)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

        let keypair = Keypair::from_bytes(&secret_key_bytes).unwrap();
        // let deploy_keypair = if let Ok(deploy_keypair) = signer_state
        //     .get_expected_scoped_buffer_bytes(&_caller_uuid.to_string(), PROGRAM_DEPLOYMENT_KEYPAIR)
        // {
        //     let deploy_keypair = Keypair::from_bytes(&deploy_keypair).unwrap();

        //     println!("deploy keypair: {:?}", deploy_keypair.pubkey());
        //     Some(deploy_keypair)
        // } else {
        //     None
        // };

        let rpc_api_url = values
            .get_expected_string(RPC_API_URL)
            .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?
            .to_string();

        if values.get_bool(IS_DEPLOYMENT).unwrap_or(false) {
            let commitment = match values.get_string(COMMITMENT_LEVEL).unwrap() {
                "finalized" => CommitmentLevel::Finalized,
                "processed" => CommitmentLevel::Processed,
                "confirmed" => CommitmentLevel::Confirmed,
                _ => CommitmentLevel::Processed,
            };
            let do_await_confirmation = values.get_bool(DO_AWAIT_CONFIRMATION).unwrap_or(false);

            let rpc_client = Arc::new(RpcClient::new_with_commitment(
                rpc_api_url.clone(),
                CommitmentConfig { commitment },
            ));

            let (transaction_bytes, available_keypair_bytes) =
                SvmValue::parse_transaction_with_keypairs(payload).map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!(
                            "failed to deserialize transaction with keypairs for signing: {e}"
                        ),
                    )
                })?;

            let mut transaction: Transaction =
                serde_json::from_slice(&transaction_bytes).map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("failed to deserialize transaction for signing: {e}"),
                    )
                })?;

            let blockhash = rpc_client.get_latest_blockhash().map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("failed to get latest blockhash: {e}"),
                )
            })?;

            transaction.message.recent_blockhash = blockhash;
            let mut keypairs: Vec<&Keypair> = vec![];
            let mut owned_keypairs: Vec<Keypair> = vec![];

            for keypair_bytes in available_keypair_bytes.iter() {
                let kp = Keypair::from_bytes(keypair_bytes).unwrap();
                owned_keypairs.push(kp);
            }

            for kp in owned_keypairs.iter() {
                keypairs.push(kp);
            }

            if payload.expect_addon_data().id.ne(SVM_TEMP_AUTHORITY_SIGNED_TRANSACTION) {
                keypairs.push(&keypair);
            }

            transaction.try_sign(&keypairs, transaction.message.recent_blockhash).map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("failed to sign transaction: {e}"),
                )
            })?;
            let _ = transaction.verify_and_hash_message().map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("failed to verify signed transaction: {}", e),
                )
            })?;
            let transaction_bytes = serde_json::to_vec(&transaction).map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("failed to serialize signed transaction: {e}"),
                )
            })?;

            let signature =
                send_transaction(rpc_client.clone(), do_await_confirmation, &transaction_bytes)
                    .map_err(|e| {
                        (
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("failed to send transaction: {e}"),
                        )
                    })?;
            result.outputs.insert(
                SIGNED_TRANSACTION_BYTES.into(),
                SvmValue::transaction(&transaction).map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("invalid signed transaction: {e}"),
                    )
                })?,
            );
            result.outputs.insert(SIGNATURE.into(), Value::string(signature));
        } else {
            let mut transaction: Transaction = SvmValue::to_transaction(&payload)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

            transaction
                .try_partial_sign(&[keypair], transaction.message.recent_blockhash)
                .map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("failed to sign transaction: {e}"),
                    )
                })?;

            result.outputs.insert(
                PARTIALLY_SIGNED_TRANSACTION_BYTES.into(),
                SvmValue::transaction(&transaction)
                    .map_err(|e| (signers.clone(), signer_state.clone(), e))?,
            );
        };

        return_synchronous_result(Ok((signers, signer_state, result)))
    }
}
