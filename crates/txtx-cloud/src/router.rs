use std::{future::Future, pin::Pin};

use serde_json::Value as JsonValue;
use txtx_addon_kit::{
    reqwest::{self, Client},
    types::cloud_interface::{
        AuthenticatedCloudServiceRouter, CloudService, DeploySubgraphCommand, SvmService,
    },
};

use crate::auth::AuthConfig;

#[derive(Debug, Clone)]
pub struct TxtxAuthenticatedCloudServiceRouter {
    id_service_url: String,
}

impl AuthenticatedCloudServiceRouter for TxtxAuthenticatedCloudServiceRouter {
    fn route<'a>(
        &'a self,
        service: CloudService,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            let token_required = service.token_required();
            let access_token = if token_required {
                let Some(auth_config) = AuthConfig::read_from_system_config()? else {
                    return Err("You must be logged in to use txtx cloud services. Run `txtx cloud login` to log in.".to_string());
                };
                auth_config
                    .refresh_session_if_needed(&self.id_service_url, &auth_config.pat)
                    .await
                    .map_err(|e| e.to_string())?;
                Some(auth_config.access_token)
            } else {
                None
            };
            TxtxCloudServiceRouter::new(service).route(access_token).await
        })
    }
}

impl TxtxAuthenticatedCloudServiceRouter {
    pub fn new(id_service_url: &str) -> Self {
        Self { id_service_url: id_service_url.to_string() }
    }
}

#[derive(Debug, Clone)]
pub struct TxtxCloudServiceRouter {
    pub service: CloudService,
}

impl TxtxCloudServiceRouter {
    async fn route(self, token: Option<String>) -> Result<String, String> {
        match &self.service {
            CloudService::Registry => {
                // Handle registry service
            }
            CloudService::Id => {
                // Handle ID service
            }
            CloudService::Svm(svm_service) => match svm_service {
                SvmService::DeploySubgraph(DeploySubgraphCommand {
                    url,
                    params,
                    do_include_token,
                }) => {
                    let token = if *do_include_token { token } else { None };
                    let client = Client::new();

                    let res =
                        rpc_call::<JsonValue>(&client, url, "loadPlugin", params, token.as_ref())
                            .await
                            .map_err(|e| {
                                format!("Failed to send request to deploy subgraph: {}", e)
                            })?;

                    return Ok(res.to_string());
                }
            },
            CloudService::Evm => {
                // Handle EVM service
            }
        }
        Ok("".into())
    }
}

impl TxtxCloudServiceRouter {
    pub fn new(service: CloudService) -> Self {
        Self { service }
    }
}

async fn rpc_call<T: for<'de> serde::Deserialize<'de> + std::convert::From<JsonValue>>(
    client: &reqwest::Client,
    url: &str,
    method: &str,
    params: &JsonValue,
    token: Option<&String>,
) -> Result<T, Box<dyn std::error::Error>> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1, //todo
    });
    let mut req = client.post(url).json(&body);
    if let Some(token) = token {
        req = req.bearer_auth(token);
    }
    let resp = req.send().await?.json::<JsonValue>().await?;

    Ok(resp["result"].clone().try_into()?)
}
