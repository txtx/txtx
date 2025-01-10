use txtx_addon_network_bitcoin::BitcoinNetworkAddon;
use txtx_addon_network_evm::EvmNetworkAddon;
use txtx_addon_network_stacks::StacksNetworkAddon;
use txtx_addon_network_svm::SvmNetworkAddon;
use txtx_addon_telegram::TelegramAddon;
use txtx_core::{kit::Addon, std::StdAddon};

mod macros;

#[macro_use]
extern crate hiro_system_kit;

pub mod cli;
pub mod manifest;
pub mod snapshots;
pub mod term_ui;
pub mod web_ui;
pub mod serve;

pub fn get_available_addons() -> Vec<Box<dyn Addon>> {
    vec![
        Box::new(StdAddon::new()),
        Box::new(SvmNetworkAddon::new()),
        Box::new(StacksNetworkAddon::new()),
        Box::new(EvmNetworkAddon::new()),
        Box::new(BitcoinNetworkAddon::new()),
        Box::new(TelegramAddon::new()),
        #[cfg(feature = "sp1")]
        Box::new(Sp1Addon::new()),
        #[cfg(feature = "ovm")]
        Box::new(OvmNetworkAddon::new()),
    ]
}

pub fn get_addon_by_namespace(namespace: &str) -> Option<Box<dyn Addon>> {
    let available_addons = get_available_addons();
    for addon in available_addons.into_iter() {
        if namespace.starts_with(&format!("{}", addon.get_namespace())) {
            return Some(addon);
        }
    }
    None
}

fn main() {
    cli::main();
}
