use alloy::signers::k256::ecdsa::SigningKey;
use alloy_signer_local::{coins_bip39::English, LocalSigner, MnemonicBuilder};
use hmac::digest::generic_array::GenericArray;

use crate::signers::DEFAULT_DERIVATION_PATH;

pub type UnsafeSigner = LocalSigner<SigningKey>;
pub fn mnemonic_to_unsafe_signer(
    mnemonic: &str,
    derivation_path: Option<&str>,
    is_encrypted: Option<bool>,
    password: Option<&str>,
) -> Result<UnsafeSigner, String> {
    if is_encrypted.is_some() {
        return Err("encrypted mnemonic signers are not yet supported".to_string());
    }
    let derivation_path = derivation_path.unwrap_or(DEFAULT_DERIVATION_PATH);

    let mut mnemonic_builder = MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .derivation_path(derivation_path)
        .map_err(|e| format!("failed to instantiate unsafe signer from mnemonic: {e}"))?;

    if let Some(password) = password {
        mnemonic_builder = mnemonic_builder.password(password)
    }
    let signer = mnemonic_builder
        .build()
        .map_err(|e| format!("failed to build unsafe signer from mnemonic: {e}"))?;
    Ok(signer)
}

pub fn secret_key_to_unsafe_signer(secret_key: &Vec<u8>) -> Result<UnsafeSigner, String> {
    let signing_key = SigningKey::from_slice(&secret_key)
        .map_err(|e| format!("failed to generate signing key from secret key: {e}"))?;
    let signer = UnsafeSigner::from_signing_key(signing_key);
    Ok(signer)
}

pub fn field_bytes_to_unsafe_signer(field_bytes: &Vec<u8>) -> Result<UnsafeSigner, String> {
    let bytes = GenericArray::from_slice(field_bytes);
    UnsafeSigner::from_field_bytes(bytes).map_err(|e| format!("failed to generate signer: {}", e))
}
