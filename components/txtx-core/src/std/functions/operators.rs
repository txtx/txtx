use jaq_core;
use txtx_addon_kit::{
    define_function,
    types::{
        functions::{FunctionImplementation, FunctionSpecification},
        typing::{Typing, Value},
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
                        typing: vec![Typing::SignedInteger]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::SignedInteger
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
                        typing: vec![Typing::Bool]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::Bool
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
                        typing: vec![Typing::Bool]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::Bool]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::Bool
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
                        typing: vec![Typing::Bool]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::Bool]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::String
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
                        typing: vec![Typing::SignedInteger]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::SignedInteger]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::SignedInteger
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
                        typing: vec![Typing::UnsignedInteger]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::UnsignedInteger]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::UnsignedInteger
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
                        typing: vec![Typing::Null, Typing::SignedInteger, Typing::Float, Typing::UnsignedInteger, Typing::String, Typing::Bool]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::Null, Typing::SignedInteger, Typing::Float, Typing::UnsignedInteger, Typing::String, Typing::Bool]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::Bool
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
                        typing: vec![Typing::Null, Typing::SignedInteger, Typing::Float, Typing::UnsignedInteger, Typing::String, Typing::Bool]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::Null, Typing::SignedInteger, Typing::Float, Typing::UnsignedInteger, Typing::String, Typing::Bool]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::Bool
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
                        typing: vec![Typing::Null, Typing::SignedInteger, Typing::Float, Typing::UnsignedInteger, Typing::String, Typing::Bool]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::Null, Typing::SignedInteger, Typing::Float, Typing::UnsignedInteger, Typing::String, Typing::Bool]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::Bool
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
                        typing: vec![Typing::Null, Typing::SignedInteger, Typing::Float, Typing::UnsignedInteger, Typing::String, Typing::Bool]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::Null, Typing::SignedInteger, Typing::Float, Typing::UnsignedInteger, Typing::String, Typing::Bool]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::Bool
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
                        typing: vec![Typing::Null, Typing::SignedInteger, Typing::Float, Typing::UnsignedInteger, Typing::String, Typing::Bool]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::Null, Typing::SignedInteger, Typing::Float, Typing::UnsignedInteger, Typing::String, Typing::Bool]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::Bool
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
                        typing: vec![Typing::Null, Typing::SignedInteger, Typing::Float, Typing::UnsignedInteger, Typing::String, Typing::Bool]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::Null, Typing::SignedInteger, Typing::Float, Typing::UnsignedInteger, Typing::String, Typing::Bool]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::Bool
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
                        typing: vec![Typing::UnsignedInteger]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::UnsignedInteger]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::UnsignedInteger
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
                        typing: vec![Typing::UnsignedInteger]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::UnsignedInteger]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::UnsignedInteger
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
                        typing: vec![Typing::UnsignedInteger]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::UnsignedInteger]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::UnsignedInteger
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
                        typing: vec![Typing::UnsignedInteger]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::UnsignedInteger]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::UnsignedInteger
                },
            }
        }
    ];
}

pub struct UnaryNegInteger;
impl FunctionImplementation for UnaryNegInteger {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct UnaryNotBool;
impl FunctionImplementation for UnaryNotBool {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryAndBool;
impl FunctionImplementation for BinaryAndBool {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryOrBool;
impl FunctionImplementation for BinaryOrBool {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryDivSignedInteger;
impl FunctionImplementation for BinaryDivSignedInteger {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryDivUnsignedInteger;
impl FunctionImplementation for BinaryDivUnsignedInteger {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryEq;
impl FunctionImplementation for BinaryEq {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryGreater;
impl FunctionImplementation for BinaryGreater {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryGreaterEq;
impl FunctionImplementation for BinaryGreaterEq {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryLess;
impl FunctionImplementation for BinaryLess {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryLessEq;
impl FunctionImplementation for BinaryLessEq {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryNotEq;
impl FunctionImplementation for BinaryNotEq {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryMinusUInt;
impl FunctionImplementation for BinaryMinusUInt {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryModuloUInt;
impl FunctionImplementation for BinaryModuloUInt {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryMulUInt;
impl FunctionImplementation for BinaryMulUInt {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        unimplemented!()
    }
}

pub struct BinaryPlusUInt;
impl FunctionImplementation for BinaryPlusUInt {
    fn check(ctx: &FunctionSpecification, args: &Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Value {
        let lhs = match args.get(0) {
            Some(Value::UnsignedInteger(val)) => val,
            _ => unreachable!(),
        };
        let rhs = match args.get(1) {
            Some(Value::UnsignedInteger(val)) => val,
            _ => unreachable!(),
        };
        Value::UnsignedInteger(lhs + rhs)
    }
}
