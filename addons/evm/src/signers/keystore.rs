use alloy::primitives::Address;
use std::collections::HashMap;
use txtx_addon_kit::channel;
use txtx_addon_kit::types::frontend::{
    ActionItemRequestType, ActionItemStatus, Actions, BlockEvent, ProvideInputRequest,
    ReviewInputRequest,
};
use txtx_addon_kit::types::signers::{
    return_synchronous_result, CheckSignabilityOk, SignerActionErr, SignerActionsFutureResult,
    SignerActivateFutureResult, SignerImplementation, SignerInstance, SignerSignFutureResult,
    SignerSpecification, SignersState,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type, Value};
use txtx_addon_kit::types::commands::CommandSpecification;
use txtx_addon_kit::types::{diagnostics::Diagnostic, AuthorizationContext, ConstructDid};

use super::common::{activate_signer, check_signability, sign_transaction};
use crate::codec::crypto::{keystore_to_secret_key_signer, resolve_keystore_path};
use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_PROVIDE_KEYSTORE_PASSWORD, CHECKED_ADDRESS,
    KEYSTORE_ACCOUNT, KEYSTORE_PASSWORD, KEYSTORE_PATH, PUBLIC_KEYS,
};
use crate::typing::EvmValue;
use txtx_addon_kit::constants::DESCRIPTION;
use txtx_addon_kit::types::signers::return_synchronous_actions;

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
        auth_ctx: &AuthorizationContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        let mut actions = Actions::none();

        if signer_state.get_value(PUBLIC_KEYS).is_some() {
            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        }

        let description = values.get_string(DESCRIPTION).map(|d| d.to_string());
        let markdown = values
            .get_markdown(auth_ctx)
            .map_err(|d| (signers.clone(), signer_state.clone(), d))?;

        let keystore_account = values
            .get_expected_string(KEYSTORE_ACCOUNT)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
        let keystore_path = values.get_string(KEYSTORE_PATH);

        let resolved_path = resolve_keystore_path(keystore_account, keystore_path)
            .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

        signer_state.insert(
            "resolved_keystore_path",
            Value::string(resolved_path.to_string_lossy().into_owned()),
        );

        // If no password yet, prompt for it and return early
        let Some(password) = signer_state.get_string(KEYSTORE_PASSWORD) else {
            let keystore_display = resolved_path
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
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

            let future = async move { Ok((signers, signer_state, actions)) };
            return Ok(Box::pin(future));
        };

        // Password available - decrypt keystore
        let signer = keystore_to_secret_key_signer(&resolved_path, password)
            .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

        let expected_address: Address = signer.address();

        signer_state.insert(
            "signer_field_bytes",
            EvmValue::signer_field_bytes(signer.to_field_bytes().to_vec()),
        );
        signer_state.insert("signer_address", Value::string(expected_address.to_string()));

        // Handle address verification based on supervision context
        let needs_review = supervision_context.review_input_values
            && signer_state.get_expected_string(CHECKED_ADDRESS).is_err();

        if needs_review {
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
        } else {
            signer_state.insert(CHECKED_ADDRESS, Value::string(expected_address.to_string()));
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
        let (signers, signer_state, result) = activate_signer(signer_state, signers)?;
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
        check_signability(
            construct_did,
            title,
            description,
            meta_description,
            markdown,
            payload,
            values,
            signer_state,
            signers,
            supervision_context,
        )
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

        let future = async move { sign_transaction(&caller_uuid, &values, signer_state, signers).await };
        Ok(Box::pin(future))
    }
}
