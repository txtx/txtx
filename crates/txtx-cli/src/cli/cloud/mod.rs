use super::{CloudCommand, Context};

pub mod auth;
pub mod gql;
pub mod login;
pub mod publish;

pub async fn handle_cloud_commands(
    cmd: &CloudCommand,
    buffer_stdin: Option<String>,
    ctx: &Context,
) -> Result<(), String> {
    match cmd {
        CloudCommand::Login(login_command) => login::handle_login_command(login_command, ctx).await,
        CloudCommand::Publish(publish_runbook) => {
            publish::handle_publish_command(publish_runbook, buffer_stdin, ctx).await
        }
    }
}
