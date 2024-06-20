use actix_web::error::ErrorInternalServerError;
use actix_web::http::StatusCode;
use actix_web::web::Json;
use actix_web::HttpResponseBuilder;
use actix_web::{HttpRequest, HttpResponse};
use txtx_core::kit::reqwest;

const URL: &str = "";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename = "runbook")]
pub struct OpenChannelRequest {
    pub name: String,
    pub description: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct OpenChannelResponse {
    pub totp_token: String,
    pub http_endpoint_url: String,
    pub ws_endpoint_url: String,
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
    let path = format!("{}/api/v1/channels", URL);

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
