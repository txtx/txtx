use alloy::network::{EthereumWallet, TransactionBuilder};
use alloy::primitives::Address;
use alloy_rpc_types::TransactionRequest;
use std::collections::HashMap;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::SIGNATURE_APPROVED;
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemRequestType, ActionItemStatus, ProvideInputRequest,
    ProvideSignedTransactionRequest, ReviewInputRequest,
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

use crate::codec::crypto::{field_bytes_to_secret_key_signer, keystore_to_secret_key_signer, resolve_keystore_path};
use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_PROVIDE_KEYSTORE_PASSWORD,
    ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION, CHAIN_ID, FORMATTED_TRANSACTION,
    KEYSTORE_ACCOUNT, KEYSTORE_PASSWORD, KEYSTORE_PATH, NAMESPACE, RPC_API_URL,
    SECRET_KEY_WALLET_UNSIGNED_TRANSACTION_BYTES, TX_HASH,
};
use crate::rpc::EvmWalletRpc;
use crate::typing::EvmValue;
use txtx_addon_kit::types::signers::return_synchronous_actions;

use crate::constants::PUBLIC_KEYS;

lazy_static! {
    pub static ref EVM_KEYSTORE_SIGNER: SignerSpecification = define_signer! {
        EvmKeystoreSigner => {
          name: "EVM Keystore Signer",
          matcher: "keystore",
          documentation: txtx_addon_kit::indoc! {r#"
The `evm::keystore` signer uses a Foundry-compatible encrypted keystore file to sign transactions.
The keystore password will be prompted at runtime for security.

### Inputs

| Name               | Type   | Required | Description                                                                 |
|--------------------|--------|----------|-----------------------------------------------------------------------------|
| `keystore_account` | string | Yes      | The account name (filename without .json) or full path to the keystore file |
| `keystore_path`    | string | No       | Directory containing keystores. Defaults to `~/.foundry/keystores`          |

### Outputs

| Name      | Type   | Description                              |
|-----------|--------|------------------------------------------|
| `address` | string | The Ethereum address derived from the keystore |

### Security

- The keystore password is never stored in manifests
- Password is prompted interactively at runtime
- Compatible with keystores created by `cast wallet import`
"#},
          inputs: [
            keystore_account: {
                documentation: "The account name (filename without .json) in the keystores directory, or a full path to the keystore file.",
                typing: Type::string(),
                optional: false,
                tainting: true,
                sensitive: false
            },
            keystore_path: {
                documentation: "The directory containing keystore files. Defaults to ~/.foundry/keystores",
                typing: Type::string(),
                optional: true,
                tainting: true,
                sensitive: false
            }
          ],
          outputs: [
              address: {
                documentation: "The address derived from the keystore.",
                typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
            // Use a keystore from the default Foundry keystores directory
            signer "deployer" "evm::keystore" {
                keystore_account = "my-deployer-account"
            }

            // Use a keystore from a custom directory
            signer "deployer" "evm::keystore" {
                keystore_account = "deployer"
                keystore_path = "./keystores"
            }

            // Use a full path to a keystore file
            signer "deployer" "evm::keystore" {
                keystore_account = "/path/to/my-keystore.json"
            }
        "#}
      }
    };
}

pub struct EvmKeystoreSigner;

impl SignerImplementation for EvmKeystoreSigner {
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
        use txtx_addon_kit::constants::DESCRIPTION;
        use crate::constants::CHECKED_ADDRESS;

        let mut actions = Actions::none();

        // If we already have the signer set up, we're done
        if signer_state.get_value(PUBLIC_KEYS).is_some() {
            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        }

        let description = values.get_string(DESCRIPTION).map(|d| d.to_string());
        let markdown = values
            .get_markdown(auth_ctx)
            .map_err(|d| (signers.clone(), signer_state.clone(), d))?;

        // Get keystore configuration
        let keystore_account = values
            .get_expected_string(KEYSTORE_ACCOUNT)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
        let keystore_path = values.get_string(KEYSTORE_PATH);

        // Resolve the keystore file path
        let resolved_path = resolve_keystore_path(keystore_account, keystore_path)
            .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

        // Store the resolved path for later use
        signer_state.insert(
            "resolved_keystore_path",
            Value::string(resolved_path.to_string_lossy().to_string()),
        );

        // Check if we have the password yet (from a previous response)
        let password = signer_state.get_string(KEYSTORE_PASSWORD);

        if let Some(password) = password {
            // We have the password, decrypt the keystore
            let signer = keystore_to_secret_key_signer(&resolved_path, password)
                .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

            let expected_address: Address = signer.address();

            // Store the signer field bytes for later signing
            signer_state.insert(
                "signer_field_bytes",
                EvmValue::signer_field_bytes(signer.to_field_bytes().to_vec()),
            );
            signer_state.insert("signer_address", Value::string(expected_address.to_string()));

            if supervision_context.review_input_values {
                if signer_state.get_expected_string(CHECKED_ADDRESS).is_ok() {
                    signer_state.insert(CHECKED_ADDRESS, Value::string(expected_address.to_string()));
                } else {
                    // Ask user to review/confirm the address
                    actions.push_sub_group(
                        None,
                        vec![ReviewInputRequest::new("", &Value::string(expected_address.to_string()))
                            .to_action_type()
                            .to_request(instance_name, ACTION_ITEM_CHECK_ADDRESS)
                            .with_construct_did(construct_did)
                            .with_some_description(description.clone())
                            .with_meta_description(&format!("Check {} expected address", instance_name))
                            .with_some_markdown(markdown.clone())],
                    );
                }
            } else {
                signer_state.insert(CHECKED_ADDRESS, Value::string(expected_address.to_string()));
            }
        } else {
            // No password yet - prompt for it
            let keystore_display = resolved_path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| keystore_account.to_string());

            let request = ProvideInputRequest {
                default_value: None,
                input_name: KEYSTORE_PASSWORD.to_string(),
                typing: Type::string(),
            };

            actions.push_sub_group(
                Some(format!("Enter password for keystore '{}'", keystore_display)),
                vec![ActionItemRequestType::ProvideInput(request)
                    .to_request(instance_name, ACTION_ITEM_PROVIDE_KEYSTORE_PASSWORD)
                    .with_construct_did(construct_did)
                    .with_some_description(description.clone())
                    .with_meta_description(&format!("Unlock keystore for {}", instance_name))
                    .with_some_markdown(markdown.clone())
                    .with_status(ActionItemStatus::Todo)],
            );
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
            if signer_state.get_scoped_value(construct_did_str, SIGNATURE_APPROVED).is_some() {
                return Ok((signers, signer_state, Actions::none()));
            }

            let chain_id = values
                .get_expected_uint(CHAIN_ID)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

            let status = ActionItemStatus::Todo;

            let formatted_payload =
                signer_state.get_scoped_value(construct_did_str, FORMATTED_TRANSACTION);

            let request = ProvideSignedTransactionRequest::new(
                &signer_state.uuid,
                payload,
                NAMESPACE,
                &chain_id.to_string(),
            )
            .check_expectation_action_uuid(construct_did)
            .only_approval_needed()
            .formatted_payload(formatted_payload)
            .to_action_type()
            .to_request(title, ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION)
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
        signer_state: ValueStore,
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
                tx.set_create();
            }

            let tx_envelope = tx.build(&eth_signer).await.map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("failed to build transaction envelope: {e}"),
                )
            })?;

            let rpc = EvmWalletRpc::new(&rpc_api_url, eth_signer)
                .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

            let tx_hash = rpc.sign_and_send_tx(tx_envelope).await.map_err(|e| {
                (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e.to_string()))
            })?;

            result.outputs.insert(TX_HASH.to_string(), EvmValue::tx_hash(tx_hash.to_vec()));

            Ok((signers, signer_state, result))
        };
        Ok(Box::pin(future))
    }
}
