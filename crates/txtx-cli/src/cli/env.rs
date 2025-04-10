use dotenvy::dotenv;

pub const AUTH_SERVICE_URL_KEY: &str = "AUTH_SERVICE_URL";
pub const AUTH_CALLBACK_PORT_KEY: &str = "AUTH_CALLBACK_PORT";
pub const TXTX_CONSOLE_URL_KEY: &str = "TXTX_CONSOLE_URL";
pub const TXTX_ID_SERVICE_URL_KEY: &str = "TXTX_ID_SERVICE_URL";
pub const REGISTRY_GQL_URL_KEY: &str = "REGISTRY_GQL_URL";

pub const DEFAULT_AUTH_SERVICE_URL: &str = "https://auth.txtx.run";
pub const DEFAULT_AUTH_CALLBACK_PORT: u16 = 8488;
pub const DEFAULT_TXTX_CONSOLE_URL: &str = "https://txtx.run";
pub const DEFAULT_TXTX_ID_SERVICE_URL: &str = "https://id.txtx.run/v1";
pub const DEFAULT_REGISTRY_GQL_URL: &str = "https://registry.gql.txtx.run/v1";

pub fn get_env_var<T: ToString>(key: &str, default: T) -> String {
    dotenv().ok();
    std::env::var(key).unwrap_or(default.to_string())
}

#[derive(Debug, Clone)]
pub struct TxtxEnv {
    pub auth_service_url: String,
    pub auth_callback_port: String,
    pub txtx_console_url: String,
    pub id_service_url: String,
    pub registry_gql_url: String,
}

impl TxtxEnv {
    pub fn load() -> Self {
        let auth_service_url = get_env_var(AUTH_SERVICE_URL_KEY, DEFAULT_AUTH_SERVICE_URL);
        let auth_callback_port = get_env_var(AUTH_CALLBACK_PORT_KEY, DEFAULT_AUTH_CALLBACK_PORT);
        let txtx_console_url = get_env_var(TXTX_CONSOLE_URL_KEY, DEFAULT_TXTX_CONSOLE_URL);
        let txtx_id_service_url = get_env_var(TXTX_ID_SERVICE_URL_KEY, DEFAULT_TXTX_ID_SERVICE_URL);
        let registry_gql_url = get_env_var(REGISTRY_GQL_URL_KEY, DEFAULT_REGISTRY_GQL_URL);

        Self {
            auth_service_url,
            auth_callback_port,
            txtx_console_url,
            id_service_url: txtx_id_service_url,
            registry_gql_url,
        }
    }
}
