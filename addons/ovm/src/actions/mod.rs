pub mod generate_l2_config_files;

use generate_l2_config_files::GENERATE_L2_CONFIG_FILES;

use txtx_addon_kit::types::commands::PreCommandSpecification;

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![GENERATE_L2_CONFIG_FILES.clone()];
}
