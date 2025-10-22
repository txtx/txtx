use txtx_addon_kit::types::AuthorizationContext;
use txtx_addon_kit::{
    define_function, indoc,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{Type, Value},
    },
};

lazy_static! {
    pub static ref LIST_FUNCTIONS: Vec<FunctionSpecification> = vec![define_function! {
        Index => {
            name: "index",
            documentation: "`index` gets the entry from a list at the specified index.",
            example: indoc!{r#"
            output "entry" { 
                value = index(['a', 'b', 'c'], 1)
            }
            > entry: b
          "#},
            inputs: [
                list: {
                    documentation: "The list to retrieve an entry from.",
                    typing: vec![Type::array(Type::string()), Type::array(Type::integer()), Type::array(Type::buffer())] // todo: needs to be any
                },
                index: {
                    documentation: "The index of the entry to retrieve.",
                    typing: vec![Type::integer()]
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
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        let Value::Array(list) = args.get(0).unwrap() else {
            panic!("index function requires list for first input")
        };
        let Value::Integer(index) = args.get(1).unwrap() else {
            panic!("index function requires uint for second input")
        };
        match list.get(*index as usize) {
            Some(r) => Ok(r.clone()),
            None => panic!("index {} exceeds list bounds: {:?}", index, list),
        }
    }
}
