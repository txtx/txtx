pub mod secret_key;

use secret_key::SOLANA_SECRET_KEY;
use txtx_addon_kit::types::signers::SignerSpecification;

lazy_static! {
    pub static ref SIGNERS: Vec<SignerSpecification> = vec![SOLANA_SECRET_KEY.clone()];
}
