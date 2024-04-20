use txtx_addon_kit::{
    define_function,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{PrimitiveValue, Type, Value},
    },
};

lazy_static! {
    pub static ref OPERATORS_FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            UnaryNegInteger => {
                name: "neg_integer",
                documentation: "",
                example: "",
                inputs: [
                    value: {
                        documentation: "",
                        typing: vec![Type::int()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::int()
                },
            }
        },
        define_function! {
            UnaryNotBool => {
                name: "not_bool",
                documentation: "",
                example: "",
                inputs: [
                    value: {
                        documentation: "",
                        typing: vec![Type::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryAndBool => {
                name: "and_bool",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryOrBool => {
                name: "or_bool",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::string()
                },
            }
        },
        define_function! {
            BinaryDivSignedInteger => {
                name: "div_int",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::int()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::int()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::int()
                },
            }
        },
        define_function! {
            BinaryDivUnsignedInteger => {
                name: "div_uint",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::uint()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::uint()
                },
            }
        },
        define_function! {
            BinaryEq => {
                name: "eq",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryGreater => {
                name: "gt",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryGreaterEq => {
                name: "gte",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryLess => {
                name: "lt",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryLessEq => {
                name: "lte",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryNotEq => {
                name: "neq",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::null(), Type::int(), Type::float(), Type::uint(), Type::string(), Type::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::bool()
                },
            }
        },
        define_function! {
            BinaryMinusUInt => {
                name: "minus_uint",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::uint()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::uint()
                },
            }
        },
        define_function! {
            BinaryModuloUInt => {
                name: "modulo_uint",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::uint()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::uint()
                },
            }
        },
        define_function! {
            BinaryMulUInt => {
                name: "multiply_uint",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::uint()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::uint()
                },
            }
        },
        define_function! {
            BinaryPlusUInt => {
                name: "add_uint",
                documentation: "",
                example: "",
                inputs: [
                    lhs: {
                        documentation: "",
                        typing: vec![Type::uint()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Type::uint()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Type::uint()
                },
            }
        }
    ];
}

pub struct UnaryNegInteger;
impl FunctionImplementation for UnaryNegInteger {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct UnaryNotBool;
impl FunctionImplementation for UnaryNotBool {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryAndBool;
impl FunctionImplementation for BinaryAndBool {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
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
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
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
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
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
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
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
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(rhs))) = args.get(1) else {
            unreachable!()
        };
        Ok(Value::bool(lhs.eq(rhs)))
    }
}

pub struct BinaryGreater;
impl FunctionImplementation for BinaryGreater {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
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
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
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
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
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
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
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
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
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
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
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
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
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
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
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
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(lhs))) = args.get(0) else {
            unreachable!()
        };
        let Some(Value::Primitive(PrimitiveValue::UnsignedInteger(rhs))) = args.get(1) else {
            unreachable!()
        };
        Ok(Value::uint(lhs + rhs))
    }
}
