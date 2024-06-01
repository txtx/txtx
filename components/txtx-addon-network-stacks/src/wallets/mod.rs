use txtx_addon_kit::types::wallets::WalletSpecification;

mod connect;
mod multisig;

use connect::STACKS_CONNECT;
use multisig::STACKS_MULTISIG;

lazy_static! {
    pub static ref WALLETS: Vec<WalletSpecification> =
        vec![STACKS_CONNECT.clone(), STACKS_MULTISIG.clone()];
}
