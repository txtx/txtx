use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    functions::{FunctionImplementation, FunctionSpecification},
    types::{Type, Value},
    AuthorizationContext,
};

use crate::{
    codec::BitcoinOpcode,
    functions::{arg_checker, to_diag},
    typing::{BitcoinValue, BITCOIN_OPCODE},
};

lazy_static! {
    pub static ref CONSTANTS: Vec<FunctionSpecification> = vec![
        define_function! {
            OpZero => {
                name: "op_0",
                documentation: "`btc::op_0` pushes 0 onto the stack.",
                example: indoc! {r#"
                    output "opcode" {
                        value = btc::op_0()
                    }                
                    // > opcode: 0x00
                "#},
                inputs: [],
                output: {
                    documentation: "`0x00`",
                    typing: Type::addon(BITCOIN_OPCODE)
                },
            }
        },
        define_function! {
            PushData => {
                name: "op_pushdata",
                documentation: "`btc::op_pushdata` pushes the provided length byte and data bytes onto the stack, ensuring that the data's length matches the provided length.",
                example: indoc! {r#"
                    output "opcode" {
                        value = btc::op_pushdata(1, "ff")
                    }                
                    // > opcode: 0x01ff
                "#},
                inputs: [
                    length: {
                        documentation: "The number of bytes, between 1 and 75, that will be pushed to the stack.",
                        typing: vec![Type::integer()],
                        optional: false
                    },
                    bytes: {
                        documentation: "The hex-encoded bytes that will be pushed to the stack, which should have length equal to the first argument.",
                        typing: vec![Type::string(), Type::buffer()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The number of bytes followed by the data, all encoded in hex.",
                    typing: Type::addon(BITCOIN_OPCODE)
                },
            }
        },
        define_function! {
            PushData1 => {
                name: "op_pushdata1",
                documentation: "`btc::op_pushdata1` pushes the `OP_PUSHDATA1` opcode, the provided length byte, and the data bytes onto the stack, ensuring that the data's length matches the provided length.",
                example: indoc! {r#"
                    output "opcode" {
                        value = btc::op_pushdata1(1, "ff")
                    }                
                    // > opcode: 0xc401ff
                "#},
                inputs: [
                    length: {
                        documentation: "The number of bytes that will be pushed to the stack.",
                        typing: vec![Type::integer()],
                        optional: false
                    },
                    bytes: {
                        documentation: "The hex-encoded bytes that will be pushed to the stack, which should have length equal to the first argument.",
                        typing: vec![Type::string(), Type::buffer()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The `OP_PUSHDATA1` opcode, the number of bytes, and the data, all encoded in hex.",
                    typing: Type::addon(BITCOIN_OPCODE)
                },
            }
        },
    ];
}

#[derive(Clone)]
pub struct OpZero;
impl FunctionImplementation for OpZero {
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
        Ok(BitcoinValue::opcode(BitcoinOpcode::Op0.get_code()))
    }
}

#[derive(Clone)]
pub struct PushData;
impl FunctionImplementation for PushData {
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
        let expected_bytes_length = args.get(0).unwrap().as_integer().unwrap();
        let mut data = args
            .get(1)
            .unwrap()
            .try_get_buffer_bytes_result()
            .map_err(|e| to_diag(fn_spec, format!("argument must be decodable to hex: {e}")))?
            .unwrap();

        let expected_bytes_length: u8 = expected_bytes_length
            .try_into()
            .ok()
            .and_then(|l| if l >= 1 && l <= 75 { Some(l) } else { None })
            .ok_or(to_diag(fn_spec, format!("byte length must be between 1 and 75")))?;

        if data.len() != expected_bytes_length as usize {
            return Err(to_diag(
                fn_spec,
                format!(
                    "provided length ({}) does not equal data byte length ({})",
                    expected_bytes_length,
                    data.len()
                ),
            ));
        }

        let mut bytes: Vec<u8> = BitcoinOpcode::OpPushData.get_code();
        bytes.push(expected_bytes_length);

        bytes.append(&mut data);

        Ok(BitcoinValue::opcode(bytes))
    }
}
#[derive(Clone)]
pub struct PushData1;
impl FunctionImplementation for PushData1 {
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
        let expected_bytes_length = args.get(0).unwrap().as_integer().unwrap();
        let mut data = args
            .get(1)
            .unwrap()
            .try_get_buffer_bytes_result()
            .map_err(|e| diagnosed_error!("argument must be decodable to hex: {e}"))?
            .unwrap();

        let expected_bytes_length: u8 = expected_bytes_length
            .try_into()
            .map_err(|e| to_diag(fn_spec, format!("byte length must be one byte: {e}")))?;

        if data.len() != expected_bytes_length as usize {
            return Err(to_diag(
                fn_spec,
                format!(
                    "provided length ({}) does not equal data byte length ({})",
                    expected_bytes_length,
                    data.len()
                ),
            ));
        }

        let mut bytes: Vec<u8> = BitcoinOpcode::OpPushData1.get_code();
        bytes.push(expected_bytes_length);

        bytes.append(&mut data);

        Ok(BitcoinValue::opcode(bytes))
    }
}
