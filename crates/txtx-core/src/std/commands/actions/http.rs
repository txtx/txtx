use txtx_addon_kit::reqwest::{self, Method};
use txtx_addon_kit::types::commands::{CommandExecutionFutureResult, PreCommandSpecification};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::{define_command, indoc};

lazy_static! {
    pub static ref SEND_HTTP_REQUEST: PreCommandSpecification = define_command! {
        SendHttpRequest => {
            name: "Send an HTTP request",
            matcher: "send_http_request",
            documentation: "`std::send_http_request` makes an HTTP request to the given URL and exports the response.",
            implements_signing_capability: false,
            implements_background_task_capability: false,
            inputs: [
                url: {
                    documentation: "The URL for the request. Supported schemes are http and https.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                body: {
                  documentation: "The request body as a string or json object.",
                  typing: Type::string(),
                  optional: true,
                  tainting: true,
                  internal: false
                },
                method: {
                  documentation: indoc!{r#"
                  The HTTP Method for the request. 
                  Allowed methods are a subset of methods defined in RFC7231: GET, HEAD, and POST. 
                  POST support is only intended for read-only URLs, such as submitting a search."#},
                  typing: Type::string(),
                  optional: true,
                  tainting: true,
                  internal: false
                },
                timeout_ms: {
                  documentation: "The request timeout in milliseconds.",
                  typing: Type::integer(),
                  optional: true,
                  tainting: true,
                  internal: false
                },
                headers: {
                    documentation: "A map of request header field names and values.",
                    typing: Type::arbitrary_object(),
                    optional: true,
                    tainting: true,
                    internal: false
                }
            ],
            outputs: [
                response_body: {
                    documentation: "The response body returned as a string.",
                    typing: Type::string()
                },
                status_code: {
                    documentation: "The HTTP response status code.",
                    typing: Type::integer()
                }
            ],
            example: indoc!{r#"
            action "example" "std::send_http_request" {
              url = "https://example.com"
            }
          
            output "status" {
              value = action.example.status_code
            }
            // > status: 200
            "#},
        }
    };
}
pub struct SendHttpRequest;

impl CommandImplementation for SendHttpRequest {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none())
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let mut result = CommandExecutionResult::new();
        let values = values.clone();
        let url = values.get_expected_string("url")?.to_string();
        let request_body = values.get_value("body").cloned();
        let method = {
            let value = values.get_string("method").unwrap_or("GET");
            Method::try_from(value).unwrap()
        };
        let request_headers = values.get_value("headers").cloned();

        let future = async move {
            let request_headers = request_headers
                .as_ref()
                .map(|value| {
                    value
                        .as_object()
                        .ok_or_else(|| diagnosed_error!("request headers must be an object"))
                })
                .transpose()?;

            let client = reqwest::Client::new();
            let mut req_builder = client.request(method, url);

            if let Some(request_headers) = request_headers {
                for (k, v) in request_headers.iter() {
                    req_builder = req_builder.header(
                        k,
                        v.as_string().ok_or_else(|| {
                            diagnosed_error!("request header value must be a string; found type '{}' for header '{}'", v.get_type().to_string(), k)
                        })?,
                    );
                }
            }

            if let Some(request_body) = request_body {
                if request_body.as_object().is_some() {
                    req_builder = req_builder.json(&request_body.to_json());
                } else {
                    req_builder = req_builder.body(request_body.encode_to_string());
                }
            }

            let res = req_builder.send().await.map_err(|e| {
                Diagnostic::error_from_string(format!("unable to send http request - {e}"))
            })?;

            let status_code = res.status();
            let response_body = res.text().await.map_err(|e| {
                Diagnostic::error_from_string(format!("Failed to parse http request result: {e}"))
            })?;

            result
                .outputs
                .insert(format!("status_code"), Value::integer(status_code.as_u16().into()));

            result.outputs.insert(format!("response_body"), Value::string(response_body));

            Ok::<CommandExecutionResult, Diagnostic>(result)
        };
        #[cfg(feature = "wasm")]
        panic!("async commands are not enabled for wasm");
        #[cfg(not(feature = "wasm"))]
        Ok(Box::pin(future))
    }
}
