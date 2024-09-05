use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    signers::{signer_diag_with_namespace_ctx, SignerSpecification},
};

pub mod mnemonic;

use mnemonic::EVM_MNEMONIC;

use crate::constants::NAMESPACE;

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/60'/0'/0/0";

lazy_static! {
    pub static ref WALLETS: Vec<SignerSpecification> = vec![EVM_MNEMONIC.clone()];
}

pub fn namespaced_err_fn() -> impl Fn(&SignerSpecification, &str, String) -> Diagnostic {
    let error_fn = signer_diag_with_namespace_ctx(NAMESPACE.to_string());
    error_fn
}
