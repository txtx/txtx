use actix_web::error::ErrorInternalServerError;
use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::HttpResponseBuilder;
use actix_web::{HttpRequest, HttpResponse};
use dotenvy_macro::dotenv;
use native_tls::TlsConnector;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::connect_async_tls_with_config;
use tokio_tungstenite::tungstenite::handshake::client::{generate_key, Request};
use tokio_tungstenite::tungstenite::Message;
use totp_rs::{Algorithm, TOTP};
use txtx_core::kit::channel::Sender;
use txtx_core::kit::futures::{SinkExt, StreamExt};
use txtx_core::kit::reqwest::{self};
use txtx_core::kit::types::frontend::{
    ActionItemResponse, BlockEvent, OpenChannelRequest, OpenChannelResponse,
    OpenChannelResponseBrowser,
};
use txtx_core::kit::uuid::Uuid;
use txtx_gql::Context as GraphContext;

const RELAYER_BASE_URL: &str = dotenv!("RELAYER_BASE_URL");
const RELAYER_HOST: &str = dotenv!("RELAYER_HOST");

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
    pub fn new(
        operator_token: String,
        totp: String,
        slug: String,
        open_channel_response: OpenChannelResponse,
    ) -> Self {
        ChannelData {
            operator_token,
            totp: totp,
            http_endpoint_url: open_channel_response.http_endpoint_url,
            ws_endpoint_url: open_channel_response.ws_endpoint_url,
            slug: slug,
        }
    }
}

pub async fn open_channel(
    req: HttpRequest,
    relayer_context: Data<RelayerContext>,
    graph_context: Data<GraphContext>,
) -> actix_web::Result<HttpResponse> {
    println!("POST /api/v1/channels");
    let Some(cookie) = req.cookie("hanko") else {
        return Ok(HttpResponse::Unauthorized().body("No auth data provided"));
    };

    let token = cookie.value();
    let client = reqwest::Client::new();
    let path = format!("{}/api/v1/channels", RELAYER_BASE_URL);

    let totp = auth_token_to_totp(token).get_secret_base32();
    let uuid = Uuid::new_v4();

    use base58::ToBase58;
    let slug = uuid.as_bytes().to_base58()[0..8].to_string();

    let block_store = graph_context.block_store.read().await.clone();
    let payload = OpenChannelRequest {
        runbook_name: graph_context.runbook_name.clone(),
        runbook_description: graph_context.runbook_description.clone(),
        block_store: block_store.clone(),
        uuid: uuid.clone(),
        slug: slug.clone(),
        operating_token: token.to_string(),
        totp: totp.clone(),
    };

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
    *channel = Some(ChannelData::new(
        token.to_string(),
        totp.clone(),
        slug.clone(),
        body.clone(),
    ));

    let response = OpenChannelResponseBrowser {
        totp: totp.clone(),
        http_endpoint_url: body.http_endpoint_url,
        ws_endpoint_url: body.ws_endpoint_url,
        slug: slug.clone(),
    };
    Ok(HttpResponseBuilder::new(StatusCode::OK).json(response))
}

fn auth_token_to_totp(token: &str) -> TOTP {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let hashed_auth_token = hasher.finalize();
    TOTP::new(Algorithm::SHA256, 6, 1, 60, hashed_auth_token.to_vec()).unwrap()
}

pub async fn forward_block_event(
    token: String,
    slug: String,
    payload: BlockEvent,
) -> Result<(), String> {
    let path = format!("{}/api/v1/channels/{}", RELAYER_BASE_URL, slug);

    let _ = request_with_retry(&path, &token, &payload)
        .await
        .map_err(|e| format!("failed to forward block event to relayer: {}", e))?;

    Ok(())
}

async fn request_with_retry<T>(
    path: &String,
    auth_token: &String,
    payload: &T,
) -> Result<(), String>
where
    T: Serialize + ?Sized,
{
    let max_attempts = 3;
    let mut attempts = 0;
    let client = reqwest::Client::new();
    loop {
        match client
            .post(path)
            .bearer_auth(&auth_token)
            .json(payload)
            .send()
            .await
        {
            Ok(req) => match req.error_for_status() {
                Ok(_) => return Ok(()),
                Err(e) => {
                    attempts = attempts + 1;
                    println!("retry attempt {}", attempts);
                    if max_attempts == 3 {
                        return Err(format!("failed to make request {} times: {}", attempts, e));
                    }
                }
            },
            Err(e) => {
                attempts = attempts + 1;
                println!("retry attempt {}", attempts);
                if max_attempts == 3 {
                    return Err(format!("failed to make request {} times: {}", attempts, e));
                }
            }
        };
    }
}

pub async fn get_opened_channel_data(
    relayer_channel: Arc<RwLock<Option<ChannelData>>>,
) -> ChannelData {
    let channel = loop {
        if let Some(channel) = relayer_channel.read().await.clone() {
            break channel;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    };
    channel
}

pub async fn process_relayer_ws_events(
    channel: ChannelData,
    action_item_events_tx: Sender<ActionItemResponse>,
) -> Result<(), String> {
    let req = Request::builder()
        .method("GET")
        .uri(&channel.ws_endpoint_url)
        .header(
            "authorization",
            format!("Bearer {}", &channel.operator_token),
        )
        .header("sec-websocket-key", generate_key())
        .header("host", RELAYER_HOST)
        .header("upgrade", "websocket")
        .header("connection", "upgrade")
        .header("sec-websocket-version", 13)
        .body(())
        .map_err(|e| format!("failed to create relayer ws connection: {}", e))
        .unwrap();

    let (ws_stream, _) = connect_async_tls_with_config(
        req,
        None,
        false,
        Some(tokio_tungstenite::Connector::NativeTls(
            TlsConnector::new().unwrap(),
        )),
    )
    .await
    .map_err(|e| format!("failed to connect to relayer ws channel: {}", e))
    .unwrap();

    let (write, mut read) = ws_stream.split();

    let (tx, mut rx) = mpsc::unbounded_channel();
    let write_task = tokio::spawn(async move {
        let mut write = write;
        while let Some(message) = rx.recv().await {
            if let Err(e) = write.send(message).await {
                println!("Error sending message: {}", e);
            }
        }
    });

    let read_task = tokio::spawn(async move {
        while let Some(message) = read.next().await {
            match message {
                Ok(msg) => match msg {
                    Message::Text(text) => {
                        println!("Operator received WS ActionItemResponse");
                        let response = match serde_json::from_str::<ActionItemResponse>(&text) {
                            Ok(response) => response,
                            Err(e) => {
                                println!(
                                    "error deserializing action item response: {}",
                                    e.to_string()
                                );
                                continue;
                            }
                        };
                        let _ = action_item_events_tx.try_send(response);
                    }
                    Message::Binary(_) => todo!(),
                    Message::Ping(ping) => {
                        println!("Received ping: {:?}", ping);
                        // Respond with pong message to keep the connection alive
                        match tx.send(Message::Pong(ping)) {
                            Err(e) => {
                                println!("Failed to queue pong message: {}", e);
                            }
                            Ok(_) => {}
                        }
                    }
                    Message::Pong(_) => todo!(),
                    Message::Close(_) => {
                        break;
                    }
                    Message::Frame(_) => todo!(),
                },
                Err(e) => return Err(format!("error parsing ws message: {}", e)),
            }
        }
        Ok(())
    });
    let _ = tokio::join!(write_task, read_task);
    Ok(())
}
