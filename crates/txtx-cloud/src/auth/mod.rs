pub mod jwt;

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use txtx_core::kit::reqwest;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    pub access_token: String,
    pub exp: u64,
    pub refresh_token: String,
    pub pat: Option<String>,
    pub user: AuthUser,
}

impl AuthConfig {
    pub fn new(
        access_token: String,
        exp: u64,
        refresh_token: String,
        pat: Option<String>,
        user: AuthUser,
    ) -> Self {
        Self { access_token, exp, refresh_token, pat, user }
    }

    async fn from_refresh_session_response(
        id_service_url: &str,
        RefreshSessionResponse { access_token, refresh_token, user }: &RefreshSessionResponse,
        pat: &Option<String>,
    ) -> Result<Self, String> {
        let jwt_manager = jwt::JwtManager::initialize(id_service_url)
            .await
            .map_err(|e| format!("Failed to initialize JWT manager: {}", e))?;

        let access_token_claims = jwt_manager
            .decode_jwt(access_token, true)
            .map_err(|e| format!("Failed to decode access token: {}", e))?;

        Ok(Self {
            access_token: access_token.clone(),
            exp: access_token_claims.exp,
            refresh_token: refresh_token.clone(),
            pat: pat.clone(),
            user: user.clone(),
        })
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

    /// Get a new access token by sending a POST request to the auth service with the refresh token.
    /// If the request is successful, the new auth config is written to the system config.
    pub async fn refresh_session(
        &self,
        id_service_url: &str,
        pat: &Option<String>,
    ) -> Result<AuthConfig, String> {
        let client = reqwest::Client::new();
        let res = client
            .post(&format!("{id_service_url}/token"))
            .json(&serde_json::json!({
                "refreshToken": &self.refresh_token,
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to send request to refresh session: {}", e))?;

        if res.status().is_success() {
            let res = res
                .json::<RefreshSessionResponse>()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            let auth_config = AuthConfig::from_refresh_session_response(id_service_url, &res, pat)
                .await
                .map_err(|e| format!("Failed to parse refresh session response: {e}"))?;

            auth_config
                .write_to_system_config()
                .map_err(|e| format!("Failed to write refreshed session to config: {}", e))?;
            return Ok(auth_config);
        } else {
            let err = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Failed to refresh session: {}", err));
        }
    }

    pub fn is_access_token_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("SystemTime before UNIX EPOCH")
            .as_secs() as i64;

        self.exp < now as u64
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshSessionResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: AuthUser,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthUser {
    pub id: String,
    pub email: Option<String>,
    pub display_name: String,
}
