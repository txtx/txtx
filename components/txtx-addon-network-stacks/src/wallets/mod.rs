use txtx_addon_kit::types::wallets::WalletSpecification;

use crate::wallets::connect::STACKS_CONNECT;
mod connect;

lazy_static! {
    pub static ref WALLETS: Vec<WalletSpecification> = vec![STACKS_CONNECT.clone()];
}
