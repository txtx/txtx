pub mod package_rollup;
pub mod start_rollup;

use start_rollup::START_ROLLUP;

use package_rollup::PACKAGE_ROLLUP;
use txtx_addon_kit::types::commands::PreCommandSpecification;

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> =
        vec![START_ROLLUP.clone(), PACKAGE_ROLLUP.clone()];
}
