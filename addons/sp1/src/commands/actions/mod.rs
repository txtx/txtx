pub mod create_proof;

use create_proof::CREATE_PROOF;
use txtx_addon_kit::types::commands::PreCommandSpecification;

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![CREATE_PROOF.clone()];
}
