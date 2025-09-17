use atty::Stream;
use clap::{ArgAction, Parser, Subcommand};
use dotenvy::dotenv;
use env::TxtxEnv;
use hiro_system_kit::{self, Logger};
use runbooks::load_runbook_from_manifest;
use std::process;
use txtx_cloud::{LoginCommand, PublishRunbook};

mod common;
mod docs;
mod doctor;
mod env;
mod lsp;
mod runbooks;
mod snapshots;

/// Parse a single key-value pair
fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s.find('=').ok_or_else(|| format!("invalid KEY=VALUE: no '=' found in '{}'", s))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

pub const AUTH_SERVICE_URL_KEY: &str = "AUTH_SERVICE_URL";
pub const AUTH_CALLBACK_PORT_KEY: &str = "AUTH_CALLBACK_PORT";
pub const TXTX_CONSOLE_URL_KEY: &str = "TXTX_CONSOLE_URL";
pub const TXTX_ID_SERVICE_URL_KEY: &str = "TXTX_ID_SERVICE_URL";
pub const REGISTRY_GQL_URL_KEY: &str = "REGISTRY_GQL_URL";

pub const DEFAULT_AUTH_SERVICE_URL: &str = "https://auth.txtx.run";
pub const DEFAULT_AUTH_CALLBACK_PORT: u16 = 8488;
pub const DEFAULT_TXTX_CONSOLE_URL: &str = "https://txtx.run";
pub const DEFAULT_TXTX_ID_SERVICE_URL: &str = "https://id.gql.txtx.run/v1";
pub const DEFAULT_REGISTRY_GQL_URL: &str = "https://registry.gql.txtx.run/v1";

#[derive(Clone)]
pub struct Context {
    pub logger: Option<Logger>,
    pub tracer: bool,
}

#[allow(dead_code)]
impl Context {
    pub fn empty() -> Context {
        Context { logger: None, tracer: false }
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
    /// List the runbooks indexed in the txtx manifest
    #[clap(name = "ls", bin_name = "ls")]
    List(ListRunbooks),
    /// Create a new runbook
    #[clap(name = "new", bin_name = "new")]
    New(CreateRunbook),
    /// Check a runbook against a previous execution's statefile to list which actions will be re-executed
    #[clap(name = "check", bin_name = "check")]
    Check(CheckRunbook),
    /// Execute a runbook. Run, runbook, run!
    #[clap(name = "run", bin_name = "run")]
    Run(ExecuteRunbook),
    /// Display documentation
    #[clap(name = "docs", bin_name = "docs")]
    Docs(GetDocumentation),
    /// Start the txtx language server
    #[clap(name = "lsp", bin_name = "lsp")]
    Lsp(LspCommand),
    /// Start a server to listen for requests to execute runbooks
    #[clap(name = "serve", bin_name = "serve")]
    #[cfg(feature = "txtx_serve")]
    Serve(StartServer),
    /// Snapshot management (work in progress)
    #[clap(subcommand)]
    Snapshots(SnapshotCommand),
    /// Txtx cloud commands
    #[clap(subcommand, name = "cloud", bin_name = "cloud")]
    Cloud(CloudCommand),
    /// Diagnose issues with runbook configuration
    #[clap(name = "doctor", bin_name = "doctor")]
    Doctor(DoctorCommand),
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
enum SnapshotCommand {
    /// Begin new snapshot
    #[clap(name = "begin", bin_name = "begin")]
    Begin(BeginSnapshot),
    /// Finalize snapshot
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
    /// A set of inputs to use for batch processing
    #[arg(long = "input")]
    pub inputs: Vec<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct GetDocumentation;

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct LspCommand {
    /// Start the language server in stdio mode (this flag is accepted for compatibility but has no effect as stdio is the default)
    #[arg(long = "stdio")]
    pub stdio: bool,
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct DoctorCommand {
    /// Path to the manifest
    #[arg(long = "manifest-file-path", short = 'm', default_value = "./txtx.yml")]
    pub manifest_path: Option<String>,
    /// Specific runbook to validate (validates all if not specified)
    pub runbook: Option<String>,
    /// Choose the environment variables to validate against from those configured in the txtx.yml
    #[arg(long = "env", short = 'e')]
    pub environment: Option<String>,
    /// Input variable overrides (format: name=value)
    #[arg(long = "input", short = 'i', value_parser = parse_key_val)]
    pub inputs: Vec<(String, String)>,
    /// Output format (auto, pretty, quickfix, json)
    #[arg(long = "format", short = 'f', default_value = "auto", value_enum)]
    pub format: DoctorOutputFormat,
}

#[derive(clap::ValueEnum, PartialEq, Clone, Debug)]
pub enum DoctorOutputFormat {
    /// Auto-detect based on output context
    Auto,
    /// Human-readable output with colors and context
    Pretty,
    /// Single-line format for editor integration
    Quickfix,
    /// Machine-readable JSON format
    Json,
}

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
    /// When running in unsupervised mode, print outputs in JSON format. If a directory is provided, the output will be written a file at the directory.
    #[arg(long = "output-json")]
    pub output_json: Option<Option<String>>,
    /// Pick a specific output to stdout at the end of the execution
    #[arg(long = "output", conflicts_with = "output_json")]
    pub output: Option<String>,
    /// Explain how the runbook will be executed.
    #[arg(long = "explain", action=ArgAction::SetTrue)]
    pub explain: bool,
    /// Set the port for hosting the web UI
    #[arg(long = "port", short = 'p', default_value = txtx_supervisor_ui::DEFAULT_BINDING_PORT )]
    #[cfg(feature = "supervisor_ui")]
    pub network_binding_port: u16,
    /// Set the port for hosting the web UI
    #[arg(long = "ip", short = 'i', default_value = txtx_supervisor_ui::DEFAULT_BINDING_ADDRESS )]
    #[cfg(feature = "supervisor_ui")]
    pub network_binding_ip_address: String,
    /// Choose the environment variable to set from those configured in the txtx.yml
    #[arg(long = "env")]
    pub environment: Option<String>,
    /// A set of inputs to use for batch processing
    #[arg(long = "input")]
    pub inputs: Vec<String>,

    /// Execute the Runbook even if the cached state suggests this Runbook has already been executed
    #[arg(long = "force", short = 'f')]
    pub force_execution: bool,
    /// The log level to use for the runbook execution. Options are "trace", "debug", "info", "warn", "error".
    #[arg(long = "log-level", short = 'l', default_value = "info")]
    pub log_level: String,
}

impl ExecuteRunbook {
    pub fn do_start_supervisor_ui(&self) -> bool {
        self.web_console || (!self.unsupervised && !self.term_console)
    }
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

#[derive(Parser, PartialEq, Clone, Debug)]
#[cfg(feature = "txtx_serve")]
pub struct StartServer {
    /// Serve runbooks from a specific project
    #[arg(long = "manifest-file-path", short = 'm', default_value = "./txtx.yml")]
    pub manifest_path: Option<String>,
    /// When running in unsupervised mode, print outputs in JSON format
    #[arg(long = "output-json", action=ArgAction::SetTrue)]
    pub output_json: bool,
    /// Pick a specific output to stdout at the end of the execution
    #[arg(long = "output", conflicts_with = "output_json")]
    pub output: Option<String>,
    /// Set the port for hosting the web UI
    #[arg(long = "port", short = 'p', default_value = txtx_serve::SERVE_BINDING_PORT )]
    pub network_binding_port: u16,
    /// Set the port for hosting the web UI
    #[arg(long = "ip", short = 'i', default_value = txtx_serve::SERVE_BINDING_ADDRESS )]
    pub network_binding_ip_address: String,
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
pub enum CloudCommand {
    /// Login to the Txtx Cloud
    #[clap(name = "login", bin_name = "login")]
    Login(LoginCommand),
    /// Publish a runbook to the cloud, allowing it to be called by other runbooks.
    /// In order to package the runbook for publishing, it will be simulated, and thus requires all required inputs to be provided.
    /// However, the published runbook will have the inputs removed.
    #[clap(name = "publish", bin_name = "publish")]
    Publish(PublishRunbook),
}

fn load_stdin() -> Option<String> {
    if atty::is(Stream::Stdin) {
        return None;
    }
    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer).ok()?;
    return Some(buffer);
}

pub fn main() {
    let logger = hiro_system_kit::log::setup_logger();
    let _guard = hiro_system_kit::log::setup_global_logger(logger.clone());
    let ctx = Context { logger: Some(logger), tracer: false };

    let opts: Opts = match Opts::try_parse() {
        Ok(opts) => opts,
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    };

    // Special case for LSP - it runs its own synchronous loop
    if let Command::Lsp(_) = opts.command {
        match lsp::run_lsp() {
            Err(e) => {
                eprintln!("LSP server error: {}", e);
                process::exit(1);
            }
            Ok(_) => return,
        }
    }

    let buffer_stdin = load_stdin();

    match hiro_system_kit::nestable_block_on(handle_command(opts, &ctx, buffer_stdin)) {
        Err(e) => {
            error!(ctx.expect_logger(), "{e}");
            std::thread::sleep(std::time::Duration::from_millis(500));
            process::exit(1);
        }
        Ok(_) => {}
    }
}

async fn handle_command(
    opts: Opts,
    ctx: &Context,
    buffer_stdin: Option<String>,
) -> Result<(), String> {
    dotenv().ok();
    let env = TxtxEnv::load();
    match opts.command {
        Command::Check(cmd) => {
            runbooks::handle_check_command(&cmd, buffer_stdin, ctx, &env).await?;
        }
        Command::Run(cmd) => {
            runbooks::handle_run_command(&cmd, buffer_stdin, ctx, &env).await?;
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
        Command::Lsp(_lsp_cmd) => {
            // This case is handled before entering the async runtime
            unreachable!("LSP command should be handled synchronously");
        }
        #[cfg(feature = "txtx_serve")]
        Command::Serve(cmd) => {
            warn!(
                ctx.expect_logger(),
                "The command `txtx serve` is experimental and will run for 30 minutes."
            );
            let addr = format!("{}:{}", cmd.network_binding_ip_address, cmd.network_binding_port);
            let _ = txtx_serve::start_server(&addr).await.unwrap();
            ctrlc::set_handler(move || {
                std::process::exit(1);
            })
            .expect("Error setting Ctrl-C handler");
            // Consider making the duration configurable or running indefinitely
            thread::sleep(std::time::Duration::new(1800, 0));
        }
        Command::Cloud(cmd) => handle_cloud_commands(&cmd, buffer_stdin, &env).await?,
        Command::Doctor(cmd) => {
            use doctor::run_doctor;
            run_doctor(
                cmd.manifest_path.clone(),
                cmd.runbook.clone(),
                cmd.environment.clone(),
                cmd.inputs.clone(),
                cmd.format.clone(),
            )?;
        }
    }
    Ok(())
}

async fn handle_cloud_commands(
    cmd: &CloudCommand,
    buffer_stdin: Option<String>,
    env: &TxtxEnv,
) -> Result<(), String> {
    match cmd {
        CloudCommand::Login(cmd) => {
            txtx_cloud::login::handle_login_command(
                cmd,
                &env.auth_service_url,
                &env.auth_callback_port,
                &env.id_service_url,
            )
            .await
        }
        CloudCommand::Publish(cmd) => {
            let (_manifest, _runbook_name, runbook, _runbook_state) = load_runbook_from_manifest(
                &cmd.manifest_path,
                &cmd.runbook,
                &cmd.environment,
                &cmd.inputs,
                buffer_stdin,
                env,
            )
            .await?;

            txtx_cloud::publish::handle_publish_command(
                cmd,
                runbook,
                &env.id_service_url,
                &env.txtx_console_url,
                &env.registry_gql_url,
            )
            .await
        }
    }
}

pub fn get_env_var<T: ToString>(key: &str, default: T) -> String {
    dotenv().ok();
    std::env::var(key).unwrap_or(default.to_string())
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
        #[cfg(feature = "supervisor_ui")]
        assert_eq!(result.network_binding_port, 8488);
        #[cfg(feature = "supervisor_ui")]
        assert_eq!(result.network_binding_ip_address, "localhost");
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
    #[cfg(feature = "supervisor_ui")]
    fn test_port_setting() {
        let args = vec!["txtx", "runbook", "--port", "9090"];
        let result = parse_args(args);
        assert_eq!(result.network_binding_port, 9090);
    }

    #[test]
    #[cfg(feature = "supervisor_ui")]
    fn test_ip_setting() {
        let args = vec!["txtx", "runbook", "--ip", "192.168.1.10"];
        let result = parse_args(args);
        assert_eq!(result.network_binding_ip_address, "192.168.1.10");
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
