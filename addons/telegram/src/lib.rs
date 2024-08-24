#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

use txtx_addon_kit::{types::commands::PreCommandSpecification, Addon};

mod send_message;
mod setup_chat;

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

    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        vec![send_message::TELEGRAM_SEND_MESSAGE.clone(), setup_chat::TELEGRAM_SETUP_CHAT.clone()]
    }
}
