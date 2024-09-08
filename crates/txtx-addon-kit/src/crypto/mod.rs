use hmac::Hmac;
use libsecp256k1::SecretKey;
use pbkdf2::pbkdf2;
use sha2::Sha512;
use tiny_hderive::bip32::ExtendedPrivKey;

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
    let ext = ExtendedPrivKey::derive(&bip39_seed[..], derivation_path)
        .map_err(|e| format!("failed to derive private key: {:?}", e))?;
    Ok(ext.secret())
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
