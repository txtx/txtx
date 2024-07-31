use kit::types::AuthorizationContext;
use txtx_addon_kit::{
    define_function, indoc,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{PrimitiveValue, Type, Value},
    },
};

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
                name: "div_int",
                documentation: "`div_int` returns the integer division of the left-hand-side argument by the right-hand-side argument, rounding any remainder down to the nearest integer.",
                example: indoc!{r#"
                output "my_int" { 
                  value = 11 / -3
                }
                // > my_int: -3
                "#},
                inputs: [
                    lhs: {
                        documentation: "The `int` dividend.",
                        typing: vec![Type::int()]
                    },
                    rhs: {
                        documentation: "The `int` divisor.",
                        typing: vec![Type::int()]
                    }
                ],
                output: {
                    documentation: "The result of dividing the dividend by the divisor.",
                    typing: Type::int()
                },
            }
        },
        define_function! {
            BinaryDivUnsignedInteger => {
                name: "div_uint",
                documentation: "`div_int` returns the integer division of the left-hand-side argument by the right-hand-side argument, rounding any remainder down to the nearest integer.",
                example: indoc!{r#"
                output "my_uint" { 
                  value = 11 / 3
                }
                // > my_uint: 3
                "#},
                inputs: [
                    lhs: {
                        documentation: "The `uint` dividend.",
                        typing: vec![Type::int()]
                    },
                    rhs: {
                        documentation: "The `uint` divisor.",
                        typing: vec![Type::int()]
                    }
                ],
                output: {
                    documentation: "The result of dividing the dividend by the divisor.",
                    typing: Type::int()
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
                        documentation: "An `int`, `uint`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "An `int`, `uint`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
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
                        documentation: "An `int`, `uint`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "An `int`, `uint`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
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
                        documentation: "An `int`, `uint`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "An `int`, `uint`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
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
                        documentation: "An `int`, `uint`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "An `int`, `uint`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
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
                        documentation: "An `int`, `uint`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "An `int`, `uint`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
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
                        documentation: "An `int`, `uint`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "An `int`, `uint`, `float`, `string`, `boolean` or `null` value.",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    }
                ],
                output: {
                    documentation: "The result of a negated equality check between the two inputs.",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryMinusUInt => {
                name: "minus_uint",
                documentation: "`minus_uint` returns the result of subtracting the right-hand-side `uint` argument from the left-hand-side `uint` argument.",
                example: indoc!{r#"
                output "my_uint" { 
                  value = 10 - 6
                }
                // > my_uint: 4
                "#},
                inputs: [
                    lhs: {
                      documentation: "The `uint` minuend.",
                      typing: vec![Type::int()]
                    },
                    rhs: {
                        documentation: "The `uint` subtrahend.",
                        typing: vec![Type::int()]
                    }
                ],
                output: {
                    documentation: "The result of the subtraction operation.",
                    typing: Type::uint()
                },
            }
        },
        define_function! {
            BinaryModuloUInt => {
                name: "modulo_uint",
                documentation: "`modulo_uint` returns the remainder of dividing the left-hand-side `uint` argument by the right-hand-side `uint` argument.",
                example: indoc!{r#"
                  output "my_mod" { 
                      value = 10 % 3
                  }
                  // > my_mod: 1
              "#},
                inputs: [
                    lhs: {
                        documentation: "The `uint` dividend.",
                        typing: vec![Type::uint()]
                    },
                    rhs: {
                        documentation: "The `uint` divisor.",
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "The remainder of the division operation.",
                    typing: Type::uint()
                },
            }
        },
        define_function! {
            BinaryMulUInt => {
                name: "multiply_uint",
                documentation: "`multiply_uint` returns the product of the left-hand-side `uint` argument and the right-hand-side `uint` argument.",
                example: indoc!{r#"
                  output "my_product" { 
                      value = 10 * 5
                  }
                  // > my_product: 50
              "#},
                inputs: [
                    lhs: {
                        documentation: "The first `uint` operand.",
                        typing: vec![Type::uint()]
                    },
                    rhs: {
                        documentation: "The second `uint` operand.",
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "The result of the multiplication operation.",
                    typing: Type::uint()
                },
            }
        },
        define_function! {
            BinaryPlusUInt => {
                name: "add_uint",
                documentation: "`add_uint` returns the sum of the left-hand-side `uint` argument and the right-hand-side `uint` argument.",
                example: indoc!{r#"
                  output "my_sum" { 
                      value = 10 + 5
                  }
                  // > my_sum: 15
              "#},
                inputs: [
                    lhs: {
                        documentation: "The first `uint` operand.",
                        typing: vec![Type::uint()]
                    },
                    rhs: {
                        documentation: "The second `uint` operand.",
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "The result of the addition operation.",
                    typing: Type::uint()
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
                        typing: vec![Type::int()]
                    }
                ],
                output: {
                    documentation: "The negated integer value.",
                    typing: Type::int()
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

pub struct UnaryNotBool;
impl FunctionImplementation for UnaryNotBool {
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

pub struct BinaryAndBool;
impl FunctionImplementation for BinaryAndBool {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::Bool(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::Bool(rhs))) = args.get(1) else {
            unreachable!()
        };
        Ok(Value::bool(*lhs && *rhs))
    }
}

pub struct BinaryOrBool;
impl FunctionImplementation for BinaryOrBool {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::Bool(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::Bool(rhs))) = args.get(1) else {
            unreachable!()
        };
        Ok(Value::bool(*lhs || *rhs))
    }
}

pub struct BinaryDivSignedInteger;
impl FunctionImplementation for BinaryDivSignedInteger {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::SignedInteger(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::SignedInteger(rhs))) = args.get(1) else {
            unreachable!()
        };
        if rhs.eq(&0) {
            Err(Diagnostic::error_from_string(
                "cannot divide by zero".to_string(),
            ))
        } else {
            Ok(Value::int(lhs.saturating_div(*rhs)))
        }
    }
}

pub struct BinaryDivUnsignedInteger;
impl FunctionImplementation for BinaryDivUnsignedInteger {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(rhs))) = args.get(1) else {
            unreachable!()
        };
        if rhs.eq(&0) {
            Err(Diagnostic::error_from_string(
                "cannot divide by zero".to_string(),
            ))
        } else {
            Ok(Value::uint(lhs.saturating_div(*rhs)))
        }
    }
}

pub struct BinaryEq;
impl FunctionImplementation for BinaryEq {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::UnsignedInteger(lhs))) => {
                let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(rhs))) = args.get(1)
                else {
                    unreachable!()
                };
                Ok(Value::bool(lhs.eq(rhs)))
            }
            Some(Value::Primitive(PrimitiveValue::SignedInteger(lhs))) => {
                let Some(Value::Primitive(PrimitiveValue::SignedInteger(rhs))) = args.get(1) else {
                    unreachable!()
                };
                Ok(Value::bool(lhs.eq(rhs)))
            }
            Some(Value::Primitive(PrimitiveValue::Float(lhs))) => {
                let Some(Value::Primitive(PrimitiveValue::Float(rhs))) = args.get(1) else {
                    unreachable!()
                };
                Ok(Value::bool(lhs.eq(rhs)))
            }
            Some(Value::Primitive(PrimitiveValue::Bool(lhs))) => {
                let Some(Value::Primitive(PrimitiveValue::Bool(rhs))) = args.get(1) else {
                    unreachable!()
                };
                Ok(Value::bool(lhs.eq(rhs)))
            }
            Some(Value::Primitive(PrimitiveValue::String(lhs))) => {
                let Some(Value::Primitive(PrimitiveValue::String(rhs))) = args.get(1) else {
                    unreachable!()
                };
                Ok(Value::bool(lhs.eq(rhs)))
            }
            Some(Value::Primitive(PrimitiveValue::Null)) => {
                let Some(Value::Primitive(PrimitiveValue::Null)) = args.get(1) else {
                    return Ok(Value::bool(false));
                };
                Ok(Value::bool(true))
            }
            _ => unreachable!(),
        }
    }
}

pub struct BinaryGreater;
impl FunctionImplementation for BinaryGreater {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(rhs))) = args.get(1) else {
            unreachable!()
        };
        Ok(Value::bool(lhs.gt(rhs)))
    }
}

pub struct BinaryGreaterEq;
impl FunctionImplementation for BinaryGreaterEq {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(rhs))) = args.get(1) else {
            unreachable!()
        };
        Ok(Value::bool(lhs.ge(rhs)))
    }
}

pub struct BinaryLess;
impl FunctionImplementation for BinaryLess {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(rhs))) = args.get(1) else {
            unreachable!()
        };
        Ok(Value::bool(lhs.lt(rhs)))
    }
}

pub struct BinaryLessEq;
impl FunctionImplementation for BinaryLessEq {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(rhs))) = args.get(1) else {
            unreachable!()
        };
        Ok(Value::bool(lhs.le(rhs)))
    }
}

pub struct BinaryNotEq;
impl FunctionImplementation for BinaryNotEq {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(rhs))) = args.get(1) else {
            unreachable!()
        };
        Ok(Value::bool(!lhs.eq(rhs)))
    }
}

pub struct BinaryMinusUInt;
impl FunctionImplementation for BinaryMinusUInt {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(rhs))) = args.get(1) else {
            unreachable!()
        };
        Ok(Value::uint(lhs.saturating_sub(*rhs)))
    }
}

pub struct BinaryModuloUInt;
impl FunctionImplementation for BinaryModuloUInt {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(rhs))) = args.get(1) else {
            unreachable!()
        };
        Ok(Value::uint(lhs.rem_euclid(*rhs)))
    }
}

pub struct BinaryMulUInt;
impl FunctionImplementation for BinaryMulUInt {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(rhs))) = args.get(1) else {
            unreachable!()
        };
        Ok(Value::uint(lhs.saturating_mul(*rhs)))
    }
}

pub struct BinaryPlusUInt;
impl FunctionImplementation for BinaryPlusUInt {
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
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(rhs))) = args.get(1) else {
            unreachable!()
        };
        Ok(Value::uint(lhs + rhs))
    }
}
