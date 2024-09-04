pub mod mnemonic;

use mnemonic::SOLANA_MNEMONIC;
use txtx_addon_kit::types::signers::SignerSpecification;

lazy_static! {
    pub static ref SIGNERS: Vec<SignerSpecification> = vec![SOLANA_MNEMONIC.clone()];
}
