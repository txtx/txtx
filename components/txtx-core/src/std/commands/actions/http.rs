use std::{collections::HashMap, pin::Pin};
use txtx_addon_kit::reqwest::{self, Method};
use txtx_addon_kit::types::commands::PreCommandSpecification;
use txtx_addon_kit::types::{
    commands::{
        CommandExecutionResult, CommandImplementationAsync, CommandInputsEvaluationResult,
        CommandSpecification,
    },
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::{define_async_command, AddonDefaults};

lazy_static! {
    pub static ref SEND_HTTP_REQUEST: PreCommandSpecification = define_async_command! {
        BroadcastStacksTransaction => {
            name: "Send an HTTP request",
            matcher: "send_http_request",
            documentation: "The http command makes an HTTP GET request to the given URL and exports information about the response.",
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
                  documentation: "The HTTP Method for the request. Allowed methods are a subset of methods defined in RFC7231 namely, GET, HEAD, and POST. POST support is only intended for read-only URLs, such as submitting a search.",
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
                    typing: Type::object(vec![]),
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
            example: "",
        }
    };
}
pub struct BroadcastStacksTransaction;
impl CommandImplementationAsync for BroadcastStacksTransaction {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<CommandExecutionResult, Diagnostic>>>> //todo: alias type
    {
        let mut result = CommandExecutionResult::new();
        let args = args.clone();
        let future = async move {
            let url = args.get("url").unwrap().expect_string();
            let request_body = args
                .get("request_body")
                .and_then(|v| Some(v.expect_string().to_string()));
            let method = {
                let value = args
                    .get("method")
                    .and_then(|v| Some(v.expect_string()))
                    .unwrap_or("GET");
                Method::try_from(value).unwrap()
            };
            let request_headers = args
                .get("request_headers")
                .and_then(|value| Some(value.expect_object()));
            let client = reqwest::Client::new();
            let mut req_builder = client.request(method, url);

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

        Box::pin(future)
    }

    fn update_input_evaluation_results_from_user_input(
        _ctx: &CommandSpecification,
        _current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String,
        _value: String,
    ) {
        unimplemented!()
    }
}
