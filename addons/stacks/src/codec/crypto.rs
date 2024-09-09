use clarity::address::AddressHashMode;
use clarity::codec::StacksMessageCodec;
use clarity::types::chainstate::StacksAddress;
use clarity::types::PrivateKey;
use clarity::util::secp256k1::{Secp256k1PrivateKey, Secp256k1PublicKey};
use txtx_addon_kit::hmac::Hmac;
use txtx_addon_kit::pbkdf2::pbkdf2;
use txtx_addon_kit::secp256k1::{PublicKey, SecretKey};
use txtx_addon_kit::sha2::Sha512;
use txtx_addon_kit::types::types::Value;

use crate::typing::StacksValue;

use super::codec::{
    StacksTransaction, StacksTransactionSigner, TransactionSpendingCondition, Txid,
};

pub fn version_from_network_id(network_id: &str) -> u8 {
    match network_id {
        "mainnet" => AddressHashMode::SerializeP2PKH.to_version_mainnet(),
        _ => AddressHashMode::SerializeP2PKH.to_version_testnet(),
    }
}

pub fn compute_keypair(
    secret_key: SecretKey,
    network_id: &str,
) -> Result<(Value, Value, StacksAddress), String> {
    let secret_key_bytes = secret_key.serialize().to_vec();
    let secret_key_hex = StacksValue::buffer(secret_key_bytes);

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
        &vec![pub_key.clone()],
    )
    .ok_or(format!("failed to generate stacks address from public key {}", pub_key.to_hex()))?;

    Ok((secret_key_hex, public_key_hex, stx_address))
}

pub fn get_bip39_seed_from_mnemonic(mnemonic: &str, password: &str) -> Result<Vec<u8>, String> {
    const PBKDF2_ROUNDS: u32 = 2048;
    const PBKDF2_BYTES: usize = 64;
    let salt = format!("mnemonic{}", password);
    let mut seed = vec![0u8; PBKDF2_BYTES];

    pbkdf2::<Hmac<Sha512>>(mnemonic.as_bytes(), salt.as_bytes(), PBKDF2_ROUNDS, &mut seed)
        .map_err(|e| e.to_string())?;
    Ok(seed)
}

pub fn sign_transaction(
    transaction_bytes: &Vec<u8>,
    secret_key_bytes: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let transaction = StacksTransaction::consensus_deserialize(&mut &transaction_bytes[..])
        .map_err(|e| format!("failed to decode stacks transaction: {e}"))?;
    let mut tx_signer = StacksTransactionSigner::new(&transaction);
    let secret_key = Secp256k1PrivateKey::from_slice(&secret_key_bytes)
        .map_err(|e| format!("failed to generate secret key: {e}"))?;
    tx_signer.sign_origin(&secret_key).map_err(|e| format!("failed to sign transaction: {}", e))?;
    let signed_transaction = tx_signer.get_tx_incomplete();

    let mut signed_transaction_bytes = vec![];
    signed_transaction
        .consensus_serialize(&mut signed_transaction_bytes)
        .map_err(|e| format!("failed to serialize signed transaction: {e}"))?;
    Ok(signed_transaction_bytes)
}

pub fn sign_message(
    message_bytes: &Vec<u8>,
    secret_key_bytes: Vec<u8>,
    public_key_bytes: Vec<u8>,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let secret_key = Secp256k1PrivateKey::from_slice(&secret_key_bytes)
        .map_err(|e| format!("failed to generate secret key: {e}"))?;
    let public_key = Secp256k1PublicKey::from_slice(&public_key_bytes)
        .map_err(|e| format!("failed to generate public key: {e}"))?;
    let signature =
        secret_key.sign(&message_bytes).map_err(|e| format!("failed to sign message: {}", e))?;
    let cur_sighash =
        Txid::from_bytes(&message_bytes).ok_or(format!("failed to generate current sighash"))?;
    let next_sighash =
        TransactionSpendingCondition::make_sighash_postsign(&cur_sighash, &public_key, &signature);
    Ok((next_sighash.to_bytes().to_vec(), signature.to_bytes().to_vec()))
}
