use super::arg_checker;
use kit::types::commands::{AssertionResult, ASSERTION_TYPE_ID};
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
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![define_function! {
        AssertEq => {
            name: "assert_eq",
            documentation: "`assert_eq` asserts that two values are equal.",
            example: indoc!{r#"
            output "assertion" { 
                value = std::assert_eq(action.example.result, 1)
            }
          "#},
            inputs: [
                left: {
                    documentation: "A value to compare.",
                    typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool(), Type::addon(""), Type::array(Type::null()), Type::arbitrary_object()]
                },
                right: {
                    documentation: "The value to compare against.",
                    typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool(), Type::addon(""), Type::array(Type::null()), Type::arbitrary_object()]
                }
            ],
            output: {
                documentation: "The result of the assertion.",
                typing: Type::addon(ASSERTION_TYPE_ID)
            },
        }
    },];
}

pub struct AssertEq;
impl FunctionImplementation for AssertEq {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let left = &args[0];
        let right = &args[1];
        if left.eq(right) {
            Ok(AssertionResult::Success.to_value())
        } else {
            Ok(AssertionResult::Failure(format!(
                "assertion failed: expected values to be equal: left: '{}', right: '{}'",
                left.to_string(),
                right.to_string()
            ))
            .to_value())
        }
    }
}
