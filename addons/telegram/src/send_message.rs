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
    pub static ref TELEGRAM_SEND_MESSAGE: PreCommandSpecification = define_command! {
      TelegramSendMessage => {
          name: "Send Telegram Message",
          matcher: "send_message",
          documentation: "The `telegram::send_message` ...",
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
            },
            telegram_chat_id: {
                documentation: "Coming soon",
                typing: Type::string(),
                optional: false,
                tainting: true,
                internal: false
            },
            message: {
                documentation: "Message to send.",
                typing: Type::string(),
                optional: false,
                tainting: true,
                internal: false
            }
          ],
          outputs: [
              result: {
                  documentation: "The contract call result.",
                  typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
            action "notify_team" "telegram::send_message" {
                description = "Notify team"
                message = "myFunction"
                telegram_bot_api_token = env.telegram_bot_api_token
            }
      "#},
      }
    };
}

pub struct TelegramSendMessage;
impl CommandImplementation for TelegramSendMessage {
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
        Ok(Actions::none()) // todo
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        _auth_ctx: &txtx_addon_kit::types::AuthorizationContext,
    ) -> CommandExecutionFutureResult {
        let args = values.clone();

        let telegram_chat_id: i64 =
            args.get_expected_integer("telegram_chat_id")?.try_into().unwrap();
        let telegram_bot_api_token =
            args.get_expected_string("telegram_bot_api_token")?.to_string();
        let message = args.get_expected_string("message")?.to_string();

        let future = async move {
            let result = CommandExecutionResult::new();
            let bot = Bot::new(telegram_bot_api_token);
            let _ = bot.send_message(ChatId(telegram_chat_id), message).await;
            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
