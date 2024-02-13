// use jaq_core;
use txtx_addon_kit::{
    define_function,
    types::{
        diagnostics::Diagnostic, functions::{FunctionImplementation, FunctionSpecification}, types::{Typing, Value}
    },
};

lazy_static! {
    pub static ref JSON_FUNCTIONS: Vec<FunctionSpecification> = vec![define_function! {
        JsonQuery => {
            name: "json_query",
            documentation: "Query Json data",
            example: "json_query(\"{ \"message\": \"Hello world!\" }\", \".root\")",
            inputs: [
                decoded_json: {
                    documentation: "Json document",
                    typing: vec![Typing::string()]
                },
                query: {
                    documentation: "Json query (see jq documentation)",
                    typing: vec![Typing::string()]
                }
            ],
            output: {
                documentation: "Result of the query",
                typing: Typing::string()
            },
        }
    },];
}

pub struct JsonQuery;
impl FunctionImplementation for JsonQuery {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        // todo(lgalabru): Parse string, parse query then run query on document
        // json!(args[0])
        // jaq_core::minimal()
        Ok(Value::Bool(true))
    }
}
