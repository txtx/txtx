use txtx_addon_kit::types::AuthorizationContext;
use txtx_addon_kit::{
    define_function, indoc,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{Type, Value},
    },
};

use super::arg_checker;

lazy_static! {
    pub static ref OPERATORS_FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            BinaryAndBool => {
                name: "and_bool",
                documentation: "`and_bool` returns the binary AND of the left- and right-hand-side arguments.",
                example: indoc!{r#"
                output "my_bool" { 
                  value = false && true
                }
                // > my_bool: false
                "#},
                inputs: [
                    lhs: {
                        documentation: "A `boolean` value.",
                        typing: vec![Type::bool()]
                    },
                    rhs: {
                        documentation: "A `boolean` value.",
                        typing: vec![Type::bool()]
                    }
                ],
                output: {
                    documentation: "The result of a binary AND between the two arguments.",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryOrBool => {
                name: "or_bool",
                documentation: "`or_bool` returns the binary OR of the left- and right-hand-side arguments.",
                example: indoc!{r#"
                output "my_bool" { 
                  value = false || true
                }
                // > my_bool: true
                "#},
                inputs: [
                    lhs: {
                        documentation: "A `boolean` value.",
                        typing: vec![Type::bool()]
                    },
                    rhs: {
                        documentation: "A `boolean` value.",
                        typing: vec![Type::bool()]
                    }
                ],
                output: {
                    documentation: "The result of a binary OR between the two arguments.",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryDivSignedInteger => {
                name: "div",
                documentation: "`div` returns the integer division of the left-hand-side argument by the right-hand-side argument, rounding any remainder down to the nearest integer.",
                example: indoc!{r#"
                output "my_int" { 
                  value = 11 / -3
                }
                // > my_int: -3
                "#},
                inputs: [
                    lhs: {
                        documentation: "The `int` dividend.",
                        typing: vec![Type::integer()]
                    },
                    rhs: {
                        documentation: "The `int` divisor.",
                        typing: vec![Type::integer()]
                    }
                ],
                output: {
                    documentation: "The result of dividing the dividend by the divisor.",
                    typing: Type::integer()
                },
            }
        },
        define_function! {
            BinaryEq => {
                name: "eq",
                documentation: "`eq` returns `true` if the left- and right-hand-side arguments are equal and `false` if they are not.",
                example: indoc!{r#"
                output "is_eq" { 
                  value = "arg" == "arg"
                }
                // > is_eq: true
                "#},
                inputs: [
                    lhs: {
                        documentation: "Any value.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool(), Type::addon(""), Type::array(Type::null()), Type::arbitrary_object()]
                    },
                    rhs: {
                        documentation: "Any value.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool(), Type::addon(""), Type::array(Type::null()), Type::arbitrary_object()]
                    }
                ],
                output: {
                    documentation: "The result of an equality check between the two inputs.",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryGreater => {
                name: "gt",
                documentation: "`gt` returns `true` if the left-hand-side argument is greater than the right-hand-side argument and `false` if it is not.",
                example: indoc!{r#"
                output "is_gt" { 
                  value = 2 > 1
                }
                // > is_gt: true
                "#},
                inputs: [
                    lhs: {
                        documentation: "An `integer`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "An `integer`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool()]
                    }
                ],
                output: {
                    documentation: "The result of checking if the left-hand-side argument is greater than the right-hand-side argument",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryGreaterEq => {
                name: "gte",
                documentation: "`gte` returns `true` if the left-hand-side argument is greater than or equal to the right-hand-side argument and `false` if it is not.",
                example: indoc!{r#"
                output "is_gte" { 
                  value = 2 >= 2
                }
                // > is_gte: true
                "#},
                inputs: [
                    lhs: {
                        documentation: "An `integer`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "An `integer`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool()]
                    }
                ],
                output: {
                    documentation: "The result of checking if the left-hand-side argument is greater than or equal to the right-hand-side argument",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryLess => {
                name: "lt",
                documentation: "`lt` returns `true` if the left-hand-side argument is less than the right-hand-side argument and `false` if it is not.",
                example: indoc!{r#"
                output "is_lt" { 
                  value = 2 < 1
                }
                // > is_lt: false
                "#},
                inputs: [
                    lhs: {
                        documentation: "An `integer`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "An `integer`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool()]
                    }
                ],
                output: {
                    documentation: "The result of checking if the left-hand-side argument is less than the right-hand-side argument",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryLessEq => {
                name: "lte",
                documentation: "`lte` returns `true` if the left-hand-side argument is less than or equal to the right-hand-side argument and `false` if it is not.",
                example: indoc!{r#"
                output "is_lte" { 
                  value = 2 <= 2
                }
                // > is_lte: true
                "#},
                inputs: [
                    lhs: {
                        documentation: "An `integer`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "An `integer`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool()]
                    }
                ],
                output: {
                    documentation: "The result of checking if the left-hand-side argument is less than or equal to the right-hand-side argument",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryNotEq => {
                name: "neq",
                documentation: "`enq` returns `true` if the left- and right-hand-side arguments are not equal and `false` otherwise.",
                example: indoc!{r#"
                output "is_neq" { 
                  value = "arg" != "arg"
                }
                // > is_neq: false
                "#},
                inputs: [
                    lhs: {
                        documentation: "An `integer`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "An `integer`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::integer(), Type::float(), Type::integer(), Type::string(), Type::bool()]
                    }
                ],
                output: {
                    documentation: "The result of a negated equality check between the two inputs.",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryMinus => {
                name: "minus",
                documentation: "`minus` returns the result of subtracting the right-hand-side `integer` argument from the left-hand-side `integer` argument.",
                example: indoc!{r#"
                output "my_integer" { 
                  value = 10 - 6
                }
                // > my_integer: 4
                "#},
                inputs: [
                    lhs: {
                      documentation: "The `integer` minuend.",
                      typing: vec![Type::integer()]
                    },
                    rhs: {
                        documentation: "The `integer` subtrahend.",
                        typing: vec![Type::integer()]
                    }
                ],
                output: {
                    documentation: "The result of the subtraction operation.",
                    typing: Type::integer()
                },
            }
        },
        define_function! {
            BinaryModulo => {
                name: "modulo",
                documentation: "`modulo` returns the remainder of dividing the left-hand-side `integer` argument by the right-hand-side `integer` argument.",
                example: indoc!{r#"
                  output "my_mod" { 
                      value = 10 % 3
                  }
                  // > my_mod: 1
              "#},
                inputs: [
                    lhs: {
                        documentation: "The `integer` dividend.",
                        typing: vec![Type::integer()]
                    },
                    rhs: {
                        documentation: "The `integer` divisor.",
                        typing: vec![Type::integer()]
                    }
                ],
                output: {
                    documentation: "The remainder of the division operation.",
                    typing: Type::integer()
                },
            }
        },
        define_function! {
            BinaryMul => {
                name: "multiply",
                documentation: "`multiply` returns the product of the left-hand-side `integer` argument and the right-hand-side `integer` argument.",
                example: indoc!{r#"
                  output "my_product" { 
                      value = 10 * 5
                  }
                  // > my_product: 50
              "#},
                inputs: [
                    lhs: {
                        documentation: "The first `integer` operand.",
                        typing: vec![Type::integer()],
                        optional: false
                    },
                    rhs: {
                        documentation: "The second `integer` operand.",
                        typing: vec![Type::integer()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The result of the multiplication operation.",
                    typing: Type::integer()
                },
            }
        },
        define_function! {
            BinaryPlus => {
                name: "add",
                documentation: "`add` returns the sum of the left-hand-side `integer` argument and the right-hand-side `integer` argument.",
                example: indoc!{r#"
                  output "my_sum" { 
                      value = 10 + 5
                  }
                  // > my_sum: 15
              "#},
                inputs: [
                    lhs: {
                        documentation: "The first `integer` operand.",
                        typing: vec![Type::integer()]
                    },
                    rhs: {
                        documentation: "The second `integer` operand.",
                        typing: vec![Type::integer()]
                    }
                ],
                output: {
                    documentation: "The result of the addition operation.",
                    typing: Type::integer()
                },
            }
        },
        define_function! {
            UnaryNegInteger => {
                name: "neg_integer",
                documentation: "Returns the negation of the given integer.",
                example: "// Coming soon",
                inputs: [
                    value: {
                        documentation: "An integer value.",
                        typing: vec![Type::integer()]
                    }
                ],
                output: {
                    documentation: "The negated integer value.",
                    typing: Type::integer()
                },
            }
        },
        define_function! {
            UnaryNotBool => {
                name: "not_bool",
                documentation: "Returns the logical negation of the given boolean value.",
                example: "// Coming soon",
                inputs: [
                    value: {
                        documentation: "A boolean value.",
                        typing: vec![Type::bool()]
                    }
                ],
                output: {
                    documentation: "The logical negation of the input boolean value.",
                    typing: Type::bool()
                },
            }
        },
    ];
}

pub struct UnaryNegInteger;
impl FunctionImplementation for UnaryNegInteger {
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
        _args: &[Value],
    ) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct UnaryNotBool;
impl FunctionImplementation for UnaryNotBool {
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
        _args: &[Value],
    ) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryAndBool;
impl FunctionImplementation for BinaryAndBool {
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
        let Some(Value::Bool(lhs)) = args.get(0) else { unreachable!() };
        let Some(Value::Bool(rhs)) = args.get(1) else { unreachable!() };
        Ok(Value::bool(*lhs && *rhs))
    }
}

pub struct BinaryOrBool;
impl FunctionImplementation for BinaryOrBool {
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
        let Some(Value::Bool(lhs)) = args.get(0) else { unreachable!() };
        let Some(Value::Bool(rhs)) = args.get(1) else { unreachable!() };
        Ok(Value::bool(*lhs || *rhs))
    }
}

pub struct BinaryDivSignedInteger;
impl FunctionImplementation for BinaryDivSignedInteger {
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
        let Some(Value::Integer(lhs)) = args.get(0) else { unreachable!() };
        let Some(Value::Integer(rhs)) = args.get(1) else { unreachable!() };
        if rhs.eq(&0) {
            Err(Diagnostic::error_from_string("cannot divide by zero".to_string()))
        } else {
            Ok(Value::integer(lhs.saturating_div(*rhs)))
        }
    }
}

pub struct BinaryEq;
impl FunctionImplementation for BinaryEq {
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
        let lhs = args.get(0).unwrap();
        let rhs = args.get(1).unwrap();
        Ok(Value::bool(lhs.eq(rhs)))
    }
}

pub struct BinaryGreater;
impl FunctionImplementation for BinaryGreater {
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
        let Some(Value::Integer(lhs)) = args.get(0) else { unreachable!() };
        let Some(Value::Integer(rhs)) = args.get(1) else { unreachable!() };
        Ok(Value::bool(lhs.gt(&rhs)))
    }
}

pub struct BinaryGreaterEq;
impl FunctionImplementation for BinaryGreaterEq {
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
        let Some(Value::Integer(lhs)) = args.get(0) else { unreachable!() };
        let Some(Value::Integer(rhs)) = args.get(1) else { unreachable!() };
        Ok(Value::bool(lhs.ge(&rhs)))
    }
}

pub struct BinaryLess;
impl FunctionImplementation for BinaryLess {
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
        let Some(Value::Integer(lhs)) = args.get(0) else { unreachable!() };
        let Some(Value::Integer(rhs)) = args.get(1) else { unreachable!() };
        Ok(Value::bool(lhs.lt(&rhs)))
    }
}

pub struct BinaryLessEq;
impl FunctionImplementation for BinaryLessEq {
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
        let Some(Value::Integer(lhs)) = args.get(0) else { unreachable!() };
        let Some(Value::Integer(rhs)) = args.get(1) else { unreachable!() };
        Ok(Value::bool(lhs.le(&rhs)))
    }
}

pub struct BinaryNotEq;
impl FunctionImplementation for BinaryNotEq {
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
        let Some(Value::Integer(lhs)) = args.get(0) else { unreachable!() };
        let Some(Value::Integer(rhs)) = args.get(1) else { unreachable!() };
        Ok(Value::bool(!lhs.eq(&rhs)))
    }
}

pub struct BinaryMinus;
impl FunctionImplementation for BinaryMinus {
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
        let Some(Value::Integer(lhs)) = args.get(0) else { unreachable!() };
        let Some(Value::Integer(rhs)) = args.get(1) else { unreachable!() };
        Ok(Value::integer(lhs - rhs))
    }
}

pub struct BinaryModulo;
impl FunctionImplementation for BinaryModulo {
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
        let Some(Value::Integer(lhs)) = args.get(0) else { unreachable!() };
        let Some(Value::Integer(rhs)) = args.get(1) else { unreachable!() };
        Ok(Value::integer(lhs.rem_euclid(*rhs)))
    }
}

pub struct BinaryMul;
impl FunctionImplementation for BinaryMul {
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
        let lhs = args.get(0).unwrap().as_integer().unwrap();
        let rhs = args.get(1).unwrap().as_integer().unwrap();
        Ok(Value::integer(lhs.saturating_mul(rhs)))
    }
}

pub struct BinaryPlus;
impl FunctionImplementation for BinaryPlus {
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
        let Some(Value::Integer(lhs)) = args.get(0) else { unreachable!() };
        let Some(Value::Integer(rhs)) = args.get(1) else { unreachable!() };
        Ok(Value::integer(lhs + rhs))
    }
}
