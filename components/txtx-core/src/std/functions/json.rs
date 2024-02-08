use jaq_core;
use txtx_addon_kit::{
    define_function,
    types::{
        functions::{FunctionImplementation, FunctionSpecification},
        typing::{Typing, Value},
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
                    typing: vec![Typing::String]
                },
                query: {
                    documentation: "Json query (see jq documentation)",
                    typing: vec![Typing::String]
                }
            ],
            output: {
                documentation: "Result of the query",
                typing: Typing::String
            },
        }
    },];
}

pub struct JsonQuery;
impl FunctionImplementation for JsonQuery {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        // todo(lgalabru): Parse string, parse query then run query on document
        // json!(args[0])
        // jaq_core::minimal()
        Value::Bool(true)
    }
}
