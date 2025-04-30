use std::collections::HashMap;

use actix_cors::Cors;
use actix_web::error::QueryPayloadError;
use actix_web::http::header;
use actix_web::web::{self, Data};
use actix_web::{middleware, App, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder};
use base64::Engine;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;

use hiro_system_kit::{green, yellow};
use serde::de::Error;
use txtx_core::kit::channel::{Receiver, Sender};

use serde::{Deserialize, Serialize};
use txtx_core::kit::futures::future::{ready, Ready};
use txtx_core::kit::{channel, reqwest};

use crate::auth::jwt::JwtManager;
use crate::auth::AuthUser;
use crate::LoginCommand;

use super::auth::AuthConfig;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginCallbackResult {
    access_token: String,
    exp: u64,
    refresh_token: String,
    pat: String,
    user: AuthUser,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginCallbackError {
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum LoginCallbackServerEvent {
    AuthCallback(LoginCallbackResult),
    AuthError(LoginCallbackError),
}
// The actix_web `Query<>` extractor was having a hard time with the enums and nested objects here,
// and we wanted to base64 encode the data, so we implemented our own `FromRequest` extractor.
impl FromRequest for LoginCallbackServerEvent {
    type Error = QueryPayloadError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        // Extract the query string from the request
        let query_string = req.query_string();

        let decoded = match base64::engine::general_purpose::URL_SAFE.decode(query_string) {
            Ok(decoded) => decoded,
            Err(err) => {
                let error = QueryPayloadError::Deserialize(serde_urlencoded::de::Error::custom(
                    format!("Base64 decode error: {}", err),
                ));
                return ready(Err(error));
            }
        };

        // Convert decoded bytes to a string
        let decoded_str = match String::from_utf8(decoded) {
            Ok(s) => s,
            Err(err) => {
                let error = QueryPayloadError::Deserialize(serde_urlencoded::de::Error::custom(
                    format!("UTF-8 conversion error: {}", err),
                ));
                return ready(Err(error));
            }
        };

        let mut params: HashMap<String, String> = match serde_urlencoded::from_str(&decoded_str) {
            Ok(params) => params,
            Err(err) => {
                let error = QueryPayloadError::Deserialize(err);
                return ready(Err(error));
            }
        };
        // Handle the `user` field separately if it exists
        if let Some(user_json) = params.remove("user") {
            let user: AuthUser = match serde_json::from_str(&user_json) {
                Ok(user) => user,
                Err(err) => {
                    let error =
                        QueryPayloadError::Deserialize(serde_urlencoded::de::Error::custom(
                            format!("Failed to parse 'user' field: {}", err),
                        ));
                    return ready(Err(error));
                }
            };

            // Reconstruct `LoginCallbackServerEvent` with the parsed `user`
            if params.contains_key("accessToken")
                && params.contains_key("exp")
                && params.contains_key("refreshToken")
                && params.contains_key("pat")
            {
                let result = LoginCallbackResult {
                    access_token: params.remove("accessToken").unwrap(),
                    exp: params.remove("exp").unwrap().parse().unwrap_or_default(),
                    refresh_token: params.remove("refreshToken").unwrap(),
                    pat: params.remove("pat").unwrap(),
                    user,
                };

                return ready(Ok(LoginCallbackServerEvent::AuthCallback(result)));
            }
        }

        // If no matching variant is found, return an error
        ready(Err(QueryPayloadError::Deserialize(serde_urlencoded::de::Error::custom(
            "Data did not match any variant",
        ))))
    }
}

#[derive(Debug, Clone)]
struct LoginCallbackServerContext {
    tx: Sender<LoginCallbackServerEvent>,
}

impl LoginCallbackServerContext {
    fn new() -> (Self, Receiver<LoginCallbackServerEvent>) {
        let (tx, rx) = channel::unbounded::<LoginCallbackServerEvent>();
        (Self { tx }, rx)
    }
}

/// ## Arguments
///
/// * `cmd` - The login command containing user-provided credentials or options.
/// * `auth_service_url` - The URL of the frontend service used to authenticate the user.
/// * `auth_callback_port` - The port for the callback server used during login.
/// * `id_service_url` - The URL of the ID service.
pub async fn handle_login_command(
    cmd: &LoginCommand,
    auth_service_url: &str,
    auth_callback_port: &str,
    id_service_url: &str,
) -> Result<(), String> {
    let auth_config = AuthConfig::read_from_system_config()?;

    let jwt_manager = crate::auth::jwt::JwtManager::initialize(id_service_url)
        .await
        .map_err(|e| format!("Failed to initialize JWT manager: {}", e))?;

    if let Some(auth_config) = auth_config {
        if auth_config.is_access_token_expired() {
            match auth_config.refresh_session(id_service_url, &auth_config.pat).await {
                Ok(auth_config) => {
                    println!(
                        "{} Logged in as {}.",
                        green!("✓"),
                        auth_config.user.display_name
                    );
                    return Ok(());
                }
                Err(_e) => {
                    if let Some(pat) = &auth_config.pat {
                        if let Ok(auth_config) = pat_login(id_service_url, &jwt_manager, &pat).await
                        {
                            auth_config.write_to_system_config()?;
                            println!(
                                "{} Logged in as {}.",
                                green!("✓"),
                                auth_config.user.display_name
                            );
                            return Ok(());
                        }
                    }
                    println!("{} Auth data already found for user, but failed to refresh session; attempting login.", yellow!("-"));
                }
            }
        } else {
            println!("{} Logged in as {}.", green!("✓"), auth_config.user.display_name);
            return Ok(());
        }
    }

    let auth_config = if let Some(email) = &cmd.email {
        let password =
            cmd.password.as_ref().ok_or("Password is required when email is provided")?;
        user_pass_login(id_service_url, &jwt_manager, email, password).await?
    } else if let Some(pat) = &cmd.pat {
        pat_login(id_service_url, &jwt_manager, &pat).await?
    } else {
        let Some(res) = auth_service_login(auth_service_url, auth_callback_port).await? else {
            return Ok(());
        };
        let auth_config =
            AuthConfig::new(res.access_token, res.exp, res.refresh_token, Some(res.pat), res.user);
        auth_config
    };

    auth_config.write_to_system_config()?;
    Ok(())
}

/// Starts a server that will only receive a POST request from the ID service with the user's auth data.
/// Directs the user to the ID service login page.
/// Upon login, the ID service will send a POST request to the server with the user's auth data.
async fn auth_service_login(
    auth_service_url: &str,
    auth_callback_port: &str,
) -> Result<Option<LoginCallbackResult>, String> {
    let redirect_url = format!("localhost:{}", auth_callback_port);

    let auth_service_url = reqwest::Url::parse(&format!(
        "{}?redirectUrl=http://{}/api/v1/auth",
        auth_service_url, redirect_url
    ))
    .map_err(|e| format!("Invalid auth service URL: {e}"))?;

    let allowed_origin = auth_service_url.origin().ascii_serialization();
    let (ctx, rx) = LoginCallbackServerContext::new();
    let ctx = Data::new(ctx);
    let server = HttpServer::new(move || {
        App::new()
            .app_data(ctx.clone())
            .wrap(
                Cors::default()
                    .allowed_origin(&allowed_origin)
                    .allowed_methods(vec!["GET", "OPTIONS"])
                    .allowed_headers(vec![header::CONTENT_TYPE, header::ACCEPT])
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(
                web::scope("/api/v1")
                .route("/auth", web::get().to(auth_callback))
            )
    })
    .workers(1)
    .bind(redirect_url)
    .map_err(|e| format!("Failed to start auth callback server: failed to bind to port {auth_callback_port}: {e}"))?
    .run();
    let handle = server.handle();
    tokio::spawn(server);

    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Open {} in your browser to log in?", auth_service_url))
        .default(true)
        .interact();

    let Ok(true) = confirm else {
        handle.stop(true).await;
        println!("\nLogin cancelled");
        return Ok(None);
    };

    if let Err(_) = open::that(auth_service_url.as_str()) {
        println!("Failed to automatically open your browser. Please open the following URL in your browser: {}", auth_service_url);
    };

    let res = rx.recv();
    handle.stop(true).await;
    match res {
        Ok(event) => match event {
            LoginCallbackServerEvent::AuthCallback(auth_callback_result) => {
                Ok(Some(auth_callback_result))
            }
            LoginCallbackServerEvent::AuthError(auth_callback_error) => {
                Err(format!("Authentication failed: {}", auth_callback_error.message))
            }
        },
        Err(e) => Err(format!("Failed to receive auth callback event: {e}")),
    }
}

async fn auth_callback(
    _req: HttpRequest,
    ctx: Data<LoginCallbackServerContext>,
    payload: LoginCallbackServerEvent,
) -> actix_web::Result<impl Responder> {
    let body = match &payload {
        LoginCallbackServerEvent::AuthCallback(_) => include_str!("./callback.html").to_string(),
        LoginCallbackServerEvent::AuthError(e) => format!("Authentication failed: {}", e.message),
    };
    ctx.tx.send(payload).map_err(|_| {
        actix_web::error::ErrorInternalServerError("Failed to send auth callback event")
    })?;
    Ok(HttpResponse::Ok().body(body))
}

/// Sends a POST request to the auth service to log in with an email and password.
async fn user_pass_login(
    id_service_url: &str,
    jwt_manager: &JwtManager,
    email: &str,
    password: &str,
) -> Result<AuthConfig, String> {
    let client = reqwest::Client::new();
    let res = client
        .post(&format!("{}/signin/email-password", id_service_url))
        .json(&serde_json::json!({
            "email": email,
            "password": password,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send username/password login request: {}", e))?;

    if res.status().is_success() {
        let res = res
            .json::<LoginResponse>()
            .await
            .map_err(|e| format!("Failed to parse username/password login response: {}", e))?;

        let access_token_claims =
            jwt_manager.decode_jwt(&res.session.access_token, true).map_err(|e| {
                format!("Failed to decode JWT from username/password login response: {}", e)
            })?;

        let auth_config = AuthConfig::new(
            res.session.access_token,
            access_token_claims.exp,
            res.session.refresh_token,
            None,
            res.session.user,
        );
        return Ok(auth_config);
    } else {
        let err = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Failed to login with username + password: {}", err));
    }
}

async fn pat_login(
    id_service_url: &str,
    jwt_manager: &JwtManager,
    pat: &str,
) -> Result<AuthConfig, String> {
    let client = reqwest::Client::new();
    let res = client
        .post(&format!("{}/signin/pat", id_service_url))
        .json(&serde_json::json!({
            "personalAccessToken": pat,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send PAT login request: {}", e))?;

    if res.status().is_success() {
        let res = res
            .json::<LoginResponse>()
            .await
            .map_err(|e| format!("Failed to parse PAT login response: {}", e))?;

        let access_token_claims = jwt_manager
            .decode_jwt(&res.session.access_token, true)
            .map_err(|e| format!("Failed to decode JWT from PAT login response: {}", e))?;

        let auth_config = AuthConfig::new(
            res.session.access_token,
            access_token_claims.exp,
            res.session.refresh_token,
            Some(pat.to_string()),
            res.session.user,
        );
        return Ok(auth_config);
    } else {
        let err = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Failed to login with PAT: {}", err));
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub session: Session,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub user: AuthUser,
}
