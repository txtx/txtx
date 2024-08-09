use actix_web::error::ErrorInternalServerError;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::HttpResponseBuilder;
use actix_web::{HttpRequest, HttpResponse};
use dotenvy_macro::dotenv;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::connect_async_tls_with_config;
use tokio_tungstenite::tungstenite::handshake::client::{generate_key, Request};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use totp_rs::{Algorithm, TOTP};
use txtx_core::kit::channel::{select, Receiver, Sender};
use txtx_core::kit::futures::{SinkExt, StreamExt};
use txtx_core::kit::reqwest::{self};
use txtx_core::kit::sha2::{Digest, Sha256};
use txtx_core::kit::types::frontend::{
    ActionItemResponse, BlockEvent, DeleteChannelRequest, OpenChannelRequest, OpenChannelResponse,
    OpenChannelResponseBrowser,
};
use txtx_core::kit::uuid::Uuid;
use txtx_gql::Context as GraphContext;

const RELAYER_BASE_URL: &str = dotenv!("RELAYER_BASE_URL");
const RELAYER_HOST: &str = dotenv!("RELAYER_HOST");

#[derive(Clone, Debug)]
pub enum RelayerChannelEvent {
    OpenChannel(ChannelData),
    DeleteChannel,
    ForwardEventToRelayer(BlockEvent),
    Exit,
}

#[derive(Clone, Debug)]
pub struct RelayerContext {
    pub relayer_channel_tx: Sender<RelayerChannelEvent>,
    pub channel_data: Arc<RwLock<Option<ChannelData>>>,
}
#[derive(Clone, Debug)]
pub struct ChannelData {
    pub operator_token: String,
    pub totp: String,
    pub http_endpoint_url: String,
    pub ws_endpoint_url: String,
    pub slug: String,
    // pub ws_channel_handle: RelayerWebSocketChannel,
}
impl ChannelData {
    pub fn new(
        operator_token: String,
        totp: String,
        slug: String,
        open_channel_response: OpenChannelResponse,
        // action_item_events_tx: &Sender<ActionItemResponse>,
    ) -> Self {
        // let ws_channel_handle = RelayerWebSocketChannel::new(
        //     &open_channel_response.ws_endpoint_url,
        //     &operator_token,
        //     action_item_events_tx,
        // );
        ChannelData {
            operator_token,
            totp: totp,
            http_endpoint_url: open_channel_response.http_endpoint_url,
            ws_endpoint_url: open_channel_response.ws_endpoint_url,
            slug: slug,
            // ws_channel_handle,
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

    let _ = relayer_context
        .relayer_channel_tx
        .send(RelayerChannelEvent::OpenChannel(ChannelData::new(
            token.to_string(),
            totp.clone(),
            slug.clone(),
            body.clone(),
            // &graph_context.action_item_events_tx,
        )));

    let response = OpenChannelResponseBrowser {
        totp: totp.clone(),
        http_endpoint_url: body.http_endpoint_url,
        ws_endpoint_url: body.ws_endpoint_url,
        slug: slug.clone(),
    };
    Ok(HttpResponseBuilder::new(StatusCode::OK).json(response))
}

pub async fn get_channel(relayer_context: Data<RelayerContext>) -> actix_web::Result<HttpResponse> {
    let Some(channel_data) = relayer_context.channel_data.read().await.clone() else {
        return Ok(HttpResponseBuilder::new(StatusCode::NOT_FOUND).finish());
    };

    let response = OpenChannelResponseBrowser {
        totp: channel_data.totp,
        http_endpoint_url: channel_data.http_endpoint_url,
        ws_endpoint_url: channel_data.ws_endpoint_url,
        slug: channel_data.slug,
    };
    Ok(HttpResponseBuilder::new(StatusCode::OK).json(response))
}

pub async fn delete_channel(
    req: HttpRequest,
    payload: Json<DeleteChannelRequest>,
) -> actix_web::Result<HttpResponse> {
    println!("DELETE /api/v1/channels");
    let Some(cookie) = req.cookie("hanko") else {
        return Ok(HttpResponse::Unauthorized().body("No auth data provided"));
    };

    let token = cookie.value();
    send_delete_channel(token, payload)
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponseBuilder::new(StatusCode::OK).finish())
}

async fn send_delete_channel(
    token: &str,
    payload: Json<DeleteChannelRequest>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let path = format!("{}/api/v1/channels", RELAYER_BASE_URL);

    let res = client
        .delete(path)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let _ = res.error_for_status().map_err(|e| e.to_string())?;
    Ok(())
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

pub async fn start_relayer_event_runloop(
    channel_data: Arc<RwLock<Option<ChannelData>>>,
    relayer_channel_rx: Receiver<RelayerChannelEvent>,
    relayer_channel_tx: Sender<RelayerChannelEvent>,
    action_item_events_tx: Sender<ActionItemResponse>,
    kill_loops_tx: Sender<bool>,
) -> Result<(), String> {
    // cache the tx that is used to send websocket messages. this will allow us to send a close signal
    let mut _ws_writer_tx: Option<tokio::sync::mpsc::UnboundedSender<Message>> = None;
    loop {
        select! {
            recv(relayer_channel_rx) -> rx_result => match rx_result {
                Err(e) => return Err(format!("relayer channel error: {}", e)),
                Ok(relayer_channel_event) => match relayer_channel_event {
                    RelayerChannelEvent::OpenChannel(new_channel) => {
                        let mut channel_data_rw = channel_data.write().await;

                        if channel_data_rw.is_none() {
                            let ws_endpoint_url = new_channel.ws_endpoint_url.clone();
                            let operator_token = new_channel.operator_token.clone();
                            let moved_action_item_events_tx = action_item_events_tx.clone();

                            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                            _ws_writer_tx = Some(tx.clone());
                            let moved_relayer_channel_tx = relayer_channel_tx.clone();

                            let _ = hiro_system_kit::thread_named("Runbook Runloop")
                                .spawn(move || {
                                    let mut ws_channel = RelayerWebSocketChannel::new(
                                        &ws_endpoint_url,
                                        &operator_token,
                                        &moved_action_item_events_tx.clone(),
                                    );
                                    let future = ws_channel.start(tx.clone(), rx, moved_relayer_channel_tx);
                                    if let Err(e) = hiro_system_kit::nestable_block_on(future) {
                                        eprintln!("WebSocket channel error: {:?}", e);
                                    }
                                })
                                .unwrap();


                            *channel_data_rw = Some(new_channel);
                        }
                    }
                    RelayerChannelEvent::ForwardEventToRelayer(block_event) => {
                      if let Some(channel_data_r) = channel_data.read().await.clone() {
                        match forward_block_event(
                            channel_data_r.operator_token,
                            channel_data_r.slug,
                            block_event,
                        )
                        .await
                        {
                            Err(e) => {
                                println!("{}", e);
                                let _ = kill_loops_tx.clone().send(true);
                            }
                            Ok(_) => {}
                        };
                      }
                    }
                    RelayerChannelEvent::DeleteChannel => {
                      let mut channel_data_rw = channel_data.write().await;
                      *channel_data_rw = None;
                      _ws_writer_tx = None;
                      println!("dropped writer");
                    }
                    // todo: on channel exit, we don't currently delete things relayer side to clean up
                    RelayerChannelEvent::Exit => {
                        if let Some(channel_data) = channel_data.read().await.clone() {
                            let _ = send_delete_channel(&channel_data.operator_token, Json(DeleteChannelRequest { slug: channel_data.slug })).await;
                        }



                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Clone, Debug)]
pub struct RelayerWebSocketChannel {
    ws_endpoint_url: String,
    operator_token: String,
    action_item_events_tx: Sender<ActionItemResponse>,
}
impl RelayerWebSocketChannel {
    pub fn new(
        ws_endpoint_url: &String,
        operator_token: &String,
        action_item_events_tx: &Sender<ActionItemResponse>,
    ) -> Self {
        RelayerWebSocketChannel {
            ws_endpoint_url: ws_endpoint_url.clone(),
            operator_token: operator_token.clone(),
            action_item_events_tx: action_item_events_tx.clone(),
        }
    }

    pub fn close(writer_tx: tokio::sync::mpsc::UnboundedSender<Message>) {
        let _ = writer_tx.send(Message::Close(Some(CloseFrame {
            code: CloseCode::Normal,
            reason: std::borrow::Cow::Borrowed("Closed by user."),
        })));
    }

    pub async fn start(
        &mut self,
        writer_tx: tokio::sync::mpsc::UnboundedSender<Message>,
        mut writer_rx: tokio::sync::mpsc::UnboundedReceiver<Message>,
        relayer_channel_tx: Sender<RelayerChannelEvent>,
    ) -> Result<(), String> {
        Ok(())
    }

    // pub async fn start(
    //     &mut self,
    //     writer_tx: tokio::sync::mpsc::UnboundedSender<Message>,
    //     mut writer_rx: tokio::sync::mpsc::UnboundedReceiver<Message>,
    //     relayer_channel_tx: Sender<RelayerChannelEvent>,
    // ) -> Result<(), String> {
    //     let req = Request::builder()
    //         .method("GET")
    //         .uri(&self.ws_endpoint_url)
    //         .header("authorization", format!("Bearer {}", &self.operator_token))
    //         .header("sec-websocket-key", generate_key())
    //         .header("host", RELAYER_HOST)
    //         .header("upgrade", "websocket")
    //         .header("connection", "upgrade")
    //         .header("sec-websocket-version", 13)
    //         .body(())
    //         .map_err(|e| format!("failed to create relayer ws connection: {}", e))
    //         .unwrap();

    //     let (ws_stream, _) = connect_async_tls_with_config(
    //         req,
    //         None,
    //         false,
    //         Some(tokio_tungstenite::Connector::Rustls(
    //             tokio_tungstenite::::ClientConfig(),
    //         )),
    //     )
    //     .await
    //     .map_err(|e| format!("relayer ws channel failed: {}", e))
    //     .unwrap();

    //     let (write, mut read) = ws_stream.split();

    //     let write_task = tokio::spawn(async move {
    //         let mut write = write;
    //         while let Some(message) = writer_rx.recv().await {
    //             if let Err(e) = write.send(message.clone()).await {
    //                 println!("Error sending message: {}", e);
    //             }
    //             if let Message::Close(_) = message {
    //                 break;
    //             }
    //         }
    //     });

    //     let action_item_events_tx = self.action_item_events_tx.clone();
    //     let read_task = tokio::spawn(async move {
    //         while let Some(message) = read.next().await {
    //             match message {
    //                 Ok(msg) => match msg {
    //                     Message::Text(text) => {
    //                         println!("Operator received WS ActionItemResponse");
    //                         let response = match serde_json::from_str::<ActionItemResponse>(&text) {
    //                             Ok(response) => response,
    //                             Err(e) => {
    //                                 println!(
    //                                     "error deserializing action item response: {}",
    //                                     e.to_string()
    //                                 );
    //                                 continue;
    //                             }
    //                         };
    //                         let _ = action_item_events_tx.try_send(response);
    //                     }
    //                     Message::Binary(_) => todo!(),
    //                     Message::Ping(ping) => {
    //                         // Respond with pong message to keep the connection alive
    //                         match writer_tx.send(Message::Pong(ping)) {
    //                             Err(e) => {
    //                                 println!("Failed to queue pong message: {}", e);
    //                             }
    //                             Ok(_) => {}
    //                         }
    //                     }
    //                     Message::Pong(_) => todo!(),
    //                     Message::Close(_) => {
    //                         println!("received close event from relayer");
    //                         let _ = relayer_channel_tx.send(RelayerChannelEvent::DeleteChannel);
    //                         break;
    //                     }
    //                     Message::Frame(_) => todo!(),
    //                 },
    //                 Err(e) => return Err(format!("error parsing ws message: {}", e)),
    //             }
    //         }
    //         Ok(())
    //     });
    //     let _ = tokio::join!(write_task, read_task);
    //     Ok(())
    // }
}
