use clap::{ArgAction, Parser, Subcommand};
use hiro_system_kit::{self, Logger};
use runbooks::DEFAULT_PORT_TXTX;
use std::process;

mod docs;
mod lsp;
mod runbooks;
mod snapshots;
mod templates;

#[derive(Clone)]
pub struct Context {
    pub logger: Option<Logger>,
    pub tracer: bool,
}

#[allow(dead_code)]
impl Context {
    pub fn empty() -> Context {
        Context {
            logger: None,
            tracer: false,
        }
    }

    pub fn try_log<F>(&self, closure: F)
    where
        F: FnOnce(&Logger),
    {
        if let Some(ref logger) = self.logger {
            closure(logger)
        }
    }

    pub fn expect_logger(&self) -> &Logger {
        self.logger.as_ref().unwrap()
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
enum Command {
    /// Inspect deployment protocol
    #[clap(name = "check", bin_name = "check")]
    Check(CheckRunbook),
    /// New Runbook
    #[clap(name = "new", bin_name = "new")]
    New(CreateRunbook),
    /// List Runbooks
    #[clap(name = "ls", bin_name = "ls")]
    List(ListRunbooks),
    /// Execute Runbook
    #[clap(name = "run", bin_name = "run")]
    Run(ExecuteRunbook),
    /// Execute Runbook
    #[clap(subcommand)]
    Snapshots(SnapshotCommand),
    /// Display Documentation
    #[clap(name = "docs", bin_name = "docs")]
    Docs(GetDocumentation),
    /// Start LSP
    #[clap(name = "lsp", bin_name = "lsp")]
    Lsp,
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
enum SnapshotCommand {
    /// Begin new snapshot
    #[clap(name = "begin", bin_name = "begin")]
    Begin(BeginSnapshot),
    /// New Runbook
    #[clap(name = "end", bin_name = "end")]
    Commit(CommitSnapshot),
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct BeginSnapshot {
    /// Path to the manifest
    #[arg(long = "manifest-file-path", short = 'm', default_value = "./txtx.yml")]
    pub manifest_path: String,
    /// Path to the snapshot
    #[arg(long = "snapshot-file-path", short = 's')]
    pub snapshot_path: String,
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct CommitSnapshot {
    /// Path to the manifest
    #[arg(long = "manifest-file-path", short = 'm', default_value = "./txtx.yml")]
    pub manifest_path: String,
    /// Path to the snapshot
    #[arg(long = "snapshot-file-path", short = 's')]
    pub snapshot_path: String,
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct CheckRunbook {
    /// Path to the manifest
    #[arg(long = "manifest-file-path", short = 'm', default_value = "./txtx.yml")]
    pub manifest_path: String,
    /// Name of the runbook as indexed in the txtx.yml, or the path of the .tx file to run
    pub runbook: String,
    /// Choose the environment variable to set from those configured in the txtx.yml
    #[arg(long = "env")]
    pub environment: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct GetDocumentation;

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct InspectRunbook {
    /// Path to the manifest
    #[arg(long = "manifest-file-path", short = 'm', default_value = "./txtx.yml")]
    pub manifest_path: String,
    /// Disable Terminal UI
    #[clap(long = "no-term-ui")]
    pub no_tui: bool,
    /// Path to runbook root file
    // #[clap(long = "runbook-path", conflicts_with = "runbook")]
    // pub runbook_path: Option<String>,
    /// Name of runbook as indexed in txtx.yml
    #[clap(long = "runbook")]
    pub runbook: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
#[command(group = clap::ArgGroup::new("execution_mode").multiple(false).args(["unsupervised", "web_console", "term_console"]).required(false))]
pub struct ExecuteRunbook {
    /// Path to the manifest
    #[arg(long = "manifest-file-path", short = 'm', default_value = "./txtx.yml")]
    pub manifest_path: String,
    /// Name of the runbook as indexed in the txtx.yml, or the path of the .tx file to run
    pub runbook: String,

    /// Execute the runbook without supervision
    #[arg(long = "unsupervised", short = 'u', action=ArgAction::SetTrue, group = "execution_mode")]
    pub unsupervised: bool,
    /// Execute the runbook with supervision via the browser UI (this is the default execution mode)
    #[arg(long = "browser", short = 'b', action=ArgAction::SetTrue, group = "execution_mode")]
    pub web_console: bool,
    /// Execute the runbook with supervision via the terminal console (coming soon)
    #[arg(long = "terminal", short = 't', action=ArgAction::SetTrue, group = "execution_mode")]
    pub term_console: bool,

    /// Set the port for hosting the web UI
    #[arg(long = "port", short = 'p', default_value = DEFAULT_PORT_TXTX )]
    pub port: u16,
    /// Choose the environment variable to set from those configured in the txtx.yml
    #[arg(long = "env")]
    pub environment: Option<String>,
    /// A set of inputs to use for batch processing
    #[arg(long = "input")]
    pub inputs: Vec<String>,

    /// Execute the Runbook even if the cached state suggests this Runbook has already been executed
    #[arg(long = "force", short = 'f')]
    pub force_execution: bool,
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct CreateRunbook {
    /// Path to the manifest
    #[arg(long = "manifest-file-path", short = 'm', default_value = "./txtx.yml")]
    pub manifest_path: String,
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct ListRunbooks {
    /// Path to the manifest
    #[arg(long = "manifest-file-path", short = 'm', default_value = "./txtx.yml")]
    pub manifest_path: String,
}

pub fn main() {
    let logger = hiro_system_kit::log::setup_logger();
    let _guard = hiro_system_kit::log::setup_global_logger(logger.clone());
    let ctx = Context {
        logger: Some(logger),
        tracer: false,
    };

    let opts: Opts = match Opts::try_parse() {
        Ok(opts) => opts,
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    };

    match hiro_system_kit::nestable_block_on(handle_command(opts, &ctx)) {
        Err(e) => {
            error!(ctx.expect_logger(), "{e}");
            std::thread::sleep(std::time::Duration::from_millis(500));
            process::exit(1);
        }
        Ok(_) => {}
    }
}

async fn handle_command(opts: Opts, ctx: &Context) -> Result<(), String> {
    match opts.command {
        Command::Check(cmd) => {
            runbooks::handle_check_command(&cmd, ctx).await?;
        }
        Command::Run(cmd) => {
            runbooks::handle_run_command(&cmd, ctx).await?;
        }
        Command::List(cmd) => {
            runbooks::handle_list_command(&cmd, ctx).await?;
        }
        Command::New(cmd) => {
            runbooks::handle_new_command(&cmd, ctx).await?;
        }
        Command::Docs(cmd) => {
            docs::handle_docs_command(&cmd, ctx).await?;
        }
        Command::Snapshots(SnapshotCommand::Begin(cmd)) => {
            snapshots::handle_begin_command(&cmd, ctx).await?;
        }
        Command::Snapshots(SnapshotCommand::Commit(cmd)) => {
            snapshots::handle_commit_command(&cmd, ctx).await?;
        }
        Command::Lsp => {
            lsp::run_lsp().await?;
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    fn parse_args(args: Vec<&str>) -> ExecuteRunbook {
        ExecuteRunbook::parse_from(args)
    }

    #[test]
    fn test_execute_runbook_default_values() {
        let args = vec!["txtx", "runbook"];
        let result = parse_args(args);
        assert_eq!(result.manifest_path, "./txtx.yml");
        assert_eq!(result.runbook, "runbook");
        assert_eq!(result.unsupervised, false);
        assert_eq!(result.web_console, false);
        assert_eq!(result.term_console, false);
        assert_eq!(result.port, 8488);
        assert_eq!(result.environment, None);
        assert!(result.inputs.is_empty());
    }

    #[test]
    fn test_unsupervised_mode() {
        let args = vec!["txtx", "runbook", "--unsupervised"];
        let result = parse_args(args);
        assert_eq!(result.unsupervised, true);
        assert_eq!(result.web_console, false);
        assert_eq!(result.term_console, false);
    }

    #[test]
    fn test_web_console_mode() {
        let args = vec!["txtx", "runbook", "--browser"];
        let result = parse_args(args);
        assert_eq!(result.unsupervised, false);
        assert_eq!(result.web_console, true);
        assert_eq!(result.term_console, false);
    }

    #[test]
    fn test_terminal_console_mode() {
        let args = vec!["txtx", "runbook", "--terminal"];
        let result = parse_args(args);
        assert_eq!(result.unsupervised, false);
        assert_eq!(result.web_console, false);
        assert_eq!(result.term_console, true);
    }

    #[test]
    fn test_port_setting() {
        let args = vec!["txtx", "runbook", "--port", "9090"];
        let result = parse_args(args);
        assert_eq!(result.port, 9090);
    }

    #[test]
    fn test_environment_setting() {
        let args = vec!["txtx", "runbook", "--env", "production"];
        let result = parse_args(args);
        assert_eq!(result.environment, Some(String::from("production")));
    }

    #[test]
    fn test_inputs_setting() {
        let args = vec!["txtx", "runbook", "--input", "input1", "--input", "input2"];
        let result = parse_args(args);
        assert_eq!(result.inputs, vec!["input1", "input2"]);
    }

    #[test_case("--unsupervised", "--browser")]
    #[test_case("--unsupervised", "--terminal")]
    #[test_case("--browser", "--terminal")]
    fn test_conflicting_arguments(arg1: &str, arg2: &str) {
        let args = vec!["txtx", "runbook", arg1, arg2];
        let thing = ExecuteRunbook::try_parse_from(args);
        let err = thing.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }
}
