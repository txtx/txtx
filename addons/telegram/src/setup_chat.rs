use teloxide::prelude::*;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{commands::CommandSpecification, diagnostics::Diagnostic, types::Type};

lazy_static! {
    pub static ref TELEGRAM_SETUP_CHAT: PreCommandSpecification = define_command! {
      TelegramSetupChat => {
          name: "Setup Telegram Chat",
          matcher: "setup_chat",
          documentation: "The `telegram::setup_chat` ...",
          implements_signing_capability: false,
          implements_background_task_capability: false,
          inputs: [
            description: {
                documentation: "A description of the call.",
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false
            },
            telegram_bot_api_token: {
              documentation: "Coming soon",
              typing: Type::string(),
              optional: false,
              tainting: false,
                internal: false
            }
          ],
          outputs: [
              result: {
                  documentation: "The chat_id.",
                  typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
            action "setup_bot_chat" "telegram::setup_chat" {
                description = "Handshake"
                telegram_bot_api_token = env.telegram_bot_api_token
            }
      "#},
      }
    };
}

pub struct TelegramSetupChat;
impl CommandImplementation for TelegramSetupChat {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _supervision_context: &RunbookSupervisionContext,
        _auth_context: &txtx_addon_kit::types::AuthorizationContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none())
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let values = values.clone();

        let telegram_bot_api_token =
            values.get_expected_string("telegram_bot_api_token")?.to_string();

        let future = async move {
            let result = CommandExecutionResult::new();
            let bot = Bot::new(telegram_bot_api_token);
            let code = "8488".to_string();
            println!(
                "Telegram Handshake required. Send PIN '{}' in your Telegram chat with the Bot.",
                code
            );
            teloxide::repl(bot, |bot: Bot, msg: Message| async move {
                match msg.text() {
                    Some("8488") => {
                        bot.send_message(msg.chat.id, "Setup completed 💪").await?;
                        println!(
                            "Telegram Handshake completed, the following action can now be used:\n\naction \"notify_team\" \"telegram::send_message\" {{\n    telegram_bot_api_token = \"{}\"\n    telegram_chat_id = {}\n    message = \"Runbook execution triggered.\"\n}}\n\nCtrl+C to exit",
                            bot.token(),
                            msg.chat.id
                        );
                        return Ok(())
                    }
                    _ => {
                        bot.send_message(msg.chat.id, "Setup failed").await?;
                    }
                };
                Ok(())
            })
            .await;
            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
