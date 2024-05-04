use txtx_addon_kit::{
    define_function,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{PrimitiveValue, Type, Value},
    },
};

lazy_static! {
    pub static ref LIST_FUNCTIONS: Vec<FunctionSpecification> = vec![define_function! {
        Index => {
            name: "index",
            documentation: "Get the entry from a list at the specified index",
            example: "index(['a', 'b', 'c'], 1)",
            inputs: [
                list: {
                    documentation: "The list",
                    typing: vec![Type::string(), Type::uint(), Type::int()] // todo: needs to be any
                },
                index: {
                    documentation: "The index",
                    typing: vec![Type::uint()]
                }
            ],
            output: {
                documentation: "The entry from list at the specified index",
                typing: Type::string() // todo: the result can be any type, but our types don't have an any
            },
        }
    },];
}

pub struct Index;
impl FunctionImplementation for Index {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let Value::Array(list) = args.get(0).unwrap() else {
            panic!("index function requires list for first input")
        };
        let Value::Primitive(PrimitiveValue::UnsignedInteger(index)) = args.get(1).unwrap() else {
            panic!("index function requires uint for second input")
        };
        match list.get(*index as usize) {
            Some(r) => Ok(r.clone()),
            None => panic!("index {} exceeds list bounds: {:?}", index, list),
        }
    }
}
