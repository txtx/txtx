use std::str::FromStr;

use bip32::{DerivationPath, XPrv as ExtendedPrivKey};
use ed25519_dalek_bip32::{DerivationPath as Ed25519DerivationPath, ExtendedSigningKey};
use hmac::Hmac;
use libsecp256k1::SecretKey;
use pbkdf2::pbkdf2;
use sha2::Sha512;

/// Derives a 32-byte secret key from a mnemonic using BIP32/secp256k1 derivation.
/// This is suitable for EVM and other secp256k1-based chains.
pub fn secret_key_bytes_from_mnemonic(
    mnemonic: &str,
    derivation_path: &str,
    is_encrypted: bool,
    password: Option<&str>,
) -> Result<[u8; 32], String> {
    if is_encrypted {
        return Err(format!("encrypted secret keys not yet supported"));
    }
    let bip39_seed = get_bip39_seed_from_mnemonic(mnemonic, password.unwrap_or(""))?;
    let derivation_path = DerivationPath::from_str(derivation_path)
        .map_err(|e| format!("failed to parse derivation path: {:?}", e))?;

    let ext = ExtendedPrivKey::derive_from_path(&bip39_seed[..], &derivation_path)
        .map_err(|e| format!("failed to derive private key: {:?}", e))?;

    Ok(ext.to_bytes())
}

/// Derives a 32-byte Ed25519 seed from a mnemonic using SLIP-0010 derivation.
/// This is the standard derivation method for Solana and other Ed25519-based chains.
/// The derivation path should be in the format "m/44'/501'/0'/0'" for Solana.
pub fn ed25519_secret_key_from_mnemonic(
    mnemonic: &str,
    derivation_path: &str,
    is_encrypted: bool,
    password: Option<&str>,
) -> Result<[u8; 32], String> {
    if is_encrypted {
        return Err(format!("encrypted secret keys not yet supported"));
    }
    let bip39_seed = get_bip39_seed_from_mnemonic(mnemonic, password.unwrap_or(""))?;

    // Parse the derivation path for ed25519-dalek-bip32
    let path = Ed25519DerivationPath::from_str(derivation_path)
        .map_err(|e| format!("failed to parse derivation path: {:?}", e))?;

    // Derive the extended signing key using SLIP-0010
    let extended_key = ExtendedSigningKey::from_seed(&bip39_seed)
        .and_then(|key| key.derive(&path))
        .map_err(|e| format!("failed to derive Ed25519 key: {:?}", e))?;

    Ok(extended_key.signing_key.to_bytes())
}

pub fn secret_key_from_bytes(secret_key_bytes: &Vec<u8>) -> Result<SecretKey, String> {
    SecretKey::parse_slice(&secret_key_bytes)
        .map_err(|e| format!("failed to parse secret key: {e}"))
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

#[cfg(test)]
mod tests {
    use super::*;

    // Standard BIP39 test mnemonic (12 words)
    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    // Solana's default derivation path
    const SOLANA_DERIVATION_PATH: &str = "m/44'/501'/0'/0'";

    #[test]
    fn test_ed25519_mnemonic_derivation_produces_32_bytes() {
        let result =
            ed25519_secret_key_from_mnemonic(TEST_MNEMONIC, SOLANA_DERIVATION_PATH, false, None);
        assert!(result.is_ok());
        let secret_key = result.unwrap();
        assert_eq!(secret_key.len(), 32);
    }

    #[test]
    fn test_ed25519_mnemonic_derivation_is_deterministic() {
        let result1 =
            ed25519_secret_key_from_mnemonic(TEST_MNEMONIC, SOLANA_DERIVATION_PATH, false, None)
                .unwrap();
        let result2 =
            ed25519_secret_key_from_mnemonic(TEST_MNEMONIC, SOLANA_DERIVATION_PATH, false, None)
                .unwrap();
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_ed25519_mnemonic_derivation_known_vector() {
        // This test uses the standard BIP39 test mnemonic with Solana's derivation path.
        // The expected secret key bytes are derived using the SLIP-0010 Ed25519 standard.
        //
        // This seed, when used to create a Solana Keypair, produces the public key:
        // HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk
        let secret_key =
            ed25519_secret_key_from_mnemonic(TEST_MNEMONIC, SOLANA_DERIVATION_PATH, false, None)
                .unwrap();

        // Expected secret key bytes for the test mnemonic with m/44'/501'/0'/0'
        // using SLIP-0010 Ed25519 derivation
        let expected_secret_key: [u8; 32] = [
            55, 223, 87, 59, 58, 196, 173, 91, 82, 46, 6, 78, 37, 182, 62, 161, 107, 203, 231, 157,
            68, 158, 129, 160, 38, 141, 16, 71, 148, 139, 180, 69,
        ];

        assert_eq!(secret_key, expected_secret_key, "Secret key should match known test vector");
    }

    #[test]
    fn test_ed25519_different_derivation_paths_produce_different_keys() {
        let key1 = ed25519_secret_key_from_mnemonic(TEST_MNEMONIC, "m/44'/501'/0'/0'", false, None)
            .unwrap();
        let key2 = ed25519_secret_key_from_mnemonic(TEST_MNEMONIC, "m/44'/501'/1'/0'", false, None)
            .unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_ed25519_password_changes_derived_key() {
        let key_no_password =
            ed25519_secret_key_from_mnemonic(TEST_MNEMONIC, SOLANA_DERIVATION_PATH, false, None)
                .unwrap();
        let key_with_password = ed25519_secret_key_from_mnemonic(
            TEST_MNEMONIC,
            SOLANA_DERIVATION_PATH,
            false,
            Some("mypassword"),
        )
        .unwrap();
        assert_ne!(key_no_password, key_with_password);
    }

    #[test]
    fn test_bip39_seed_generation() {
        let seed = get_bip39_seed_from_mnemonic(TEST_MNEMONIC, "").unwrap();
        assert_eq!(seed.len(), 64);

        // BIP39 test vector for this mnemonic with empty password
        // First few bytes should match known values
        let expected_seed_start: [u8; 8] = [0x5e, 0xb0, 0x0b, 0xbd, 0xdc, 0xf0, 0x69, 0x08];
        assert_eq!(&seed[0..8], &expected_seed_start);
    }
}
