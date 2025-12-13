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

    // If keystore_account is already an absolute path, use it directly.
    // Security: The user provides both the path and password interactively; only valid encrypted keystores are accepted.
    // UX: The .json extension check is mainly to catch typos and provide clearer error messages.
    let account_path = PathBuf::from(keystore_account);
    if account_path.is_absolute() {
        if !account_path.extension().map_or(false, |ext| ext == "json") {
            return Err(format!(
                "absolute keystore path should have .json extension: {:?}",
                account_path
            ));
        }
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

    let secret_key = eth_keystore::decrypt_key(keystore_path, password).map_err(|e| {
        let err_str = e.to_string();
        if err_str.contains("Mac Mismatch") {
            format!("incorrect password for keystore '{}'", keystore_path.display())
        } else {
            format!("failed to decrypt keystore '{}': {}", keystore_path.display(), err_str)
        }
    })?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ============ resolve_keystore_path tests ============

    #[test]
    fn test_resolve_keystore_path_absolute_path_with_json() {
        let result = resolve_keystore_path("/absolute/path/to/keystore.json", None);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().to_str().unwrap(),
            "/absolute/path/to/keystore.json"
        );
    }

    #[test]
    fn test_resolve_keystore_path_absolute_path_without_json_rejected() {
        let result = resolve_keystore_path("/some/path/without/extension", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains(".json extension"));
    }

    #[test]
    fn test_resolve_keystore_path_with_custom_directory() {
        let temp_dir = TempDir::new().unwrap();
        let keystore_path = temp_dir.path().join("myaccount.json");
        fs::write(&keystore_path, "{}").unwrap();

        let result =
            resolve_keystore_path("myaccount", Some(temp_dir.path().to_str().unwrap()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), keystore_path);
    }

    #[test]
    fn test_resolve_keystore_path_adds_json_extension() {
        let temp_dir = TempDir::new().unwrap();
        let keystore_path = temp_dir.path().join("myaccount.json");
        fs::write(&keystore_path, "{}").unwrap();

        let result =
            resolve_keystore_path("myaccount", Some(temp_dir.path().to_str().unwrap()));
        assert!(result.is_ok());
        assert!(result.unwrap().to_string_lossy().ends_with(".json"));
    }

    #[test]
    fn test_resolve_keystore_path_without_json_extension() {
        let temp_dir = TempDir::new().unwrap();
        let keystore_path = temp_dir.path().join("myaccount");
        fs::write(&keystore_path, "{}").unwrap();

        let result =
            resolve_keystore_path("myaccount", Some(temp_dir.path().to_str().unwrap()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), keystore_path);
    }

    #[test]
    fn test_resolve_keystore_path_nonexistent_returns_with_json() {
        let temp_dir = TempDir::new().unwrap();

        let result =
            resolve_keystore_path("nonexistent", Some(temp_dir.path().to_str().unwrap()));
        assert!(result.is_ok());
        assert!(result
            .unwrap()
            .to_string_lossy()
            .ends_with("nonexistent.json"));
    }

    #[test]
    fn test_resolve_keystore_path_default_foundry_dir() {
        let result = resolve_keystore_path("myaccount", None);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains(".foundry"));
        assert!(path.to_string_lossy().contains("keystores"));
    }

    #[test]
    fn test_resolve_keystore_path_prefers_json_extension() {
        let temp_dir = TempDir::new().unwrap();
        // Create both files
        let with_json = temp_dir.path().join("myaccount.json");
        let without_json = temp_dir.path().join("myaccount");
        fs::write(&with_json, "json").unwrap();
        fs::write(&without_json, "no-json").unwrap();

        let result =
            resolve_keystore_path("myaccount", Some(temp_dir.path().to_str().unwrap()));
        assert!(result.is_ok());
        // Should prefer .json extension
        assert_eq!(result.unwrap(), with_json);
    }

    // ============ keystore_to_secret_key_signer tests ============

    #[test]
    fn test_keystore_to_secret_key_signer_file_not_found() {
        let result = keystore_to_secret_key_signer(
            std::path::Path::new("/nonexistent/path.json"),
            "password",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_keystore_to_secret_key_signer_directory_error() {
        let temp_dir = TempDir::new().unwrap();

        let result = keystore_to_secret_key_signer(temp_dir.path(), "password");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("directory"));
    }

    #[test]
    fn test_keystore_to_secret_key_signer_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let keystore_path = temp_dir.path().join("invalid.json");
        fs::write(&keystore_path, "not valid json").unwrap();

        let result = keystore_to_secret_key_signer(&keystore_path, "password");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("decrypt"));
    }

    #[test]
    fn test_keystore_to_secret_key_signer_wrong_password() {
        let temp_dir = TempDir::new().unwrap();

        // Create a valid keystore with known password
        // Note: eth_keystore::encrypt_key returns UUID, but file is named by the `name` param
        let secret_key = [1u8; 32];
        let mut rng = rand::thread_rng();
        let _uuid = eth_keystore::encrypt_key(
            temp_dir.path(),
            &mut rng,
            &secret_key,
            "correct_password",
            Some("test"),
        )
        .unwrap();

        let keystore_path = temp_dir.path().join("test");
        let result = keystore_to_secret_key_signer(&keystore_path, "wrong_password");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("incorrect password"));
    }

    #[test]
    fn test_keystore_cannot_decrypt_without_password() {
        let temp_dir = TempDir::new().unwrap();

        let secret_key = [0xABu8; 32];
        let mut rng = rand::thread_rng();
        let _uuid = eth_keystore::encrypt_key(
            temp_dir.path(),
            &mut rng,
            &secret_key,
            "strong_password_123",
            Some("secured"),
        )
        .unwrap();

        let keystore_path = temp_dir.path().join("secured");

        // Verify keystore file exists and contains encrypted data
        assert!(keystore_path.exists());
        let keystore_contents = fs::read_to_string(&keystore_path).unwrap();
        assert!(keystore_contents.contains("crypto"));
        assert!(keystore_contents.contains("ciphertext"));
        // The raw secret key should NOT appear in the file
        assert!(!keystore_contents.contains("abababab"));

        // Empty password must fail
        let result_empty = keystore_to_secret_key_signer(&keystore_path, "");
        assert!(result_empty.is_err(), "Empty password should fail decryption");

        // Partial password must fail
        let result_partial = keystore_to_secret_key_signer(&keystore_path, "strong");
        assert!(result_partial.is_err(), "Partial password should fail decryption");

        // Correct password succeeds
        let result_correct = keystore_to_secret_key_signer(&keystore_path, "strong_password_123");
        assert!(result_correct.is_ok(), "Correct password should succeed");
    }

    #[test]
    fn test_keystore_to_secret_key_signer_success() {
        let temp_dir = TempDir::new().unwrap();

        let secret_key = [0x42u8; 32];
        let mut rng = rand::thread_rng();
        let _uuid = eth_keystore::encrypt_key(
            temp_dir.path(),
            &mut rng,
            &secret_key,
            "test_password",
            Some("testaccount"),
        )
        .unwrap();

        // File is named by the `name` parameter, not the returned UUID
        let keystore_path = temp_dir.path().join("testaccount");
        let result = keystore_to_secret_key_signer(&keystore_path, "test_password");

        assert!(result.is_ok());
        let signer = result.unwrap();
        let _address = signer.address();
    }

    /// Tests the full keystore encryption/decryption roundtrip:
    /// 1. Create a signer from a known mnemonic (source of truth)
    /// 2. Extract the private key bytes and encrypt them into a keystore file
    /// 3. Decrypt the keystore with the correct password
    /// 4. Verify the decrypted signer produces the same address as the original
    ///
    /// This proves that providing the correct password extracts the original key,
    /// since identical addresses can only be derived from identical private keys.
    #[test]
    fn test_keystore_roundtrip_extracts_correct_key_with_password() {
        let temp_dir = TempDir::new().unwrap();

        // Step 1: Create signer from known mnemonic - this is our source of truth
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let original_signer =
            mnemonic_to_secret_key_signer(mnemonic, None, None, None).unwrap();
        let expected_address = original_signer.address();

        // Step 2: Encrypt the private key into a keystore file
        let field_bytes = original_signer.to_field_bytes().to_vec();
        let mut rng = rand::thread_rng();
        let _uuid = eth_keystore::encrypt_key(
            temp_dir.path(),
            &mut rng,
            &field_bytes,
            "password",
            Some("test"),
        )
        .unwrap();

        // Step 3: Decrypt keystore with correct password
        let keystore_path = temp_dir.path().join("test");
        let decrypted_signer =
            keystore_to_secret_key_signer(&keystore_path, "password").unwrap();

        // Step 4: Verify decrypted key matches original by comparing addresses
        // (address is derived from private key, so matching = same key)
        assert_eq!(decrypted_signer.address(), expected_address);
    }

    // ============ mnemonic_to_secret_key_signer tests ============

    #[test]
    fn test_mnemonic_to_secret_key_signer_valid() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let result = mnemonic_to_secret_key_signer(mnemonic, None, None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mnemonic_to_secret_key_signer_invalid_mnemonic() {
        let result =
            mnemonic_to_secret_key_signer("invalid mnemonic words", None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_mnemonic_to_secret_key_signer_custom_derivation_path() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let result1 = mnemonic_to_secret_key_signer(mnemonic, None, None, None);
        let result2 =
            mnemonic_to_secret_key_signer(mnemonic, Some("m/44'/60'/0'/0/1"), None, None);

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        // Different derivation paths should produce different addresses
        assert_ne!(result1.unwrap().address(), result2.unwrap().address());
    }

    #[test]
    fn test_mnemonic_to_secret_key_signer_encrypted_not_supported() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let result =
            mnemonic_to_secret_key_signer(mnemonic, None, Some(true), Some("password"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not yet supported"));
    }
}
