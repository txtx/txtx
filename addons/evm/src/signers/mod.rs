use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    signers::{err_to_signer_ctx_diag, SignerSpecification},
};

pub mod common;
mod secret_key;
mod web_wallet;

use secret_key::EVM_SECRET_KEY_SIGNER;
use web_wallet::EVM_WEB_WALLET;

use crate::constants::NAMESPACE;

lazy_static! {
    pub static ref WALLETS: Vec<SignerSpecification> =
        vec![EVM_SECRET_KEY_SIGNER.clone(), EVM_WEB_WALLET.clone()];
}

pub fn namespaced_err_fn() -> impl Fn(&SignerSpecification, &str, String) -> Diagnostic {
    let error_fn = err_to_signer_ctx_diag(NAMESPACE.to_string());
    error_fn
}
