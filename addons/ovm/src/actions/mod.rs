pub mod package_rollup;
pub mod setup_rollup;

use setup_rollup::SETUP_ROLLUP;

use package_rollup::PACKAGE_ROLLUP;
use txtx_addon_kit::types::commands::PreCommandSpecification;

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> =
        vec![SETUP_ROLLUP.clone(), PACKAGE_ROLLUP.clone()];
}
