use std::io::{Read, Write};

use serde::{Deserialize, Serialize};
use txtx_core::kit::reqwest;

use crate::get_env_var;

pub const AUTH_SERVICE_URL: &str = "https://auth.txtx.run";
pub const AUTH_CALLBACK_PORT: u16 = 8488;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    pub pat: String,
    pub user: AuthUser,
}

impl AuthConfig {
    pub fn new(pat: String, user: AuthUser) -> Self {
        // let user: AuthUser = serde_json::from_str(&user).unwrap();
        Self { pat, user }
    }

    /// Write auth config to system data directory.
    pub fn write_to_system_config(&self) -> Result<(), String> {
        let data_dir = dirs::data_dir().ok_or("Failed to get system data directory")?;

        std::fs::create_dir_all(data_dir.join("txtx"))
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        let path = data_dir.join("txtx/auth.toml");

        let mut file = std::fs::File::create(&path)
            .map_err(|e| format!("Failed to create config file: {}", e))?;

        let toml = toml::to_string(&self)
            .map_err(|e| format!("Failed to serialize auth config: {}", e))?;

        file.write_all(toml.as_bytes())
            .map_err(|e| format!("Failed to write auth config: {}", e))?;
        Ok(())
    }

    /// Read auth config from system data directory.
    pub fn read_from_system_config() -> Result<Option<Self>, String> {
        let data_dir = dirs::data_dir().ok_or("Failed to get system data directory")?;
        let path = data_dir.join("txtx/auth.toml");

        if !path.exists() {
            return Ok(None);
        }

        let mut file =
            std::fs::File::open(&path).map_err(|e| format!("Failed to open config file: {}", e))?;
        let mut buf = String::new();

        file.read_to_string(&mut buf).map_err(|e| format!("Failed to read config file: {}", e))?;

        let config =
            toml::from_str(&buf).map_err(|e| format!("Failed to parse auth config file: {}", e))?;
        Ok(Some(config))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUser {
    pub id: String,
    pub email: Option<String>,
    pub display_name: String,
}
