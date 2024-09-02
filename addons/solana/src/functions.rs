use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{FunctionImplementation, FunctionSpecification},
    types::{Type, Value},
    AuthorizationContext,
};

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![define_function! {
        EncodeClarityValueSome => {
            name: "cv_some",
            documentation: "`stacks::cv_some` wraps the given Clarity value in a Clarity `Optional`.",
            example: indoc! {r#"
                output "some" { 
                  value = stacks::cv_some(stacks::cv_bool(true))
                }
                // > some: 0x0a03
                "#},
            inputs: [
                clarity_value: {
                    documentation: "A Clarity Value.",
                    typing: vec![Type::integer()]
                }
            ],
            output: {
                documentation: "The input Clarity value wrapped in a Clarity `Optional`.",
                typing: Type::string()
            },
        }
    }];
}

pub struct EncodeClarityValueSome;
impl FunctionImplementation for EncodeClarityValueSome {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}
