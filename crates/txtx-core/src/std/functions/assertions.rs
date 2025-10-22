use super::arg_checker;
use kit::types::commands::AssertionResult;
use kit::types::commands::ASSERTION_TYPE_ID;
use txtx_addon_kit::types::functions::FunctionSpecification;
use txtx_addon_kit::types::types::{Type, Value};
use txtx_addon_kit::types::AuthorizationContext;
use txtx_addon_kit::{
    define_function, indoc,
    types::{diagnostics::Diagnostic, functions::FunctionImplementation},
};

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
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
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool(), Type::addon(""), Type::array(Type::null()), Type::arbitrary_object()],
                        optional: false
                    },
                    right: {
                        documentation: "The value to compare against.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool(), Type::addon(""), Type::array(Type::null()), Type::arbitrary_object()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The result of the assertion.",
                    typing: Type::addon(ASSERTION_TYPE_ID)
                },
            }
        },
        define_function! {
            AssertNe => {
                name: "assert_ne",
                documentation: "`assert_ne` asserts that two values are not equal.",
                example: indoc!{r#"
                    output "assertion" { 
                        value = std::assert_ne(action.example.result, 1)
                    }
                "#},
                inputs: [
                    left: {
                        documentation: "A value to compare.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool(), Type::addon(""), Type::array(Type::null()), Type::arbitrary_object()],
                        optional: false
                    },
                    right: {
                        documentation: "The value to compare against.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool(), Type::addon(""), Type::array(Type::null()), Type::arbitrary_object()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The result of the assertion.",
                    typing: Type::addon(ASSERTION_TYPE_ID)
                },
            }
        },
        define_function! {
            AssertGt => {
                name: "assert_gt",
                documentation: "`assert_gt` asserts that the left value is greater than the right value.",
                example: indoc!{r#"
                    output "assertion" { 
                        value = std::assert_gt(action.example.result, 1)
                    }
                "#},
                inputs: [
                    left: {
                        documentation: "An integer or float to compare.",
                        typing: vec![Type::integer(), Type::float()],
                        optional: false
                    },
                    right: {
                        documentation: "An integer or float to compare against.",
                        typing: vec![Type::integer(), Type::float()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The result of the assertion.",
                    typing: Type::addon(ASSERTION_TYPE_ID)
                },
            }
        },
        define_function! {
            AssertGte => {
                name: "assert_gte",
                documentation: "`assert_gte` asserts that the left value is greater than or equal to the right value.",
                example: indoc!{r#"
                    output "assertion" { 
                        value = std::assert_gte(action.example.result, 1)
                    }
                "#},
                inputs: [
                    left: {
                        documentation: "An integer or float to compare.",
                        typing: vec![Type::integer(), Type::float()],
                        optional: false
                    },
                    right: {
                        documentation: "An integer or float to compare against.",
                        typing: vec![Type::integer(), Type::float()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The result of the assertion.",
                    typing: Type::addon(ASSERTION_TYPE_ID)
                },
            }
        },
        define_function! {
            AssertLt => {
                name: "assert_lt",
                documentation: "`assert_lt` asserts that the left value is less than the right value.",
                example: indoc!{r#"
                    output "assertion" { 
                        value = std::assert_lt(action.example.result, 1)
                    }
                "#},
                inputs: [
                    left: {
                        documentation: "An integer or float to compare.",
                        typing: vec![Type::integer(), Type::float()],
                        optional: false
                    },
                    right: {
                        documentation: "An integer or float to compare against.",
                        typing: vec![Type::integer(), Type::float()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The result of the assertion.",
                    typing: Type::addon(ASSERTION_TYPE_ID)
                },
            }
        },
        define_function! {
            AssertLte => {
                name: "assert_lte",
                documentation: "`assert_lte` asserts that the left value is less than or equal to the right value.",
                example: indoc!{r#"
                    output "assertion" { 
                        value = std::assert_lte(action.example.result, 1)
                    }
                "#},
                inputs: [
                    left: {
                        documentation: "An integer or float to compare.",
                        typing: vec![Type::integer(), Type::float()],
                        optional: false
                    },
                    right: {
                        documentation: "An integer or float to compare against.",
                        typing: vec![Type::integer(), Type::float()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The result of the assertion.",
                    typing: Type::addon(ASSERTION_TYPE_ID)
                },
            }
        }
    ];
}

pub struct AssertEq;
impl FunctionImplementation for AssertEq {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
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

pub struct AssertNe;
impl FunctionImplementation for AssertNe {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let left = &args[0];
        let right = &args[1];
        if left.ne(right) {
            Ok(AssertionResult::Success.to_value())
        } else {
            Ok(AssertionResult::Failure(format!(
                "assertion failed: expected values to be not equal: left: '{}', right: '{}'",
                left.to_string(),
                right.to_string()
            ))
            .to_value())
        }
    }
}

pub struct AssertGt;
impl FunctionImplementation for AssertGt {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let left = &args[0];
        let right = &args[1];
        match (left, right) {
            (Value::Integer(left_int), Value::Integer(right_int)) => {
                if left_int > right_int {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be greater than right value '{}'",
                        left_int, right_int
                    ))
                    .to_value())
                }
            }
            (Value::Float(left_float), Value::Float(right_float)) => {
                if left_float > right_float {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be greater than right value '{}'",
                        left_float, right_float
                    ))
                    .to_value())
                }
            }
            (Value::Float(left_float), Value::Integer(right_int)) => {
                if *left_float > *right_int as f64 {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be greater than right value '{}'",
                        left_float, right_int
                    ))
                    .to_value())
                }
            }
            (Value::Integer(left_int), Value::Float(right_float)) => {
                if *left_int as f64 > *right_float {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be greater than right value '{}'",
                        left_int, right_float
                    ))
                    .to_value())
                }
            }
            _ => unreachable!(),
        }
    }
}

pub struct AssertGte;
impl FunctionImplementation for AssertGte {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let left = &args[0];
        let right = &args[1];
        match (left, right) {
            (Value::Integer(left_int), Value::Integer(right_int)) => {
                if left_int >= right_int {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be greater than or equal to right value '{}'",
                        left_int, right_int
                    ))
                    .to_value())
                }
            }
            (Value::Float(left_float), Value::Float(right_float)) => {
                if left_float >= right_float {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be greater than or equal to right value '{}'",
                        left_float, right_float
                    ))
                    .to_value())
                }
            }
            (Value::Float(left_float), Value::Integer(right_int)) => {
                if *left_float >= *right_int as f64 {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be greater than or equal to right value '{}'",
                        left_float, right_int
                    ))
                    .to_value())
                }
            }
            (Value::Integer(left_int), Value::Float(right_float)) => {
                if *left_int as f64 >= *right_float {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be greater than or equal to right value '{}'",
                        left_int, right_float
                    ))
                    .to_value())
                }
            }
            _ => unreachable!(),
        }
    }
}

pub struct AssertLt;
impl FunctionImplementation for AssertLt {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let left = &args[0];
        let right = &args[1];
        match (left, right) {
            (Value::Integer(left_int), Value::Integer(right_int)) => {
                if left_int < right_int {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be less than right value '{}'",
                        left_int, right_int
                    ))
                    .to_value())
                }
            }
            (Value::Float(left_float), Value::Float(right_float)) => {
                if left_float < right_float {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be less than right value '{}'",
                        left_float, right_float
                    ))
                    .to_value())
                }
            }
            (Value::Float(left_float), Value::Integer(right_int)) => {
                if *left_float < *right_int as f64 {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be less than right value '{}'",
                        left_float, right_int
                    ))
                    .to_value())
                }
            }
            (Value::Integer(left_int), Value::Float(right_float)) => {
                if (*left_int as f64) < (*right_float) {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be less than right value '{}'",
                        left_int, right_float
                    ))
                    .to_value())
                }
            }
            _ => unreachable!(),
        }
    }
}

pub struct AssertLte;
impl FunctionImplementation for AssertLte {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &[Type],
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let left = &args[0];
        let right = &args[1];
        match (left, right) {
            (Value::Integer(left_int), Value::Integer(right_int)) => {
                if left_int <= right_int {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be less than or equal to right value '{}'",
                        left_int, right_int
                    ))
                    .to_value())
                }
            }
            (Value::Float(left_float), Value::Float(right_float)) => {
                if left_float <= right_float {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be less than or equal to right value '{}'",
                        left_float, right_float
                    ))
                    .to_value())
                }
            }
            (Value::Float(left_float), Value::Integer(right_int)) => {
                if *left_float <= *right_int as f64 {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be less than or equal to right value '{}'",
                        left_float, right_int
                    ))
                    .to_value())
                }
            }
            (Value::Integer(left_int), Value::Float(right_float)) => {
                if (*left_int as f64) <= (*right_float) {
                    Ok(AssertionResult::Success.to_value())
                } else {
                    Ok(AssertionResult::Failure(format!(
                        "assertion failed: expected left value '{}' to be less than or equal to right value '{}'",
                        left_int, right_float
                    ))
                    .to_value())
                }
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use kit::helpers::fs::FileLocation;
    use test_case::test_case;

    use super::*;

    fn get_spec_by_name(name: &str) -> FunctionSpecification {
        FUNCTIONS.iter().find(|f| f.name == name).cloned().unwrap()
    }

    fn dummy_auth_ctx() -> AuthorizationContext {
        AuthorizationContext { workspace_location: FileLocation::working_dir() }
    }

    #[test_case("assert_eq", Value::Integer(5), Value::Integer(5), AssertionResult::Success; "assert_eq success")]
    #[test_case("assert_eq", Value::String("foo".into()), Value::String("bar".into()), AssertionResult::Failure("assertion failed: expected values to be equal: left: 'foo', right: 'bar'".into()); "assert_eq failure")]
    #[test_case("assert_ne", Value::Bool(true), Value::Bool(false), AssertionResult::Success; "assert_ne success")]
    #[test_case("assert_ne", Value::Float(1.0), Value::Float(1.0), AssertionResult::Failure("assertion failed: expected values to be not equal: left: '1', right: '1'".into()); "assert_ne failure")]
    #[test_case("assert_gt", Value::Integer(10), Value::Integer(5), AssertionResult::Success; "assert_gt success int")]
    #[test_case("assert_gt", Value::Float(3.5), Value::Float(2.1), AssertionResult::Success; "assert_gt success float")]
    #[test_case("assert_gt", Value::Integer(2), Value::Integer(10), AssertionResult::Failure("assertion failed: expected left value '2' to be greater than right value '10'".into()); "assert_gt failure")]
    #[test_case("assert_gte", Value::Integer(7), Value::Integer(7), AssertionResult::Success; "assert_gte success equal")]
    #[test_case("assert_gte", Value::Float(8.0), Value::Float(7.9), AssertionResult::Success; "assert_gte success greater")]
    #[test_case("assert_gte", Value::Integer(1), Value::Integer(2), AssertionResult::Failure("assertion failed: expected left value '1' to be greater than or equal to right value '2'".into()); "assert_gte failure")]
    #[test_case("assert_lt", Value::Integer(1), Value::Integer(2), AssertionResult::Success; "assert_lt success")]
    #[test_case("assert_lt", Value::Float(3.0), Value::Float(2.1), AssertionResult::Failure("assertion failed: expected left value '3' to be less than right value '2.1'".into()); "assert_lt failure")]
    #[test_case("assert_lte", Value::Integer(5), Value::Integer(5), AssertionResult::Success; "assert_lte success equal")]
    #[test_case("assert_lte", Value::Float(1.1), Value::Float(2.2), AssertionResult::Success; "assert_lte success less")]
    #[test_case("assert_lte", Value::Integer(10), Value::Integer(2), AssertionResult::Failure("assertion failed: expected left value '10' to be less than or equal to right value '2'".into()); "assert_lte failure")]
    fn test_assertions(fn_spec_name: &str, left: Value, right: Value, expected: AssertionResult) {
        let fn_spec = get_spec_by_name(fn_spec_name);
        let args = vec![left, right];
        let result = (fn_spec.runner)(&fn_spec, &dummy_auth_ctx(), &args).unwrap();
        assert_eq!(result, expected.to_value());
    }

    #[test_case("assert_gt", Value::String("test".into()), Value::Integer(6), Diagnostic::error_from_string("function 'std::assert_gt' argument #1 (left) should be of type (integer,float), found string".into()); "assert_gt with string and integer")]
    #[test_case("assert_gte", Value::Bool(false), Value::Float(2.5), Diagnostic::error_from_string("function 'std::assert_gte' argument #1 (left) should be of type (integer,float), found bool".into()); "assert_gte with bool and float")]
    #[test_case("assert_lt", Value::Bool(true), Value::Float(3.5), Diagnostic::error_from_string("function 'std::assert_lt' argument #1 (left) should be of type (integer,float), found bool".into()); "assert_lt with bool and float")]
    #[test_case("assert_lte", Value::String("test".into()), Value::Integer(6), Diagnostic::error_from_string("function 'std::assert_lte' argument #1 (left) should be of type (integer,float), found string".into()); "assert_lte with string and integer")]
    fn test_invalid_inputs(fn_spec_name: &str, left: Value, right: Value, expected: Diagnostic) {
        let fn_spec = get_spec_by_name(fn_spec_name);
        let args = vec![left, right];
        let result = (fn_spec.runner)(&fn_spec, &dummy_auth_ctx(), &args).unwrap_err();
        assert_eq!(result, expected);
    }
}
