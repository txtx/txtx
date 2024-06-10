use std::collections::HashMap;

use clarity::address::AddressHashMode;
use clarity::codec::StacksMessageCodec;
use clarity::types::chainstate::StacksAddress;
use clarity::util::secp256k1::{Secp256k1PrivateKey, Secp256k1PublicKey};
use clarity_repl::codec::{StacksTransaction, StacksTransactionSigner};
use hmac::Hmac;
use libsecp256k1::{PublicKey, SecretKey};
use pbkdf2::pbkdf2;
use sha2::Sha512;
use tiny_hderive::bip32::ExtendedPrivKey;
use txtx_addon_kit::types::commands::{CommandExecutionContext, CommandExecutionResult};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::wallets::{
    return_synchronous_result, WalletActionsFutureResult, WalletActivateFutureResult,
    WalletInstance, WalletSignFutureResult, WalletsState,
};
use txtx_addon_kit::types::wallets::{WalletImplementation, WalletSpecification};
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::{channel, AddonDefaults};
use txtx_addon_kit::{types::frontend::{ActionItemRequest, ActionItemRequestType, ActionItemStatus, ReviewInputRequest}, uuid::Uuid};

use crate::constants::ACTION_ITEM_CHECK_ADDRESS;
use txtx_addon_kit::types::wallets::return_synchronous_actions;


use crate::constants::{NETWORK_ID, PUBLIC_KEYS, SIGNED_TRANSACTION_BYTES};
use crate::typing::CLARITY_BUFFER;

use super::DEFAULT_DERIVATION_PATH;

lazy_static! {
    pub static ref STACKS_MNEMONIC: WalletSpecification = define_wallet! {
        StacksMnemonic => {
          name: "Mnemonic Wallet",
          matcher: "mnemonic",
          documentation: "Coming soon",
          inputs: [
            mnemonic: {
                documentation: "Coming soon",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            derivation_path: {
                documentation: "Coming soon",
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
                documentation: "Coming soon",
                typing: Type::array(Type::buffer())
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
        // Coming soon
    "#},
      }
    };
}

pub struct StacksMnemonic;
impl WalletImplementation for StacksMnemonic {
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
        mut wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        execution_context: &CommandExecutionContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> WalletActionsFutureResult {
        let mut actions = Actions::none();

        if wallet_state.get_value(PUBLIC_KEYS).is_some() {
            wallets.push_wallet_state(wallet_state);
            return return_synchronous_actions(Ok((wallets, actions)))
        }        

        println!("{instance_name} => {:?}", args);
        let mnemonic = args.get_expected_value("mnemonic").unwrap().clone();
        let derivation_path = match args.get_value("derivation_path") {
            Some(v) => v.clone(),
            None => Value::string(DEFAULT_DERIVATION_PATH.into())
        };
        let is_encrypted = match args.get_value("is_encrypted") {
            Some(v) => v.clone(),
            None => Value::bool(false)
        };
        let network_id = args.get_defaulting_string(NETWORK_ID, defaults).unwrap();
        let version = match network_id.as_str() {
            "mainnet" => AddressHashMode::SerializeP2PKH.to_version_mainnet(),
            _ => AddressHashMode::SerializeP2PKH.to_version_testnet(),
        };
        wallet_state.insert("mnemonic", mnemonic);
        wallet_state.insert("derivation_path", derivation_path);
        wallet_state.insert("is_encrypted", is_encrypted);
        wallet_state.insert("hash_flag", Value::uint(version.into()));
        wallet_state.insert("multi_sig", Value::bool(false));

        let (_, public_key, expected_address) = match compute_keypair(args, defaults, &wallet_state) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };
        wallet_state.insert(PUBLIC_KEYS, Value::array(vec![public_key.clone()]));        

        if execution_context.review_input_values {
            actions.push_sub_group(vec![ActionItemRequest::new(
                &Uuid::new_v4(),
                &None,
                &format!("Check {} expected address", instance_name),
                None,
                ActionItemStatus::Todo,
                ActionItemRequestType::ReviewInput(ReviewInputRequest {
                    input_name: "".into(),
                    value: Value::string(expected_address.to_string()),
                }),
                ACTION_ITEM_CHECK_ADDRESS,
            )])
        }
        let future = async move {
            wallets.push_wallet_state(wallet_state);
            Ok((wallets, actions))
        };
        Ok(Box::pin(future))
    }

    fn activate(
        _uuid: &ConstructUuid,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        wallet_state: ValueStore,
        mut wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult {
        let result = CommandExecutionResult::new();
        wallets.push_wallet_state(wallet_state);
        return_synchronous_result(Ok((wallets, result)))
    }

    fn check_signability(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _description: &Option<String>,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        wallet_state: ValueStore,
        mut wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)> {
        wallets.push_wallet_state(wallet_state);
        Ok((wallets, Actions::none()))
    }

    fn sign(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        payload: &Value,
        _spec: &WalletSpecification,
        args: &ValueStore,
        wallet_state: ValueStore,
        mut wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
    ) -> WalletSignFutureResult {
        println!("VALUES: {:?}", args);
        println!("WALLET: {:?}", wallet_state);
        let mut result = CommandExecutionResult::new();

        let (secret_key_value, _, _) = match compute_keypair(args, defaults, &wallet_state) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };

        let transaction_payload_bytes = payload.expect_buffer_bytes();
        let transaction =
            StacksTransaction::consensus_deserialize(&mut &transaction_payload_bytes[..]).unwrap();

        let mut tx_signer = StacksTransactionSigner::new(&transaction);
        let secret_key = Secp256k1PrivateKey::from_slice(&secret_key_value.to_bytes()).unwrap();
        tx_signer.sign_origin(&secret_key).unwrap();
        let signed_transaction = tx_signer.get_tx().unwrap();

        let mut signed_transaction_bytes = vec![];
        signed_transaction
            .consensus_serialize(&mut signed_transaction_bytes)
            .unwrap(); // todo
        let signed_transaction_bytes_value =
            Value::buffer(signed_transaction_bytes, CLARITY_BUFFER.clone());
        result.outputs.insert(
            SIGNED_TRANSACTION_BYTES.into(),
            signed_transaction_bytes_value,
        );

        wallets.push_wallet_state(wallet_state);

        return_synchronous_result(Ok((wallets, result)))
    }
}

pub fn compute_keypair(
    args: &ValueStore,
    defaults: &AddonDefaults,
    wallet_state: &ValueStore,
) -> Result<(Value, Value, StacksAddress), Diagnostic> {

    let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;
    let mnemonic = wallet_state.get_expected_string("mnemonic")?;
    let derivation_path = wallet_state
        .get_expected_string("derivation_path")
        .unwrap_or(DEFAULT_DERIVATION_PATH);
    let is_encrypted = wallet_state
        .get_expected_bool("is_encrypted")
        .unwrap_or(false);
    if is_encrypted {
        unimplemented!()
    }

    let bip39_seed = match get_bip39_seed_from_mnemonic(mnemonic, "") {
        Ok(bip39_seed) => bip39_seed,
        Err(_) => panic!(),
    };

    let ext = ExtendedPrivKey::derive(&bip39_seed[..], derivation_path).unwrap();
    let secret_key = SecretKey::parse_slice(&ext.secret()).unwrap();

    // Enforce a 33 bytes secret key format, expected by Stacks
    let mut secret_key_bytes = secret_key.serialize().to_vec();
    secret_key_bytes.push(1);
    let secret_key_hex = Value::buffer(secret_key_bytes, CLARITY_BUFFER.clone());

    let public_key = PublicKey::from_secret_key(&secret_key);
    let pub_key = Secp256k1PublicKey::from_slice(&public_key.serialize_compressed()).unwrap();
    let public_key_hex = Value::string(pub_key.to_hex());

    let version = if network_id.eq("mainnet") {
        clarity_repl::clarity::address::C32_ADDRESS_VERSION_MAINNET_SINGLESIG
    } else {
        clarity_repl::clarity::address::C32_ADDRESS_VERSION_TESTNET_SINGLESIG
    };

    let stx_address = StacksAddress::from_public_keys(
        version,
        &AddressHashMode::SerializeP2PKH,
        1,
        &vec![pub_key],
    )
    .unwrap();

    Ok((secret_key_hex, public_key_hex, stx_address))
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
