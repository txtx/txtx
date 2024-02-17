use txtx_addon_kit::{
    define_function,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{PrimitiveValue, Typing, Value},
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
                        typing: vec![Typing::int()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::int()
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
                        typing: vec![Typing::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::bool()
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
                        typing: vec![Typing::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::bool()
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
                        typing: vec![Typing::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::string()
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
                        typing: vec![Typing::int()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::int()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::int()
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
                        typing: vec![Typing::uint()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::uint()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::uint()
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
                        typing: vec![Typing::null(), Typing::int(), Typing::float(), Typing::uint(), Typing::string(), Typing::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::null(), Typing::int(), Typing::float(), Typing::uint(), Typing::string(), Typing::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::bool()
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
                        typing: vec![Typing::null(), Typing::int(), Typing::float(), Typing::uint(), Typing::string(), Typing::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::null(), Typing::int(), Typing::float(), Typing::uint(), Typing::string(), Typing::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::bool()
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
                        typing: vec![Typing::null(), Typing::int(), Typing::float(), Typing::uint(), Typing::string(), Typing::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::null(), Typing::int(), Typing::float(), Typing::uint(), Typing::string(), Typing::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::bool()
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
                        typing: vec![Typing::null(), Typing::int(), Typing::float(), Typing::uint(), Typing::string(), Typing::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::null(), Typing::int(), Typing::float(), Typing::uint(), Typing::string(), Typing::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::bool()
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
                        typing: vec![Typing::null(), Typing::int(), Typing::float(), Typing::uint(), Typing::string(), Typing::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::null(), Typing::int(), Typing::float(), Typing::uint(), Typing::string(), Typing::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::bool()
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
                        typing: vec![Typing::null(), Typing::int(), Typing::float(), Typing::uint(), Typing::string(), Typing::bool()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::null(), Typing::int(), Typing::float(), Typing::uint(), Typing::string(), Typing::bool()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::bool()
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
                        typing: vec![Typing::uint()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::uint()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::uint()
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
                        typing: vec![Typing::uint()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::uint()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::uint()
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
                        typing: vec![Typing::uint()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::uint()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::uint()
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
                        typing: vec![Typing::uint()]
                    },
                    rhs: {
                        documentation: "",
                        typing: vec![Typing::uint()]
                    }
                ],
                output: {
                    documentation: "",
                    typing: Typing::uint()
                },
            }
        }
    ];
}

pub struct UnaryNegInteger;
impl FunctionImplementation for UnaryNegInteger {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct UnaryNotBool;
impl FunctionImplementation for UnaryNotBool {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryAndBool;
impl FunctionImplementation for BinaryAndBool {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryOrBool;
impl FunctionImplementation for BinaryOrBool {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryDivSignedInteger;
impl FunctionImplementation for BinaryDivSignedInteger {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryDivUnsignedInteger;
impl FunctionImplementation for BinaryDivUnsignedInteger {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryEq;
impl FunctionImplementation for BinaryEq {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryGreater;
impl FunctionImplementation for BinaryGreater {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryGreaterEq;
impl FunctionImplementation for BinaryGreaterEq {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryLess;
impl FunctionImplementation for BinaryLess {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryLessEq;
impl FunctionImplementation for BinaryLessEq {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryNotEq;
impl FunctionImplementation for BinaryNotEq {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryMinusUInt;
impl FunctionImplementation for BinaryMinusUInt {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryModuloUInt;
impl FunctionImplementation for BinaryModuloUInt {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryMulUInt;
impl FunctionImplementation for BinaryMulUInt {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, _args: &Vec<Value>) -> Result<Value, Diagnostic> {
        unimplemented!()
    }
}

pub struct BinaryPlusUInt;
impl FunctionImplementation for BinaryPlusUInt {
    fn check(_ctx: &FunctionSpecification, _args: &Vec<Typing>) -> Result<Typing, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let lhs = match args.get(0) {
            Some(Value::Primitive(PrimitiveValue::UnsignedInteger(val))) => val,
            _ => unreachable!(),
        };
        let rhs = match args.get(1) {
            Some(Value::Primitive(PrimitiveValue::UnsignedInteger(val))) => val,
            _ => unreachable!(),
        };
        Ok(Value::uint(lhs + rhs))
    }
}
