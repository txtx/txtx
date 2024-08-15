use txtx_addon_kit::types::signers::SignerSpecification;

pub mod mnemonic;

use mnemonic::EVM_MNEMONIC;

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/60'/0'/0/0";

lazy_static! {
    pub static ref WALLETS: Vec<SignerSpecification> = vec![EVM_MNEMONIC.clone()];
}
