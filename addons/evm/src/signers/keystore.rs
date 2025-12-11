use alloy::primitives::Address;
use std::collections::HashMap;
use txtx_addon_kit::channel;
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
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
use crate::constants::{CHECKED_ADDRESS, KEYSTORE_ACCOUNT, KEYSTORE_PATH, PASSWORD, PUBLIC_KEYS};
use crate::typing::EvmValue;
use txtx_addon_kit::types::signers::return_synchronous_actions;

lazy_static! {
    pub static ref EVM_KEYSTORE_SIGNER: SignerSpecification = define_signer! {
        EvmKeystoreSigner => {
          name: "EVM Keystore Signer",
          matcher: "keystore",
          documentation: txtx_addon_kit::indoc! {r#"
The `evm::keystore` signer uses a Foundry-compatible encrypted keystore file to sign transactions.

**Note:** This signer is only supported in unsupervised mode (`--unsupervised` flag). For supervised/interactive mode, use `evm::web_wallet` instead.

### Inputs

| Name               | Type   | Required | Description                                                                 |
|--------------------|--------|----------|-----------------------------------------------------------------------------|
| `keystore_account` | string | Yes      | The account name (filename without .json) or full path to the keystore file |
| `keystore_path`    | string | No       | Directory containing keystores. Defaults to `~/.foundry/keystores`          |

### Outputs

| Name      | Type   | Description                              |
|-----------|--------|------------------------------------------|
| `address` | string | The Ethereum address derived from the keystore |

### Password

The keystore password is prompted interactively at CLI startup (Foundry-style UX). The password is never stored in the manifest or on disk.

### Security

- Compatible with keystores created by `cast wallet import`
- Password is prompted securely (hidden input) before execution begins
- Password is held only in memory during execution
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
            },
            password: {
                documentation: "Internal use only - populated by CLI interactive prompt.",
                typing: Type::string(),
                optional: true,
                tainting: false,
                sensitive: true
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
        _construct_did: &ConstructDid,
        _instance_name: &str,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
        _auth_ctx: &AuthorizationContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        // Keystore signer only supports unsupervised mode - for supervised mode use web_wallet
        if supervision_context.is_supervised {
            return Err((
                signers,
                signer_state,
                diagnosed_error!(
                    "evm::keystore signer is only supported in unsupervised mode. \
                    For supervised mode, use evm::web_wallet instead."
                ),
            ));
        }

        let mut actions = Actions::none();

        if signer_state.get_value(PUBLIC_KEYS).is_some() {
            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        }

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

        // Password is injected by CLI before execution (interactive prompt)
        // Password is never stored in the manifest
        let password = values.get_string(PASSWORD).ok_or_else(|| {
            (
                signers.clone(),
                signer_state.clone(),
                diagnosed_error!(
                    "keystore password not provided. This should not happen in unsupervised mode - \
                    the password should be prompted interactively before execution."
                ),
            )
        })?;

        // Password available - decrypt keystore
        let signer = keystore_to_secret_key_signer(&resolved_path, password)
            .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

        let expected_address: Address = signer.address();

        signer_state.insert(
            "signer_field_bytes",
            EvmValue::signer_field_bytes(signer.to_field_bytes().to_vec()),
        );
        signer_state.insert("signer_address", Value::string(expected_address.to_string()));

        // In unsupervised mode, we don't need interactive address review
        signer_state.insert(CHECKED_ADDRESS, Value::string(expected_address.to_string()));

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
