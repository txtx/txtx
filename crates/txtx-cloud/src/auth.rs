use std::io::{Read, Write};

use serde::{Deserialize, Serialize};
use txtx_core::kit::reqwest;

use crate::get_env_var;

pub const AUTH_SERVICE_URL: &str = "AUTH_SERVICE_URL";
pub const AUTH_CALLBACK_PORT: u16 = 8081;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    pub access_token: String,
    pub refresh_token: String,
    pub user: AuthUser,
    pub exp: u64,
}

impl AuthConfig {
    pub fn new(access_token: String, refresh_token: String, user: AuthUser, exp: u64) -> Self {
        // let user: AuthUser = serde_json::from_str(&user).unwrap();
        Self { access_token, refresh_token, user, exp }
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

    /// Refresh the session by sending a POST request to the auth service with the refresh token.
    /// If the request is successful, the new auth config is written to the system config.
    pub async fn refresh_session(&self) -> Result<AuthConfig, String> {
        let client = reqwest::Client::new();
        let res = client
            .post(&format!("{}/refresh", get_env_var(AUTH_SERVICE_URL)))
            .json(&serde_json::json!({
                "refreshToken": &self.refresh_token,
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to send request to refresh session: {}", e))?;

        if res.status().is_success() {
            let res = res
                .json::<AuthConfig>()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;
            res.write_to_system_config()
                .map_err(|e| format!("Failed to write refreshed session to config: {}", e))?;
            return Ok(res);
        } else {
            let err = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Failed to refresh session: {}", err));
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("SystemTime before UNIX EPOCH")
            .as_secs() as i64;

        self.exp < now as u64
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUser {
    pub id: String,
    pub email: Option<String>,
    pub display_name: String,
}
