use actix_cors::Cors;
use actix_web::http::header;
use actix_web::web::{self, Data, Json};
use actix_web::{middleware, App, HttpRequest, HttpResponse, HttpServer, Responder};
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;

use txtx_core::kit::channel::{Receiver, Sender};

use serde::{Deserialize, Serialize};
use txtx_core::kit::{channel, reqwest};

use crate::cli::{Context, LoginCommand};

use super::{AuthConfig, AuthUser};
use crate::cli::cloud::{AUTH_CALLBACK_PORT, AUTH_SERVICE_URL};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginCallbackResult {
    access_token: String,
    refresh_token: String,
    user: AuthUser,
    exp: u64,
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

pub async fn handle_login_command(cmd: &LoginCommand, _ctx: &Context) -> Result<(), String> {
    let auth_config = AuthConfig::read_from_system_config()?;

    if let Some(auth_config) = auth_config {
        match auth_config.refresh_session().await {
            Ok(auth_config) => {
                println!(
                    "{} User {} already logged in.",
                    green!("✓"),
                    auth_config.user.display_name
                );
                return Ok(());
            }
            Err(e) => {
                println!("{} Auth data already found for user, but failed to refresh session: {}; attempting login.", yellow!("-"), e);
            }
        }
    }

    let auth_config = if let Some(email) = &cmd.email {
        let password =
            cmd.password.as_ref().ok_or("Password is required when email is provided")?;
        user_pass_login(email, password).await?
    } else if let Some(pat) = &cmd.pat {
        pat_login(pat)?
    } else {
        let Some(res) = id_service_login().await? else { return Ok(()) };
        let auth_config = AuthConfig::new(res.access_token, res.refresh_token, res.user, res.exp);
        auth_config
    };

    auth_config.write_to_system_config()?;
    Ok(())
}

/// Starts a server that will only receive a POST request from the ID service with the user's auth data.
/// Directs the user to the ID service login page.
/// Upon login, the ID service will send a POST request to the server with the user's auth data.
async fn id_service_login() -> Result<Option<LoginCallbackResult>, String> {
    let redirect_url = format!("127.0.0.1:{AUTH_CALLBACK_PORT}");

    let auth_service_url = reqwest::Url::parse(&format!(
        "{}/login?callbackUrl=http://{}/api/v1/auth",
        AUTH_SERVICE_URL, redirect_url
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
                    .allowed_methods(vec!["POST", "OPTIONS"])
                    .allowed_headers(vec![header::CONTENT_TYPE, header::ACCEPT])
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(
                web::scope("/api/v1")
                .route("/auth", web::post().to(auth_callback))
            )
    })
    .workers(1)
    .bind(redirect_url)
    .map_err(|e| format!("Failed to start auth callback server: failed to bind to port {AUTH_CALLBACK_PORT}: {e}"))?
    .run();
    let handle = server.handle();
    tokio::spawn(server);

    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Open id.txtx.run in your browser to log in?")
        .default(true)
        .interact();

    let Ok(true) = confirm else {
        handle.stop(true).await;
        println!("\nLogin cancelled");
        return Ok(None);
    };

    if let Err(_) = open::that(auth_service_url.as_str()) {
        println!("Please open the following URL in your browser: {}", auth_service_url);
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
    payload: Json<LoginCallbackServerEvent>,
) -> actix_web::Result<impl Responder> {
    let payload = payload.into_inner();
    let msg = match &payload {
        LoginCallbackServerEvent::AuthCallback(_) => {
            "Authentication successful. You can close this tab.".into()
        }
        LoginCallbackServerEvent::AuthError(e) => format!("Authentication failed: {}", e.message),
    };
    ctx.tx.send(payload).map_err(|_| {
        actix_web::error::ErrorInternalServerError("Failed to send auth callback event")
    })?;
    Ok(HttpResponse::Ok().body(msg))
}

/// Sends a POST request to the auth service to log in with an email and password.
async fn user_pass_login(email: &str, password: &str) -> Result<AuthConfig, String> {
    let client = reqwest::Client::new();
    let res = client
        .post(&format!("{}/signin/email-password", AUTH_SERVICE_URL))
        .json(&serde_json::json!({
            "email": email,
            "password": password,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send login request: {}", e))?;

    if res.status().is_success() {
        let res = res
            .json::<AuthConfig>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        return Ok(res);
    } else {
        let err = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Failed to login: {}", err));
    }
}

fn pat_login(_pat: &str) -> Result<AuthConfig, String> {
    todo!()
}
