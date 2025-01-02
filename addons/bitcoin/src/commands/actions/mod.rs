pub mod encode;

use encode::ENCODE_SCRIPT;
use txtx_addon_kit::types::commands::PreCommandSpecification;

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![ENCODE_SCRIPT.clone()];
}
