use alloy::consensus::TypedTransaction;
use alloy::network::{EthereumWallet, TransactionBuilder};
use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder, WalletProvider};
use alloy::rpc::types::{Transaction, TransactionReceipt, TransactionRequest};
use alloy::signers::k256::ecdsa::SigningKey;
use alloy_signer_local::coins_bip39::English;
use alloy_signer_local::{LocalSigner, MnemonicBuilder};
use hmac::Hmac;
use libsecp256k1::{PublicKey, SecretKey};
use pbkdf2::pbkdf2;
use serde::Deserialize;
use sha2::Sha512;
use std::collections::HashMap;
use tiny_hderive::bip32::ExtendedPrivKey;
use txtx_addon_kit::reqwest::Url;
use txtx_addon_kit::types::commands::{CommandExecutionContext, CommandExecutionResult};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, ReviewInputRequest,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::wallets::{
    return_synchronous_result, CheckSignabilityOk, WalletActionErr, WalletActionsFutureResult,
    WalletActivateFutureResult, WalletInstance, WalletSignFutureResult, WalletsState,
};
use txtx_addon_kit::types::wallets::{WalletImplementation, WalletSpecification};
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::{channel, AddonDefaults};

use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, MESSAGE_BYTES, RPC_API_URL, SIGNED_MESSAGE_BYTES,
};
use crate::typing::ETH_TX_HASH;
use txtx_addon_kit::types::wallets::return_synchronous_actions;

use crate::constants::{NETWORK_ID, PUBLIC_KEYS, SIGNED_TRANSACTION_BYTES};

use super::DEFAULT_DERIVATION_PATH;

lazy_static! {
    pub static ref EVM_MNEMONIC: WalletSpecification = define_wallet! {
        EVMMnemonic => {
          name: "EVM Mnemonic Wallet",
          matcher: "mnemonic",
          documentation:txtx_addon_kit::indoc! {r#"The `evm::mnemonic` wallet can be used to synchronously sign a transaction."#},
          inputs: [
            mnemonic: {
                documentation: "The mnemonic phrase used to generate the secret key.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            derivation_path: {
                documentation: "The derivation path used to generate the secret key.",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            is_encrypted: {
                documentation: "Coming soon",
                typing: Type::bool(),
                optional: true,
                interpolable: true
            },
            password: {
                documentation: "Coming soon",
                typing: Type::string(),
                optional: true,
                interpolable: true
            }
          ],
          outputs: [
              public_key: {
                documentation: "The public key of the account generated from the secret key.",
                typing: Type::array(Type::buffer())
              },
              tx_hash: {
                documentation: "The hash of the broadcasted transaction",
                typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
        wallet "bob" "evm::mnemonic" {
            mnemonic = "board list obtain sugar hour worth raven scout denial thunder horse logic fury scorpion fold genuine phrase wealth news aim below celery when cabin"
            derivation_path = "m/44'/5757'/0'/0/0"
        }
    "#},
      }
    };
}

pub struct EVMMnemonic;
impl WalletImplementation for EVMMnemonic {
    fn check_instantiability(
        _ctx: &WalletSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    #[cfg(not(feature = "wasm"))]
    fn check_activability(
        _uuid: &ConstructUuid,
        instance_name: &str,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut wallet_state: ValueStore,
        wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        execution_context: &CommandExecutionContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> WalletActionsFutureResult {
        let mut actions = Actions::none();

        if wallet_state.get_value(PUBLIC_KEYS).is_some() {
            return return_synchronous_actions(Ok((wallets, wallet_state, actions)));
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

        wallet_state.insert("mnemonic", mnemonic);
        wallet_state.insert("derivation_path", derivation_path);
        wallet_state.insert("is_encrypted", is_encrypted);

        let expected_wallet = match get_mnemonic_wallet(args, defaults, &wallet_state) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, wallet_state, diag)),
        };
        let expected_address: Address = expected_wallet.address();
        wallet_state.insert(
            "signer_address",
            Value::string(expected_address.to_string()),
        );
        // wallet_state.insert(PUBLIC_KEYS, Value::array(vec![public_key.clone()]));
        // wallet_state.insert(CHECKED_PUBLIC_KEY, Value::array(vec![public_key.clone()]));

        if execution_context.review_input_values {
            actions.push_sub_group(vec![ActionItemRequest::new(
                &None,
                &format!("Check {} expected address", instance_name),
                None,
                ActionItemStatus::Todo,
                ActionItemRequestType::ReviewInput(ReviewInputRequest {
                    input_name: "".into(),
                    value: Value::string(expected_address.to_string()),
                }),
                ACTION_ITEM_CHECK_ADDRESS,
            )]);
        }
        let future = async move { Ok((wallets, wallet_state, actions)) };
        Ok(Box::pin(future))
    }

    fn activate(
        _uuid: &ConstructUuid,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        wallet_state: ValueStore,
        wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult {
        let result = CommandExecutionResult::new();
        return_synchronous_result(Ok((wallets, wallet_state, result)))
    }

    fn check_signability(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _description: &Option<String>,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        wallet_state: ValueStore,
        wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<CheckSignabilityOk, WalletActionErr> {
        Ok((wallets, wallet_state, Actions::none()))
    }

    fn sign(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        payload: &Value,
        _spec: &WalletSpecification,
        args: &ValueStore,
        wallet_state: ValueStore,
        wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
    ) -> WalletSignFutureResult {
        let args = args.clone();
        let payload = payload.clone();
        let defaults = defaults.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            let rpc_api_url = args.get_defaulting_string(RPC_API_URL, &defaults).unwrap();
            let mnemonic_signer = match get_mnemonic_wallet(&args, &defaults, &wallet_state) {
                Ok(value) => value,
                Err(diag) => return Err((wallets, wallet_state, diag)),
            };
            let eth_wallet = EthereumWallet::from(mnemonic_signer);

            let payload_buffer = payload.expect_buffer_data();

            let mut tx: TransactionRequest = serde_json::from_slice(&payload_buffer.bytes).unwrap();
            // there's no to address on the tx, which is invalid unless it's set as "create"
            tx.set_create();

            let tx_envelope = tx.build(&eth_wallet).await.unwrap();

            let url = Url::try_from(rpc_api_url.as_ref()).unwrap();
            let provider = ProviderBuilder::new()
                .with_recommended_fillers()
                .wallet(eth_wallet)
                .on_http(url);
            let pending_tx = provider.send_tx_envelope(tx_envelope).await.unwrap();
            let tx_hash = pending_tx.tx_hash().0;
            result.outputs.insert(
                "tx_hash".to_string(),
                Value::buffer(tx_hash.to_vec(), ETH_TX_HASH.clone()),
            );
            Ok((wallets, wallet_state, result))
        };
        Ok(Box::pin(future))
    }
}

type MnemonicSigner = LocalSigner<SigningKey>;
pub fn get_mnemonic_wallet(
    args: &ValueStore,
    defaults: &AddonDefaults,
    wallet_state: &ValueStore,
) -> Result<MnemonicSigner, Diagnostic> {
    let mnemonic = wallet_state.get_expected_string("mnemonic")?;
    let derivation_path = wallet_state
        .get_expected_string("derivation_path")
        .unwrap_or(DEFAULT_DERIVATION_PATH);

    let mut mnemonic_builder = MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .derivation_path(derivation_path)
        .unwrap();

    if let Some(password) = wallet_state.get_string("password") {
        mnemonic_builder = mnemonic_builder.password(password)
    }
    let wallet = mnemonic_builder.build().unwrap();
    Ok(wallet)
}

pub fn get_bip39_seed_from_mnemonic(mnemonic: &str, password: &str) -> Result<Vec<u8>, String> {
    const PBKDF2_ROUNDS: u32 = 2048;
    const PBKDF2_BYTES: usize = 64;
    let salt = format!("mnemonic{}", password);
    let mut seed = vec![0u8; PBKDF2_BYTES];

    pbkdf2::<Hmac<Sha512>>(
        mnemonic.as_bytes(),
        salt.as_bytes(),
        PBKDF2_ROUNDS,
        &mut seed,
    )
    .map_err(|e| e.to_string())?;
    Ok(seed)
}
