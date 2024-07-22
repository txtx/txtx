use std::collections::HashMap;

use crate::codec::codec::{
    StacksTransaction, StacksTransactionSigner, TransactionSpendingCondition, Txid,
};
use clarity::address::AddressHashMode;
use clarity::codec::StacksMessageCodec;
use clarity::types::chainstate::StacksAddress;
use clarity::types::PrivateKey;
use clarity::util::secp256k1::{Secp256k1PrivateKey, Secp256k1PublicKey};
use hmac::Hmac;
use libsecp256k1::{PublicKey, SecretKey};
use pbkdf2::pbkdf2;
use tiny_hderive::bip32::ExtendedPrivKey;
use txtx_addon_kit::sha2::Sha512;
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, ReviewInputRequest,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::wallets::{
    return_synchronous_result, CheckSignabilityOk, SigningCommandsState, WalletActionErr,
    WalletActionsFutureResult, WalletActivateFutureResult, WalletInstance, WalletSignFutureResult,
};
use txtx_addon_kit::types::wallets::{WalletImplementation, WalletSpecification};
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructDid, ValueStore};
use txtx_addon_kit::{channel, AddonDefaults};

use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, CACHED_NONCE, MESSAGE_BYTES, SIGNED_MESSAGE_BYTES,
};
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::wallets::return_synchronous_actions;

use crate::constants::{NETWORK_ID, PUBLIC_KEYS, SIGNED_TRANSACTION_BYTES};
use crate::typing::{CLARITY_BUFFER, STACKS_SIGNATURE, STACKS_TRANSACTION};

use super::DEFAULT_DERIVATION_PATH;

lazy_static! {
    pub static ref STACKS_MNEMONIC: WalletSpecification = define_wallet! {
        StacksMnemonic => {
          name: "Mnemonic Wallet",
          matcher: "mnemonic",
          documentation:txtx_addon_kit::indoc! {r#"The `mnemonic` wallet can be used to synchronously sign a transaction."#},
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
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
        wallet "bob" "stacks::mnemonic" {
            mnemonic = "board list obtain sugar hour worth raven scout denial thunder horse logic fury scorpion fold genuine phrase wealth news aim below celery when cabin"
            derivation_path = "m/44'/5757'/0'/0/0"
        }
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
        _construct_id: &ConstructDid,
        instance_name: &str,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut signing_command_state: ValueStore,
        wallets: SigningCommandsState,
        _wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> WalletActionsFutureResult {
        use crate::constants::CHECKED_PUBLIC_KEY;

        let mut actions = Actions::none();

        if signing_command_state.get_value(PUBLIC_KEYS).is_some() {
            return return_synchronous_actions(Ok((wallets, signing_command_state, actions)));
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
        let network_id = args.get_defaulting_string(NETWORK_ID, defaults).unwrap();
        let version = match network_id.as_str() {
            "mainnet" => AddressHashMode::SerializeP2PKH.to_version_mainnet(),
            _ => AddressHashMode::SerializeP2PKH.to_version_testnet(),
        };
        signing_command_state.insert("mnemonic", mnemonic);
        signing_command_state.insert("derivation_path", derivation_path);
        signing_command_state.insert("is_encrypted", is_encrypted);
        signing_command_state.insert("hash_flag", Value::uint(version.into()));
        signing_command_state.insert("multi_sig", Value::bool(false));

        let (_, public_key, expected_address) =
            match compute_keypair(args, defaults, &signing_command_state) {
                Ok(value) => value,
                Err(diag) => return Err((wallets, signing_command_state, diag)),
            };
        signing_command_state.insert(PUBLIC_KEYS, Value::array(vec![public_key.clone()]));
        signing_command_state.insert(CHECKED_PUBLIC_KEY, Value::array(vec![public_key.clone()]));

        if supervision_context.review_input_values {
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
        let future = async move { Ok((wallets, signing_command_state, actions)) };
        Ok(Box::pin(future))
    }

    fn activate(
        _construct_id: &ConstructDid,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        signing_command_state: ValueStore,
        wallets: SigningCommandsState,
        _wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult {
        let result = CommandExecutionResult::new();
        return_synchronous_result(Ok((wallets, signing_command_state, result)))
    }

    fn check_signability(
        _caller_uuid: &ConstructDid,
        _title: &str,
        _description: &Option<String>,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        signing_command_state: ValueStore,
        wallets: SigningCommandsState,
        _wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        _defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, WalletActionErr> {
        Ok((wallets, signing_command_state, Actions::none()))
    }

    fn sign(
        _caller_uuid: &ConstructDid,
        _title: &str,
        payload: &Value,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut signing_command_state: ValueStore,
        wallets: SigningCommandsState,
        _wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        defaults: &AddonDefaults,
    ) -> WalletSignFutureResult {
        let mut result = CommandExecutionResult::new();

        let (secret_key_value, public_key_value, _) =
            match compute_keypair(args, defaults, &signing_command_state) {
                Ok(value) => value,
                Err(diag) => return Err((wallets, signing_command_state, diag)),
            };

        let payload_buffer = payload.expect_buffer_data();

        if payload_buffer.typing.eq(&STACKS_TRANSACTION) {
            let transaction =
                StacksTransaction::consensus_deserialize(&mut &payload_buffer.bytes[..]).unwrap();
            let mut tx_signer = StacksTransactionSigner::new(&transaction);
            let secret_key = Secp256k1PrivateKey::from_slice(&secret_key_value.to_bytes()).unwrap();
            tx_signer.sign_origin(&secret_key).unwrap();
            let signed_transaction = tx_signer.get_tx_incomplete();

            let nonce = signed_transaction.get_origin_nonce();
            signing_command_state.insert(CACHED_NONCE, Value::uint(nonce));

            let mut signed_transaction_bytes = vec![];
            signed_transaction
                .consensus_serialize(&mut signed_transaction_bytes)
                .unwrap(); // todo
            result.outputs.insert(
                SIGNED_TRANSACTION_BYTES.into(),
                Value::buffer(signed_transaction_bytes, STACKS_TRANSACTION.clone()),
            );
        } else {
            let secret_key =
                Secp256k1PrivateKey::from_slice(&secret_key_value.expect_buffer_bytes()).unwrap();
            let public_key =
                Secp256k1PublicKey::from_slice(&public_key_value.expect_buffer_bytes()).unwrap();
            let signature = secret_key.sign(&payload_buffer.bytes).unwrap();
            let cur_sighash = Txid::from_bytes(&payload_buffer.bytes).unwrap();
            let next_sighash = TransactionSpendingCondition::make_sighash_postsign(
                &cur_sighash,
                &public_key,
                &signature,
            );
            let message = Value::buffer(next_sighash.to_bytes().to_vec(), STACKS_SIGNATURE.clone());
            let signature = Value::buffer(signature.to_bytes().to_vec(), STACKS_SIGNATURE.clone());
            result.outputs.insert(MESSAGE_BYTES.into(), message);
            result
                .outputs
                .insert(SIGNED_MESSAGE_BYTES.into(), signature);
        }

        return_synchronous_result(Ok((wallets, signing_command_state, result)))
    }
}

pub fn compute_keypair(
    args: &ValueStore,
    defaults: &AddonDefaults,
    signing_command_state: &ValueStore,
) -> Result<(Value, Value, StacksAddress), Diagnostic> {
    let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;
    let mnemonic = signing_command_state.get_expected_string("mnemonic")?;
    let derivation_path = signing_command_state
        .get_expected_string("derivation_path")
        .unwrap_or(DEFAULT_DERIVATION_PATH);
    let is_encrypted = signing_command_state
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
