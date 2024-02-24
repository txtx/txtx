// use jaq_core;
use txtx_addon_kit::{
    define_function,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{Type, Value},
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
                    typing: vec![Type::string()]
                },
                query: {
                    documentation: "Json query (see jq documentation)",
                    typing: vec![Type::string()]
                }
            ],
            output: {
                documentation: "Result of the query",
                typing: Type::string()
            },
        }
    },];
}

pub struct JsonQuery;
impl FunctionImplementation for JsonQuery {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        // todo(lgalabru): Parse string, parse query then run query on document
        // json!(args[0])
        // jaq_core::minimal()
        Ok(Value::bool(true))
    }
}
