use jaq_core;
use txtx_addon_kit::{
    define_function,
    types::functions::{FunctionSpecification, FunctionImplementation, Typing, Value},
};

lazy_static! {
    pub static ref CORE_FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            ReadConstructAttribute => {
                name: "read_construct_attribute",
                documentation: "Read Construct attribute",
                example: "read_construct_attribute(variable.hello)",
                inputs: [
                    attribute_path: {
                        documentation: "Path of the attribute to read",
                        typing: Typing::String
                    }
                ],
                output: {
                    documentation: "Result of the query",
                    typing: Typing::String
                },
            }
        },
        define_function! {
            EvaluateString => {
                name: "eval_string",
                documentation: "Evaluate String Interpolation",
                example: "eval(|)",
                inputs: [
                    attribute_path: {
                        documentation: "Path of the attribute to read",
                        typing: Typing::String
                    }
                ],
                output: {
                    documentation: "Result of the query",
                    typing: Typing::String
                },
            }
        }];
}

pub struct ReadConstructAttribute;
impl FunctionImplementation for ReadConstructAttribute {
    fn check(ctx: &FunctionSpecification, args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: Vec<Value>) -> Value {
        println!("Executing {}", ctx.name);
        // todo(lgalabru): Parse string, parse query then run query on document
        // json!(args[0])
        // jaq_core::minimal()
        Value::Bool(true)
    }
}

