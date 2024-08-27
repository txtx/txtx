use alloy::consensus::Transaction;
use alloy::network::{EthereumWallet, TransactionBuilder};
use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::TransactionRequest;
use alloy::signers::k256::ecdsa::SigningKey;
use alloy_signer_local::coins_bip39::English;
use alloy_signer_local::{LocalSigner, MnemonicBuilder};
use std::collections::HashMap;
use txtx_addon_kit::reqwest::Url;
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, ReviewInputRequest,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::signers::{
    return_synchronous_result, CheckSignabilityOk, SignerActionErr, SignerActionsFutureResult,
    SignerActivateFutureResult, SignerInstance, SignerSignFutureResult,
};
use txtx_addon_kit::types::signers::{SignerImplementation, SignerSpecification};
use txtx_addon_kit::types::ValueStore;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{
    signers::SignersState, types::RunbookSupervisionContext, ConstructDid,
};
use txtx_addon_kit::{channel, AddonDefaults};

use crate::constants::{ACTION_ITEM_CHECK_ADDRESS, NONCE, RPC_API_URL, TX_HASH};
use crate::typing::EvmValue;
use txtx_addon_kit::types::signers::return_synchronous_actions;

use crate::constants::PUBLIC_KEYS;

use super::DEFAULT_DERIVATION_PATH;

lazy_static! {
    pub static ref EVM_MNEMONIC: SignerSpecification = define_signer! {
        EVMMnemonic => {
          name: "EVM Mnemonic Signer",
          matcher: "mnemonic",
          documentation:txtx_addon_kit::indoc! {r#"The `evm::mnemonic` signer can be used to synchronously sign a transaction."#},
          inputs: [
            mnemonic: {
                documentation: "The mnemonic phrase used to generate the secret key.",
                typing: Type::string(),
                optional: false,
                interpolable: true,
                sensitive: true
            },
            derivation_path: {
                documentation: "The derivation path used to generate the secret key.",
                typing: Type::string(),
                optional: true,
                interpolable: true,
                sensitive: true
            },
            is_encrypted: {
                documentation: "Coming soon",
                typing: Type::bool(),
                optional: true,
                interpolable: true,
                sensitive: false
            },
            password: {
                documentation: "Coming soon",
                typing: Type::string(),
                optional: true,
                interpolable: true,
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
        signer "bob" "evm::mnemonic" {
            mnemonic = "board list obtain sugar hour worth raven scout denial thunder horse logic fury scorpion fold genuine phrase wealth news aim below celery when cabin"
            derivation_path = "m/44'/5757'/0'/0/0"
        }
    "#},
      }
    };
}

pub struct EVMMnemonic;
impl SignerImplementation for EVMMnemonic {
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
        args: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        let mut actions = Actions::none();

        if signer_state.get_value(PUBLIC_KEYS).is_some() {
            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        }

        let mnemonic = args.get_expected_value("mnemonic").unwrap().clone();
        let derivation_path = match args.get_value("derivation_path") {
            Some(v) => v.clone(),
            None => Value::string(DEFAULT_DERIVATION_PATH.into()),
        };
        let is_encrypted = match args.get_value("is_encrypted") {
            Some(v) => v.clone(),
            None => Value::bool(false),
        };
        // let network_id = args.get_defaulting_string(NETWORK_ID, defaults).unwrap();

        signer_state.insert("mnemonic", mnemonic);
        signer_state.insert("derivation_path", derivation_path);
        signer_state.insert("is_encrypted", is_encrypted);

        let expected_signer = match get_mnemonic_signer(args, defaults, &signer_state) {
            Ok(value) => value,
            Err(diag) => return Err((signers, signer_state, diag)),
        };
        let expected_address: Address = expected_signer.address();
        signer_state.insert("signer_address", Value::string(expected_address.to_string()));
        // signer_state.insert(PUBLIC_KEYS, Value::array(vec![public_key.clone()]));
        // signer_state.insert(CHECKED_PUBLIC_KEY, Value::array(vec![public_key.clone()]));

        if supervision_context.review_input_values {
            actions.push_sub_group(
                None,
                vec![ActionItemRequest::new(
                    &None,
                    &format!("Check {} expected address", instance_name),
                    None,
                    ActionItemStatus::Todo,
                    ActionItemRequestType::ReviewInput(ReviewInputRequest {
                        input_name: "".into(),
                        value: Value::string(expected_address.to_string()),
                    }),
                    ACTION_ITEM_CHECK_ADDRESS,
                )],
            );
        }
        let future = async move { Ok((signers, signer_state, actions)) };
        Ok(Box::pin(future))
    }

    fn activate(
        _construct_id: &ConstructDid,
        _spec: &SignerSpecification,
        _args: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        let mut result = CommandExecutionResult::new();
        let address = signer_state
            .get_expected_value("signer_address")
            .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;
        result.outputs.insert("address".into(), address.clone());
        return_synchronous_result(Ok((signers, signer_state, result)))
    }

    fn check_signability(
        _caller_uuid: &ConstructDid,
        _title: &str,
        _description: &Option<String>,
        _payload: &Value,
        _spec: &SignerSpecification,
        _args: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        Ok((signers, signer_state, Actions::none()))
    }

    fn sign(
        _caller_uuid: &ConstructDid,
        _title: &str,
        payload: &Value,
        _spec: &SignerSpecification,
        args: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        defaults: &AddonDefaults,
    ) -> SignerSignFutureResult {
        let args = args.clone();
        let payload = payload.clone();
        let defaults = defaults.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            let rpc_api_url = args.get_defaulting_string(RPC_API_URL, &defaults).unwrap();
            let mnemonic_signer = match get_mnemonic_signer(&args, &defaults, &signer_state) {
                Ok(value) => value,
                Err(diag) => return Err((signers, signer_state, diag)),
            };
            let eth_signer = EthereumWallet::from(mnemonic_signer);

            let payload_bytes = payload.expect_buffer_bytes();

            let mut tx: TransactionRequest = serde_json::from_slice(&payload_bytes).unwrap();
            if tx.to.is_none() {
                // there's no to address on the tx, which is invalid unless it's set as "create"
                tx.set_create();
            }

            let tx_envelope = tx.build(&eth_signer).await.unwrap();
            let tx_nonce = tx_envelope.nonce();

            let url = Url::try_from(rpc_api_url.as_ref()).unwrap();
            let provider =
                ProviderBuilder::new().with_recommended_fillers().wallet(eth_signer).on_http(url);
            let pending_tx = provider.send_tx_envelope(tx_envelope).await.map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("error signing transaction: {e}"),
                )
            })?;
            let tx_hash = pending_tx.tx_hash().0;
            result.outputs.insert(TX_HASH.to_string(), EvmValue::tx_hash(tx_hash.to_vec()));
            signer_state.insert(NONCE, Value::integer(tx_nonce.into()));
            Ok((signers, signer_state, result))
        };
        Ok(Box::pin(future))
    }
}

type MnemonicSigner = LocalSigner<SigningKey>;
pub fn get_mnemonic_signer(
    _args: &ValueStore,
    _defaults: &AddonDefaults,
    signer_state: &ValueStore,
) -> Result<MnemonicSigner, Diagnostic> {
    let mnemonic = signer_state.get_expected_string("mnemonic")?;
    let derivation_path =
        signer_state.get_expected_string("derivation_path").unwrap_or(DEFAULT_DERIVATION_PATH);

    let mut mnemonic_builder = MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .derivation_path(derivation_path)
        .unwrap();

    if let Some(password) = signer_state.get_string("password") {
        mnemonic_builder = mnemonic_builder.password(password)
    }
    let signer = mnemonic_builder.build().unwrap();
    Ok(signer)
}
