pub mod secret_key;

use secret_key::SOLANA_SECRET_KEY;
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    signers::{signer_diag_with_namespace_ctx, SignerSpecification},
};

use crate::constants::NAMESPACE;

lazy_static! {
    pub static ref SIGNERS: Vec<SignerSpecification> = vec![SOLANA_SECRET_KEY.clone()];
}

pub fn namespaced_err_fn() -> impl Fn(&SignerSpecification, &str, String) -> Diagnostic {
    let error_fn = signer_diag_with_namespace_ctx(NAMESPACE.to_string());
    error_fn
}
