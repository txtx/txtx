// Note: In a real implementation, these would be public exports from txtx_cli
// For this demo, we'll use simplified versions

use error_stack::{Report, ResultExt};

#[derive(Debug)]
enum CliError {
    ManifestError,
    RunbookNotFound,
    AuthError,
    OutputError,
    EnvironmentError,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::ManifestError => write!(f, "Manifest file error"),
            CliError::RunbookNotFound => write!(f, "Runbook not found"),
            CliError::AuthError => write!(f, "Authentication failed"),
            CliError::OutputError => write!(f, "Output operation failed"),
            CliError::EnvironmentError => write!(f, "Environment configuration error"),
        }
    }
}

impl error_stack::Context for CliError {}

#[derive(Debug)]
struct ManifestInfo {
    path: String,
    expected_format: String,
}

#[derive(Debug)]
struct RunbookContext {
    runbook_name: String,
    manifest_path: String,
    environment: Option<String>,
}

#[derive(Debug)]
struct OutputInfo {
    destination: String,
    format: String,
    reason: String,
}

// Documentation attachment for demo
#[derive(Debug)]
struct Documentation {
    help: String,
    example: Option<String>,
    link: Option<String>,
}

trait CliErrorExt {
    fn with_manifest_info(self, path: &str, format: &str) -> Self;
    fn with_runbook_context(self, name: &str, manifest: &str, env: Option<String>) -> Self;
    fn with_output_info(self, dest: &str, format: &str, reason: &str) -> Self;
    fn with_documentation(self, help: &str) -> Self;
    fn with_example(self, example: &str) -> Self;
    fn with_link(self, link: &str) -> Self;
}

impl<T> CliErrorExt for Result<T, Report<CliError>> {
    fn with_manifest_info(self, path: &str, format: &str) -> Self {
        self.map_err(|e| e.attach(ManifestInfo {
            path: path.to_string(),
            expected_format: format.to_string(),
        }))
    }
    
    fn with_runbook_context(self, name: &str, manifest: &str, env: Option<String>) -> Self {
        self.map_err(|e| e.attach(RunbookContext {
            runbook_name: name.to_string(),
            manifest_path: manifest.to_string(),
            environment: env,
        }))
    }
    
    fn with_output_info(self, dest: &str, format: &str, reason: &str) -> Self {
        self.map_err(|e| e.attach(OutputInfo {
            destination: dest.to_string(),
            format: format.to_string(),
            reason: reason.to_string(),
        }))
    }
    
    fn with_documentation(self, help: &str) -> Self {
        self.map_err(|e| e.attach(Documentation {
            help: help.to_string(),
            example: None,
            link: None,
        }))
    }
    
    fn with_example(self, example: &str) -> Self {
        self.map_err(|e| {
            let mut doc = Documentation {
                help: String::new(),
                example: Some(example.to_string()),
                link: None,
            };
            if let Some(existing) = e.downcast_ref::<Documentation>() {
                doc.help = existing.help.clone();
                doc.link = existing.link.clone();
            }
            e.attach(doc)
        })
    }
    
    fn with_link(self, link: &str) -> Self {
        self.map_err(|e| {
            let mut doc = Documentation {
                help: String::new(),
                example: None,
                link: Some(link.to_string()),
            };
            if let Some(existing) = e.downcast_ref::<Documentation>() {
                doc.help = existing.help.clone();
                doc.example = existing.example.clone();
            }
            e.attach(doc)
        })
    }
}

fn main() {
    println!("\n🔍 CLI Error-Stack Demo\n");
    println!("This demo shows how error-stack improves CLI error messages for common scenarios.\n");
    
    // Demo 1: Manifest not found
    println!("1️⃣  Manifest Not Found:");
    println!("━━━━━━━━━━━━━━━━━━━━━━");
    demo_manifest_not_found();
    
    // Demo 2: Runbook not found
    println!("\n2️⃣  Runbook Not Found:");
    println!("━━━━━━━━━━━━━━━━━━━━━");
    demo_runbook_not_found();
    
    // Demo 3: Authentication required
    println!("\n3️⃣  Authentication Required:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━");
    demo_auth_required();
    
    // Demo 4: Output write failure
    println!("\n4️⃣  Output Write Failure:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━");
    demo_output_failure();
    
    // Demo 5: Environment configuration
    println!("\n5️⃣  Environment Configuration Error:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    demo_env_error();
}

fn demo_manifest_not_found() {
    let result = load_manifest("./missing/Txtx.toml");
    
    match result {
        Ok(_) => println!("✅ Success (unexpected)"),
        Err(error) => {
            println!("{:#?}", error);
            
            // Show how to extract manifest info
            if let Some(info) = error.downcast_ref::<ManifestInfo>() {
                println!("\n📁 Manifest Details:");
                println!("   Path: {}", info.path);
                println!("   Expected format: {}", info.expected_format);
            }
        }
    }
}

fn demo_runbook_not_found() {
    let result = find_runbook("./Txtx.toml", "deploy-mainnet", Some("production"));
    
    match result {
        Ok(_) => println!("✅ Success (unexpected)"),
        Err(error) => {
            println!("{:#?}", error);
            
            if let Some(ctx) = error.downcast_ref::<RunbookContext>() {
                println!("\n🎯 Context:");
                println!("   Looking for: {}", ctx.runbook_name);
                println!("   In manifest: {}", ctx.manifest_path);
                if let Some(env) = &ctx.environment {
                    println!("   Environment: {}", env);
                }
            }
        }
    }
}

fn demo_auth_required() {
    let result = check_cloud_auth();
    
    match result {
        Ok(_) => println!("✅ Success (unexpected)"),
        Err(error) => {
            println!("{:#?}", error);
        }
    }
}

fn demo_output_failure() {
    let result = write_outputs("/read-only/output.json", "JSON data");
    
    match result {
        Ok(_) => println!("✅ Success (unexpected)"),
        Err(error) => {
            println!("{:#?}", error);
            
            if let Some(info) = error.downcast_ref::<OutputInfo>() {
                println!("\n📝 Output Operation:");
                println!("   Destination: {}", info.destination);
                println!("   Format: {}", info.format);
                println!("   Reason: {}", info.reason);
            }
        }
    }
}

fn demo_env_error() {
    let result = get_required_env("INFURA_API_KEY");
    
    match result {
        Ok(_) => println!("✅ Success (unexpected)"),
        Err(error) => {
            println!("{:#?}", error);
        }
    }
}

// Helper functions that simulate CLI operations

fn load_manifest(path: &str) -> Result<(), Report<CliError>> {
    // Simulate manifest loading failure
    Err(Report::new(CliError::ManifestError))
        .attach_printable(format!("File not found: {}", path))
        .attach_printable("No such file or directory")
        .with_manifest_info(path, "TOML")
        .with_documentation("Ensure the manifest file exists and the path is correct")
        .with_example("txtx run deploy --manifest ./Txtx.toml")
        .with_link("https://docs.txtx.io/manifest-format")
}

fn find_runbook(
    manifest_path: &str, 
    runbook_name: &str,
    environment: Option<&str>
) -> Result<(), Report<CliError>> {
    let available = vec!["deploy", "setup", "test"];
    
    Err(Report::new(CliError::RunbookNotFound))
        .attach_printable(format!("No runbook named '{}'", runbook_name))
        .attach_printable(format!("Available runbooks: {}", available.join(", ")))
        .with_runbook_context(
            runbook_name, 
            manifest_path,
            environment.map(|e| e.to_string())
        )
        .with_documentation("The runbook name must match one defined in your manifest")
        .with_example(&format!("txtx run {}", available[0]))
}

fn check_cloud_auth() -> Result<(), Report<CliError>> {
    Err(Report::new(CliError::AuthError))
        .attach_printable("Authentication required for cloud service actions")
        .attach_printable("You are not currently authenticated")
        .with_documentation("Cloud services require authentication to access remote resources")
        .with_example("txtx cloud login")
        .with_link("https://docs.txtx.io/cloud/authentication")
}

fn write_outputs(path: &str, _content: &str) -> Result<(), Report<CliError>> {
    Err(Report::new(CliError::OutputError))
        .attach_printable("Permission denied")
        .with_output_info(
            path,
            "JSON",
            "Cannot write to read-only directory"
        )
        .with_documentation("Ensure the output directory exists and you have write permissions")
        .with_example("txtx run deploy --output ./outputs/result.json")
}

fn get_required_env(key: &str) -> Result<String, Report<CliError>> {
    Err(Report::new(CliError::EnvironmentError))
        .attach_printable(format!("Required environment variable '{}' not set", key))
        .with_documentation(match key {
            "INFURA_API_KEY" => "Get your Infura API key from https://infura.io/dashboard",
            "ETHERSCAN_API_KEY" => "Get your API key from https://etherscan.io/apis",
            _ => "Set this variable in your .env file or shell environment",
        })
        .with_example(&format!("export {}=your_key_here", key))
        .with_link("https://docs.txtx.io/configuration/environment")
}