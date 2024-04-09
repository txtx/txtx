use crate::cli::Context;
use rocket::config::{self, Config as RocketConfig, LogLevel};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{ContentType, Header};
use rocket::response::content::RawHtml;
use rocket::{routes, State};
use rocket::{Request, Response};
use std::borrow::Cow;
use std::error::Error;
use std::ffi::OsStr;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use txtx_gql::new_graphql_schema;
use txtx_gql::{Context as GraphContext, NestorGraphqlSchema};

use super::Asset;

pub async fn start_server(
    gql_context: GraphContext,
    port: u16,
    ctx: &Context,
) -> Result<(), Box<dyn Error>> {
    let log_level = LogLevel::Off;

    let mut shutdown_config = config::Shutdown::default();
    shutdown_config.ctrlc = false;
    shutdown_config.grace = 1;
    shutdown_config.mercy = 1;

    let control_config = RocketConfig {
        port,
        workers: 1,
        address: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        keep_alive: 5,
        temp_dir: std::env::temp_dir().into(),
        log_level,
        cli_colors: false,
        shutdown: shutdown_config,
        ..RocketConfig::default()
    };

    let routes = routes![
        serve_index,
        serve_dist,
        get_graphql,
        post_graphql,
        all_options,
        graphiql
    ];

    let ctx_cloned = ctx.clone();
    let ignite = rocket::custom(control_config)
        .manage(ctx_cloned)
        .manage(new_graphql_schema())
        .manage(gql_context)
        .attach(Cors)
        .mount("/", routes)
        .ignite()
        .await?;

    let _ = std::thread::spawn(move || {
        let _ = hiro_system_kit::nestable_block_on(ignite.launch());
    });
    Ok(())
}

#[get("/")]
fn serve_index() -> Option<RawHtml<Cow<'static, [u8]>>> {
    let asset = Asset::get("index.html")?;
    Some(RawHtml(asset.data))
}

#[get("/<file..>")]
fn serve_dist(file: PathBuf) -> Option<(ContentType, Cow<'static, [u8]>)> {
    let filename = file.display().to_string();
    let asset = Asset::get(&filename)?;
    let content_type = file
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ContentType::from_extension)
        .unwrap_or(ContentType::Bytes);

    Some((content_type, asset.data))
}

// GET request accepts query parameters like these:
// ?query=<urlencoded-graphql-query-string>
// &operationName=<optional-name>
// &variables=<optional-json-encoded-variables>
// See details here: https://graphql.org/learn/serving-over-http#get-request
#[rocket::get("/graphql?<request..>")]
async fn get_graphql(
    ctx: &State<GraphContext>,
    request: juniper_rocket::GraphQLRequest,
    schema: &State<NestorGraphqlSchema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(schema, ctx).await
}

#[rocket::post("/graphql", data = "<request>")]
async fn post_graphql(
    ctx: &State<GraphContext>,
    request: juniper_rocket::GraphQLRequest,
    schema: &State<NestorGraphqlSchema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(schema, ctx).await
}

/// Catches all OPTION requests in order to get the CORS related Fairing triggered.
#[options("/<_..>")]
fn all_options() {
    /* Intentionally left empty */
}

#[rocket::get("/explorer")]
fn graphiql() -> RawHtml<String> {
    juniper_rocket::graphiql_source("http://localhost:3210/graphql", None)
}

pub struct Cors;

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "Cross-Origin-Resource-Sharing Fairing",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, PATCH, PUT, DELETE, HEAD, OPTIONS, GET",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}
