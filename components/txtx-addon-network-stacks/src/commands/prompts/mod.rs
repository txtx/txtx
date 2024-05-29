mod multisig;

use multisig::MULTISIG;
use txtx_addon_kit::types::commands::PreCommandSpecification;

lazy_static! {
    pub static ref PROMPTS: Vec<PreCommandSpecification> = vec![
        MULTISIG.clone()
    ];
}
