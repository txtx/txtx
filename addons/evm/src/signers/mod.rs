use txtx_addon_kit::types::signers::SignerSpecification;

pub mod common;
mod keystore;
mod secret_key;
mod web_wallet;

use keystore::EVM_KEYSTORE_SIGNER;
use secret_key::EVM_SECRET_KEY_SIGNER;
use web_wallet::EVM_WEB_WALLET;

lazy_static! {
    pub static ref WALLETS: Vec<SignerSpecification> = vec![
        EVM_SECRET_KEY_SIGNER.clone(),
        EVM_KEYSTORE_SIGNER.clone(),
        EVM_WEB_WALLET.clone(),
    ];
}
