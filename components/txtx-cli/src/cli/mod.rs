use clap::{Parser, Subcommand};
use hiro_system_kit::{self, Logger};
use std::process;

mod manuals;

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
    Check(CheckManuals),
    /// Inspect deployment protocol
    #[clap(name = "inspect", bin_name = "inspect")]
    Inspect(InspectManual),
    /// Inspect deployment protocol
    #[clap(name = "run", bin_name = "run")]
    Run(RunManual),
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct CheckManuals {
    /// Path to manifest (default to ./protocol.json)
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct InspectManual {
    /// Path to manifest (default to ./protocol.json)
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    /// Disable Terminal UI
    #[clap(long = "no-term-ui")]
    pub no_tui: bool,
    /// Path to manual root file
    // #[clap(long = "manual-path", conflicts_with = "manual")]
    // pub manual_path: Option<String>,
    /// Name of manual as indexed in protocol.json
    #[clap(long = "manual", conflicts_with = "manual-path")]
    pub manual: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct RunManual {
    /// Path to manifest (default to ./protocol.json)
    #[clap(long = "manifest-path")]
    pub manifest_path: Option<String>,
    /// Path to manual root file
    // #[clap(long = "manual-path", conflicts_with = "manual")]
    // pub manual_path: Option<String>,
    /// Name of manual as indexed in protocol.json
    #[clap(long = "manual", conflicts_with = "manual-path")]
    pub manual: Option<String>,
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
            manuals::handle_check_command(&cmd, ctx).await?;
        }
        Command::Inspect(cmd) => {
            manuals::handle_inspect_command(&cmd, ctx).await?;
        }
        Command::Run(cmd) => {
            manuals::handle_run_command(&cmd, ctx).await?;
        }
    }
    Ok(())
}
