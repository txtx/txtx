use std::sync::Arc;

use actix_web::error::ErrorInternalServerError;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::HttpResponseBuilder;
use actix_web::{HttpRequest, HttpResponse};
use dotenvy_macro::dotenv;
use tokio::sync::RwLock;
use txtx_core::kit::channel::Sender;
use txtx_core::kit::reqwest;
use txtx_core::kit::types::frontend::{ActionItemResponse, BlockEvent};

const RELAYER_BASE_URL: &str = dotenv!("RELAYER_BASE_URL");

#[derive(Clone, Debug)]
pub struct RelayerContext {
    pub channel: Arc<RwLock<Option<ChannelData>>>,
    pub action_item_events_tx: Sender<ActionItemResponse>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelData {
    pub operator_token: String,
    pub totp: String,
    pub http_endpoint_url: String,
    pub ws_endpoint_url: String,
    pub slug: String,
}
impl ChannelData {
    pub fn new(operator_token: String, open_channel_response: OpenChannelResponse) -> Self {
        ChannelData {
            operator_token,
            totp: open_channel_response.totp,
            http_endpoint_url: open_channel_response.http_endpoint_url,
            ws_endpoint_url: open_channel_response.ws_endpoint_url,
            slug: open_channel_response.slug,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename = "runbook", rename_all = "camelCase")]
pub struct OpenChannelRequest {
    pub name: String,
    pub description: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenChannelResponse {
    pub totp: String,
    pub http_endpoint_url: String,
    pub ws_endpoint_url: String,
    pub slug: String,
}

#[actix_web::post("/relayer/channels")]
pub async fn open_channel(
    req: HttpRequest,
    payload: Json<OpenChannelRequest>,
    relayer_context: Data<RelayerContext>,
) -> actix_web::Result<HttpResponse> {
    let Some(cookie) = req.cookie("hanko") else {
        return Ok(HttpResponse::Unauthorized().body("No auth data provided"));
    };

    let token = cookie.value();
    let client = reqwest::Client::new();
    let path = format!("{}/api/v1/channels", RELAYER_BASE_URL);

    let res = client
        .post(path)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(ErrorInternalServerError)?;

    let body = res
        .json::<OpenChannelResponse>()
        .await
        .map_err(ErrorInternalServerError)?;

    let mut channel = relayer_context.channel.write().await;
    *channel = Some(ChannelData::new(token.to_string(), body.clone()));

    Ok(HttpResponseBuilder::new(StatusCode::OK).json(body))
}

pub async fn forward_block_event(token: String, payload: BlockEvent) -> Result<(), String> {
    let client = reqwest::Client::new();
    let path = format!("{}/gql/v1/mutations", RELAYER_BASE_URL);

    let _ = client
        .post(path)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("error forwarding block event to relayer: {}", e.to_string()))?
        .error_for_status()
        .map_err(|e| format!("received error response from relayer: {}", e.to_string()))?;

    Ok(())
}
