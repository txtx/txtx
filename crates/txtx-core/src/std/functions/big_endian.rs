use super::{arg_checker, to_diag};
use txtx_addon_kit::{
    define_function, indoc,
    types::{
        diagnostics::Diagnostic,
        functions::{FunctionImplementation, FunctionSpecification},
        types::{Type, Value},
    },
};
use txtx_addon_kit::{hex, types::AuthorizationContext};

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            BigEndianEncodeU32 => {
                name: "encode_big_endian_u32",
                documentation: "`encode_big_endian_u32` encodes a u32 to a big-endian hex string",
                example: indoc!{r#"
                    output "encoded" {
                        value = encode_big_endian_u32(1)
                    }
                    > encoded: 0x00000001
                "#},
                inputs: [
                    integer_u32: {
                        documentation: "The integer to encode",
                        typing: vec![Type::integer()]
                    }
                ],
                output: {
                    documentation: "The integer encoded as a hex string",
                    typing: Type::string()
                },
            }
        },
        define_function! {
            BigEndianEncodeU64 => {
                name: "encode_big_endian_u64",
                documentation: "`encode_big_endian_u64` encodes a u64 to a big-endian hex string",
                example: indoc!{r#"
                    output "encoded" {
                        value = encode_big_endian_u64(1)
                    }
                    > encoded: 0x0000000000000001
                "#},
                inputs: [
                    integer_u32: {
                        documentation: "The integer to encode",
                        typing: vec![Type::integer()]
                    }
                ],
                output: {
                    documentation: "The integer encoded as a hex string",
                    typing: Type::string()
                },
            }
        }
    ];
}

pub struct BigEndianEncodeU32;
impl FunctionImplementation for BigEndianEncodeU32 {
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

        let value = parse_integer::<u32>(fn_spec, args)?;
        let bytes = value.to_be_bytes();

        Ok(Value::string(format!("0x{}", hex::encode(bytes))))
    }
}

pub struct BigEndianEncodeU64;
impl FunctionImplementation for BigEndianEncodeU64 {
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

        let value = parse_integer::<u64>(fn_spec, args)?;
        let bytes = value.to_be_bytes();

        Ok(Value::string(format!("0x{}", hex::encode(bytes))))
    }
}

fn parse_integer<T>(fn_spec: &FunctionSpecification, args: &Vec<Value>) -> Result<T, Diagnostic>
where
    T: TryFrom<i128>,
{
    let Value::Integer(i) = args.get(0).unwrap() else {
        return Err(to_diag(fn_spec, "expected argument 0 to be an integer".to_string()));
    };

    let value: T =
        (*i).try_into().map_err(|_| to_diag(fn_spec, format!("integer {} is out of range", i)))?;

    Ok(value)
}
