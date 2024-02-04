use txtx_addon_kit::{define_native_function, types::functions::{FunctionImplementation, NativeFunction, TypeSignature, Value}};
use jaq_core;

lazy_static! {
    pub static ref STACKS_NATIVE_FUNCTIONS: Vec<NativeFunction> = vec![
        define_native_function! {
            JsonQuery => {
                name: "json_query",
                documentation: "Query Json data",
                example: "json_query(\"{ \"message\": \"Hello world!\" }\", \".root\")",
                inputs: [
                    decoded_json: {
                        documentation: "Json document",
                        type_signature: TypeSignature::String
                    },
                    query: {
                        documentation: "Json query (see jq documentation)",
                        type_signature: TypeSignature::String
                    }
                ],
                output: {
                    documentation: "Result of the query",
                    type_signature: TypeSignature::String
                },
            }
        },
    ];
}

pub struct JsonQuery;
impl FunctionImplementation for JsonQuery {
    fn check(ctx: &NativeFunction, args: Vec<TypeSignature>) -> TypeSignature {
        unimplemented!()
    }

    fn run(ctx: &NativeFunction, args: Vec<Value>) -> Value {
        println!("Executing {}", ctx.name);
        // todo(lgalabru): Parse string, parse query then run query on document
        // json!(args[0])
        // jaq_core::minimal()
        Value::Bool(true)
    }
}
