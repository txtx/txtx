use actix_web::error::ErrorInternalServerError;
use actix_web::http::StatusCode;
use actix_web::web::Json;
use actix_web::HttpResponseBuilder;
use actix_web::{HttpRequest, HttpResponse};
use dotenvy_macro::dotenv;
use txtx_core::kit::reqwest;

const RELAYER_BASE_URL: &str = dotenv!("RELAYER_BASE_URL");

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename = "runbook")]
pub struct OpenChannelRequest {
    pub name: String,
    pub description: String,
}
#[derive(Clone, Serialize, Deserialize)]
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
    Ok(HttpResponseBuilder::new(StatusCode::OK).json(body))
}
