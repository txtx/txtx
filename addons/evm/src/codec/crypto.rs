use alloy::{
    hex::FromHex,
    primitives::{keccak256, Address},
    signers::k256::ecdsa::{SigningKey, VerifyingKey},
};
use alloy_signer_local::{coins_bip39::English, LocalSigner, MnemonicBuilder};
use hmac::digest::generic_array::GenericArray;
use libsecp256k1::{recover, Message, RecoveryId, Signature};
use txtx_addon_kit::hex;

use crate::constants::DEFAULT_DERIVATION_PATH;

pub type SecretKeySigner = LocalSigner<SigningKey>;
pub fn mnemonic_to_secret_key_signer(
    mnemonic: &str,
    derivation_path: Option<&str>,
    is_encrypted: Option<bool>,
    password: Option<&str>,
) -> Result<SecretKeySigner, String> {
    if is_encrypted.is_some() {
        return Err("encrypted mnemonic signers are not yet supported".to_string());
    }
    let derivation_path = derivation_path.unwrap_or(DEFAULT_DERIVATION_PATH);

    let mut mnemonic_builder = MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .derivation_path(derivation_path)
        .map_err(|e| format!("failed to instantiate secret key signer from mnemonic: {e}"))?;

    if let Some(password) = password {
        mnemonic_builder = mnemonic_builder.password(password)
    }
    let signer = mnemonic_builder
        .build()
        .map_err(|e| format!("failed to build secret key signer from mnemonic: {e}"))?;
    Ok(signer)
}

pub fn secret_key_to_secret_key_signer(secret_key: &Vec<u8>) -> Result<SecretKeySigner, String> {
    let signing_key = SigningKey::from_slice(&secret_key)
        .map_err(|e| format!("failed to generate signing key from secret key: {e}"))?;
    let signer = SecretKeySigner::from_signing_key(signing_key);
    Ok(signer)
}

pub fn field_bytes_to_secret_key_signer(field_bytes: &Vec<u8>) -> Result<SecretKeySigner, String> {
    let bytes = GenericArray::from_slice(field_bytes);
    SecretKeySigner::from_field_bytes(bytes)
        .map_err(|e| format!("failed to generate signer: {}", e))
}

/// Resolves the keystore file path from the account name and optional directory.
/// If keystore_path is None, defaults to ~/.foundry/keystores
pub fn resolve_keystore_path(
    keystore_account: &str,
    keystore_path: Option<&str>,
) -> Result<std::path::PathBuf, String> {
    use std::path::PathBuf;

    let keystore_dir = match keystore_path {
        Some(path) => PathBuf::from(path),
        None => {
            let home = dirs::home_dir()
                .ok_or_else(|| "could not determine home directory".to_string())?;
            home.join(".foundry").join("keystores")
        }
    };

    // If keystore_account is already an absolute path, use it directly
    let account_path = PathBuf::from(keystore_account);
    if account_path.is_absolute() {
        return Ok(account_path);
    }

    // Otherwise, join with the keystore directory
    // Try with .json extension first, then without
    let with_json = keystore_dir.join(format!("{}.json", keystore_account));
    if with_json.exists() {
        return Ok(with_json);
    }

    let without_json = keystore_dir.join(keystore_account);
    if without_json.exists() {
        return Ok(without_json);
    }

    // Return the path with .json extension (will fail at decrypt time with better error)
    Ok(with_json)
}

/// Decrypts a keystore file and returns a SecretKeySigner
pub fn keystore_to_secret_key_signer(
    keystore_path: &std::path::Path,
    password: &str,
) -> Result<SecretKeySigner, String> {
    if !keystore_path.exists() {
        return Err(format!("keystore file not found: {:?}", keystore_path));
    }

    if keystore_path.is_dir() {
        return Err(format!(
            "keystore path is a directory, expected a file: {:?}",
            keystore_path
        ));
    }

    let secret_key = eth_keystore::decrypt_key(keystore_path, password)
        .map_err(|e| format!("failed to decrypt keystore: {}", e))?;

    let signing_key = SigningKey::from_slice(&secret_key)
        .map_err(|e| format!("invalid key in keystore: {}", e))?;

    Ok(SecretKeySigner::from_signing_key(signing_key))
}

pub fn public_key_to_address(public_key_bytes: &Vec<u8>) -> Result<Address, String> {
    let pubkey = VerifyingKey::from_sec1_bytes(&public_key_bytes)
        .map_err(|e| format!("invalid public key: {}", e))?;
    Ok(Address::from_public_key(&pubkey))
}

pub fn public_key_from_signed_message(
    message: &str,
    signature_hex: &str,
) -> Result<Vec<u8>, String> {
    let prefixed_message = format!("\x19Ethereum Signed Message:\n{}{}", message.len(), message);

    let message_hex = hex::encode(prefixed_message);

    let message_bytes = Vec::from_hex(message_hex).map_err(|e| {
        format!("failed to get public key from signature: invalid hex message: {e}")
    })?;

    let signature_bytes = Vec::from_hex(signature_hex).map_err(|e| {
        format!("failed to get public key from signature: invalid hex signature: {e}")
    })?;

    let message_hash = keccak256(&message_bytes);

    let v = signature_bytes[64];
    let signature_array: &[u8; 64] = signature_bytes[0..64]
        .try_into()
        .map_err(|_| "failed to get public key from signature: invalid signature length")?;
    let signature = Signature::parse_standard(signature_array)
        .map_err(|e| format!("failed to get public key from signature: invalid signature: {e}"))?;

    // Recover the public key
    let recovery_id = RecoveryId::parse(v - 27).map_err(|e| {
        format!("failed to get public key from signature: invalid recovery id: {e}")
    })?;
    let message = Message::parse_slice(&message_hash.0).map_err(|e| {
        format!("failed to get public key from signature: invalid message hash: {e}")
    })?;
    let public_key = recover(&message, &signature, &recovery_id).map_err(|e| {
        format!("failed to get public key from signature: failed to recover public key: {e}")
    })?;
    let public_key_bytes = public_key.serialize().to_vec();
    Ok(public_key_bytes)
}
