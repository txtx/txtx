pub mod encode;

use encode::ENCODE;
use txtx_addon_kit::types::commands::PreCommandSpecification;

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![ENCODE.clone()];
}
