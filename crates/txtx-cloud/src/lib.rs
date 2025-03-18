use clap::{Parser, Subcommand, ValueEnum};
use dotenvy::dotenv;

pub mod auth;
pub mod gql;
pub mod login;
pub mod publish;

pub fn get_env_var(key: &str) -> String {
    dotenv().ok();
    std::env::var(key).unwrap_or_else(|_| panic!("{} not found", key))
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
pub enum CloudCommand {
    #[clap(name = "login", bin_name = "login")]
    Login(LoginCommand),
    /// Publish a runbook to the cloud, allowing it to be called by other runbooks.
    /// In order to package the runbook for publishing, it will be simulated, and thus requires all required inputs to be provided.
    /// However, the published runbook will have the inputs removed.
    #[clap(name = "publish", bin_name = "publish")]
    Publish(PublishRunbook),
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct LoginCommand {
    /// The username to use for authentication
    #[arg(long = "email", short = 'e', requires = "password", conflicts_with = "pat")]
    pub email: Option<String>,

    /// The password to use for authentication
    #[arg(long = "password", short = 'p', requires = "email", conflicts_with = "pat")]
    pub password: Option<String>,

    /// Automatically log in using a Personal Access Token
    #[arg(long = "pat", conflicts_with_all = &["email", "password"])]
    pub pat: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
pub struct PublishRunbook {
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
    /// The destination to publish the runbook to. By default, the published runbook will be at /manifest/path/<runbook-id>.output.json
    #[arg(long = "destination", short = 'd')]
    pub destination: Option<String>,
    /// The permissions to set for what users can read the runbook.
    ///  - `public`: Anyone can read the runbook
    ///  - `private`: Only the owner can read the runbook
    ///  - `org`: Only members of the organization can read the runbook
    #[arg(long = "read-permissions", default_value = "private")]
    pub read_permissions: Option<PublishRunbookReadPermissions>,
    /// The permissions to set for what users can update the runbook.
    ///  - `private`: Only the owner can update the runbook
    ///  - `org`: Only members of the organization can update the runbook
    #[arg(long = "update-permissions", default_value = "private")]
    pub update_permissions: Option<PublishRunbookWritePermissions>,
    /// The permissions to set for what users can delete the runbook.
    ///  - `private`: Only the owner can delete the runbook
    ///  - `org`: Only members of the organization can delete the runbook
    #[arg(long = "delete-permissions", default_value = "private")]
    pub delete_permissions: Option<PublishRunbookWritePermissions>,
}

#[derive(ValueEnum, PartialEq, Clone, Debug)]
#[clap(rename_all = "snake-case")]
pub enum PublishRunbookReadPermissions {
    Public,
    Private,
    Org,
}

#[derive(ValueEnum, PartialEq, Clone, Debug)]
#[clap(rename_all = "snake-case")]
pub enum PublishRunbookWritePermissions {
    Private,
    Org,
}
