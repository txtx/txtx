use kit::types::ValueStore;
use std::collections::HashMap;
use txtx_addon_kit::reqwest::header::CONTENT_TYPE;
use txtx_addon_kit::reqwest::{self, Method};
use txtx_addon_kit::types::commands::{
    CommandExecutionContext, CommandExecutionFutureResult, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::ActionItemRequest;
use txtx_addon_kit::types::types::ObjectProperty;
use txtx_addon_kit::types::wallets::WalletInstance;
use txtx_addon_kit::types::ConstructUuid;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::{define_command, indoc, AddonDefaults};

lazy_static! {
    pub static ref SEND_HTTP_REQUEST: PreCommandSpecification = define_command! {
        SendHttpRequest => {
            name: "Send an HTTP request",
            matcher: "send_http_request",
            documentation: "`send_http_request` command makes an HTTP request to the given URL and exports the response.",
            inputs: [
                url: {
                    documentation: "The URL for the request. Supported schemes are http and https.",
                    typing: Type::string(),
                    optional: false,
                    interpolable: true
                },
                request_body: {
                  documentation: "The request body as a string.",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
                },
                method: {
                  documentation: indoc!{r#"
                  The HTTP Method for the request. 
                  Allowed methods are a subset of methods defined in RFC7231: GET, HEAD, and POST. 
                  POST support is only intended for read-only URLs, such as submitting a search."#},
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
                },
                request_timeout_ms: {
                  documentation: "The request timeout in milliseconds.",
                  typing: Type::uint(),
                  optional: true,
                  interpolable: true
                },
                request_headers: {
                    documentation: "A map of request header field names and values.",
                    typing: Type::object(vec![ObjectProperty {
                        name: "Content-Type".into(),
                        documentation: "Content-Type".into(),
                        typing: Type::string(),
                        optional: true,
                        interpolable: true,
                    }]),
                    optional: true,
                    interpolable: true
                }
            ],
            outputs: [
                response_body: {
                    documentation: "The response body returned as a string.",
                    typing: Type::string()
                },
                status_code: {
                    documentation: "The HTTP response status code.",
                    typing: Type::uint()
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
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Vec<ActionItemRequest>, Diagnostic> {
        unimplemented!()
    }

    fn execute(
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        _wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        _progress_tx: &txtx_addon_kit::channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult {
        let mut result = CommandExecutionResult::new();
        let args = args.clone();
        let url = args.get_expected_string("url")?.to_string();
        let request_body = args.get_string("request_body").map(|v| v.to_string());
        let method = {
            let value = args.get_string("method").unwrap_or("GET");
            Method::try_from(value).unwrap()
        };
        let request_headers = args
            .get_value("request_headers")
            .and_then(|value| Some(value.expect_object().clone()));

        let future = async move {
            let client = reqwest::Client::new();
            let mut req_builder = client.request(method, url);

            req_builder = req_builder.header(CONTENT_TYPE, "application/json");

            if let Some(request_headers) = request_headers {
                for (k, v) in request_headers.iter() {
                    if let Ok(v) = v {
                        req_builder = req_builder.header(k, v.expect_string());
                    }
                }
            }

            if let Some(request_body) = request_body {
                req_builder = req_builder.body(request_body);
            }

            let res = req_builder.send().await.map_err(|e| {
                Diagnostic::error_from_string(format!(
                    "Failed to broadcast stacks transaction: {e}"
                ))
            })?;

            let status_code = res.status();
            let response_body = res.text().await.map_err(|e| {
                Diagnostic::error_from_string(format!(
                    "Failed to parse broadcasted Stacks transaction result: {e}"
                ))
            })?;

            result.outputs.insert(
                format!("status_code"),
                Value::uint(status_code.as_u16().into()),
            );

            result
                .outputs
                .insert(format!("response_body"), Value::string(response_body));

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
