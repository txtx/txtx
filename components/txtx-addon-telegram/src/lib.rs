#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

use txtx_addon_kit::{
    types::{
        commands::PreCommandSpecification, functions::FunctionSpecification,
        wallets::WalletSpecification,
    },
    Addon,
};

mod send_message;

#[derive(Debug)]
pub struct TelegramAddon;

impl TelegramAddon {
    pub fn new() -> Self {
        Self {}
    }
}

impl Addon for TelegramAddon {
    fn get_name(&self) -> &str {
        "Telegram Notifications (alpha)"
    }

    fn get_description(&self) -> &str {
        txtx_addon_kit::indoc! {r#"
            Lorem ipsum 
            "#}
    }

    fn get_namespace(&self) -> &str {
        "telegram"
    }

    fn get_functions(&self) -> Vec<FunctionSpecification> {
        vec![]
    }

    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        vec![send_message::TELEGRAM_SEND_MESSAGE.clone()]
    }

    fn get_wallets(&self) -> Vec<WalletSpecification> {
        vec![]
    }
}
