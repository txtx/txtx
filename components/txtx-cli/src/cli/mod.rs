use clap::{ArgAction, Parser, Subcommand};
use hiro_system_kit::{self, Logger};
use std::process;

mod docs;
mod runbooks;
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
    Check(CheckRunbooks),
    /// New Runbook
    #[clap(name = "new", bin_name = "new")]
    New(CreateRunbook),
    /// List Runbooks
    #[clap(name = "ls", bin_name = "ls")]
    List(ListRunbooks),
    /// Execute Runbook
    #[clap(name = "run", bin_name = "run")]
    Run(ExecuteRunbook),
    /// Display Documentation
    #[clap(name = "docs", bin_name = "docs")]
    Docs(GetDocumentation),
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct CheckRunbooks {
    /// Path to the manifest
    #[arg(long = "manifest-file-path", short = 'm', default_value = "./txtx.yml")]
    pub manifest_path: String,
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
pub struct ExecuteRunbook {
    /// Path to the manifest
    #[arg(long = "manifest-file-path", short = 'm', default_value = "./txtx.yml")]
    pub manifest_path: String,
    /// Name of runbook as indexed in txtx.yml, or path of the .tx file to run
    pub runbook: String,
    /// Start Web Console
    #[clap(long = "web-console", short = 'w', action=ArgAction::SetTrue)]
    pub web_console: bool,
    /// Start Terminal Console
    #[clap(long = "term-console")]
    pub term_console: bool,
    /// Set the port for hosting the web UI
    #[arg(long = "port", short = 'p', default_value = DEFAULT_PORT_TXTX )]
    pub port: u16,
    pub environment: Option<String>,
    /// Choose environment variable set
    #[clap(long = "input")]
    pub inputs: Vec<String>,
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
    }
    Ok(())
}
