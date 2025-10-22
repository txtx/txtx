use std::collections::HashMap;

use solana_client::rpc_client::RpcClient;
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_keypair::Keypair;
use solana_transaction::Transaction;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::{SignerKey};
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemStatus, ProvideSignedTransactionRequest, ReviewInputRequest,
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
use txtx_addon_network_svm_types::SvmValue;

use crate::codec::DeploymentTransaction;
use txtx_addon_kit::constants::ActionItemKey;
use crate::constants::{
    ADDRESS, CHECKED_ADDRESS,
    CHECKED_PUBLIC_KEY, COMMITMENT_LEVEL, FORMATTED_TRANSACTION, IS_DEPLOYMENT, IS_SIGNABLE,
    NAMESPACE, NETWORK_ID, PARTIALLY_SIGNED_TRANSACTION_BYTES, PREVIOUSLY_SIGNED_BLOCKHASH,
    PUBLIC_KEY, RPC_API_URL, SECRET_KEY, TRANSACTION_BYTES,
};
use crate::utils::build_transaction_from_svm_value;
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
            "#}
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
        construct_did: &ConstructDid,
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
        use std::path::PathBuf;

        use crate::constants::{
            DEFAULT_DERIVATION_PATH, DERIVATION_PATH, IS_ENCRYPTED, KEYPAIR_JSON, MNEMONIC,
            PASSWORD, REQUESTED_STARTUP_DATA, SECRET_KEY,
        };
        use solana_keypair::Keypair;
        use solana_signer::Signer;
        use txtx_addon_kit::{constants::DocumentationKey, crypto::secret_key_bytes_from_mnemonic};
        let mut actions = Actions::none();

        if signer_state.get_value(CHECKED_PUBLIC_KEY).is_some() {
            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        }

        let description = values.get_string(DocumentationKey::Description.as_ref()).map(|d| d.to_string());
        let markdown = values
            .get_markdown(auth_ctx)
            .map_err(|d| (signers.clone(), signer_state.clone(), d))?;

        let secret_key_bytes = match values.get_value(SECRET_KEY) {
            None => match values.get_string(MNEMONIC) {
                None => {
                    let keypair_json_str = values
                        .get_expected_string(KEYPAIR_JSON)
                        .map_err(|_| (signers.clone(), signer_state.clone(), diagnosed_error!("either `secret_key`, `mnemonic`, or `keypair_json` fields are required")))?;

                    let keypair_json = auth_ctx
                        .get_file_location_from_path_buf(&PathBuf::from(keypair_json_str))
                        .map_err(|e| {
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

        let keypair = Keypair::try_from(secret_key_bytes.as_ref()).map_err(|e| {
            (signers.clone(), signer_state.clone(), diagnosed_error!("invalid secret key: {e}"))
        })?;

        let public_key_value = Value::string(keypair.pubkey().to_string());
        let secret_key = Value::buffer(secret_key_bytes);

        if supervision_context.review_input_values {
            signer_state.insert(&REQUESTED_STARTUP_DATA, Value::bool(true));
            if let Ok(_) = signer_state.get_expected_string(CHECKED_ADDRESS) {
                signer_state.insert(CHECKED_PUBLIC_KEY, public_key_value.clone());
                signer_state.insert(CHECKED_ADDRESS, public_key_value.clone());
                signer_state.insert(SECRET_KEY, secret_key);
            } else {
                actions.push_sub_group(
                    None,
                    vec![ReviewInputRequest::new("", &public_key_value)
                        .to_action_type()
                        .to_request(instance_name, ActionItemKey::CheckAddress)
                        .with_construct_did(construct_did)
                        .with_some_description(description)
                        .with_meta_description(&format!("Check {} expected address", instance_name))
                        .with_some_markdown(markdown)],
                );
            }
        } else {
            signer_state.insert(CHECKED_PUBLIC_KEY, public_key_value.clone());
            signer_state.insert(CHECKED_ADDRESS, public_key_value.clone());
            signer_state.insert(SECRET_KEY, secret_key);
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
        meta_description: &Option<String>,
        markdown: &Option<String>,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
        _auth_ctx: &AuthorizationContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            TRANSACTION_BYTES,
            payload.clone(),
        );

        let actions = if supervision_context.review_input_values {
            let construct_did_str = &construct_did.to_string();
            if let Some(_) = signer_state.get_scoped_value(&construct_did_str, SignerKey::SignatureApproved.as_ref()) {
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
                .get_scoped_value(&construct_did_str, SignerKey::SignatureSkippable.as_ref())
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let formatted_payload =
                signer_state.get_scoped_value(&construct_did_str, FORMATTED_TRANSACTION);

            let request = ProvideSignedTransactionRequest::new(
                &signer_state.uuid,
                &payload,
                NAMESPACE,
                &network_id,
            )
            .skippable(skippable)
            .check_expectation_action_uuid(construct_did)
            .formatted_payload(formatted_payload)
            .only_approval_needed()
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
        construct_did: &ConstructDid,
        _title: &str,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> SignerSignFutureResult {
        let mut result = CommandExecutionResult::new();

        let secret_key_bytes = signer_state
            .get_expected_buffer_bytes(SECRET_KEY)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

        let keypair = Keypair::try_from(secret_key_bytes.as_ref()).unwrap();

        // value signed (partially, maybe) by another signer
        let previously_signed_blockhash = signer_state
            .remove_scoped_value(&construct_did.to_string(), PREVIOUSLY_SIGNED_BLOCKHASH);

        // prevent discrepancies between new block hash and a hash on the transaction that's already been signed
        let blockhash = if let Some(blockhash) = &previously_signed_blockhash {
            solana_hash::Hash::new_from_array(blockhash.to_be_bytes().try_into().unwrap())
        } else {
            let rpc_api_url = values
                .get_expected_string(RPC_API_URL)
                .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?
                .to_string();

            let commitment = match values.get_string(COMMITMENT_LEVEL).unwrap_or("processed") {
                "finalized" => CommitmentLevel::Finalized,
                "processed" => CommitmentLevel::Processed,
                "confirmed" => CommitmentLevel::Confirmed,
                _ => CommitmentLevel::Processed,
            };
            let rpc_client = RpcClient::new_with_commitment(
                rpc_api_url.clone(),
                CommitmentConfig { commitment },
            );

            let blockhash = rpc_client.get_latest_blockhash().map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("failed to get latest blockhash: {e}"),
                )
            })?;
            blockhash
        };

        let is_deployment = values.get_bool(IS_DEPLOYMENT).unwrap_or(false);

        let (mut transaction, do_sign_with_txtx_signer) = if is_deployment {
            let deployment_transaction = DeploymentTransaction::from_value(&payload)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

            let mut transaction: Transaction =
                deployment_transaction.transaction.as_ref().unwrap().clone();

            transaction.message.recent_blockhash = blockhash;

            let keypairs = deployment_transaction
                .get_keypairs()
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

            transaction.try_partial_sign(&keypairs, transaction.message.recent_blockhash).map_err(
                |e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("failed to sign transaction: {e}"),
                    )
                },
            )?;

            (transaction, deployment_transaction.signers.is_some())
        } else {
            let mut transaction: Transaction = build_transaction_from_svm_value(&payload)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
            transaction.message.recent_blockhash = blockhash;

            (transaction, true)
        };

        if do_sign_with_txtx_signer {
            transaction
                .try_partial_sign(&[keypair], transaction.message.recent_blockhash)
                .map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("failed to sign transaction: {e}"),
                    )
                })?;
        }
        result.outputs.insert(
            PARTIALLY_SIGNED_TRANSACTION_BYTES.into(),
            SvmValue::transaction(&transaction)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?,
        );

        return_synchronous_result(Ok((signers, signer_state, result)))
    }
}
