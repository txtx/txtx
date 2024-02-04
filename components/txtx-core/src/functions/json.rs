use jaq_core;
use txtx_addon_kit::{
    define_native_function,
    types::functions::{FunctionDeclaration, FunctionImplementation, Typing, Value},
};


lazy_static! {
    pub static ref JSON_FUNCTIONS: Vec<FunctionDeclaration> = vec![define_native_function! {
        JsonQuery => {
            name: "json_query",
            documentation: "Query Json data",
            example: "json_query(\"{ \"message\": \"Hello world!\" }\", \".root\")",
            inputs: [
                decoded_json: {
                    documentation: "Json document",
                    typing: Typing::String
                },
                query: {
                    documentation: "Json query (see jq documentation)",
                    typing: Typing::String
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
    fn check(ctx: &FunctionDeclaration, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionDeclaration, args: Vec<Value>) -> Value {
        println!("Executing {}", ctx.name);
        // todo(lgalabru): Parse string, parse query then run query on document
        // json!(args[0])
        // jaq_core::minimal()
        Value::Bool(true)
    }
}
