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
        Secp256k1Recover => {
            name: "secp256k1_recover",
            documentation: "`secp256k1_recover` recovers a public key from a secp256k1 signature.",
            example: indoc!{r#"
            output "recovered_public_key" {
                value = secp256k1_recover("0x6a2ce4b8aab1ef79aa1aa617cf6b72d7146857b83055e203b67c5177faef212c", "0x0165a85a1e64d7157d678d177bc8a9e6bfb8d750458d52a31c34abe1e56475b5eb62f183a5e6ddbced38fca93a8ff1c73b4ce66231e39392572af916b5303fbe12")
            }
            // > recovered_public_key: 0x03b3e0a76b292b2c83fc0ac14ae6160d0438ebe94e14bbb5b7755153628886e08e
          "#},
            inputs: [
                message: {
                    documentation: "The hash of the original message that was signed.",
                    typing: vec![Type::buffer(), Type::array(Type::buffer())]
                },
                signature: {
                    documentation: "The signature that was produced using the secp256k1 elliptic curve algorithm.",
                    typing: vec![Type::buffer(), Type::array(Type::buffer())]
                }
            ],
            output: {
                documentation: "The recovered public key.",
                typing: Type::string()
            },
        }
    }];
}

pub struct Secp256k1Recover;
impl FunctionImplementation for Secp256k1Recover {
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
        use libsecp256k1::{recover, Message, RecoveryId, Signature};

        let (Some(message), Some(signature)) = (args.get(0), args.get(1)) else {
            return Err(diagnosed_error!("{}: expected 2 arguments", fn_spec.name));
        };

        let signature_bytes = signature.to_be_bytes();
        let message = Message::parse_slice(&message.to_be_bytes()).unwrap();
        let recovery_id = RecoveryId::parse(signature_bytes[0]).unwrap();
        let signature = Signature::parse_standard_slice(&signature_bytes[1..]).unwrap();
        let public_key = recover(&message, &signature, &recovery_id).unwrap();
        let public_key_hex = txtx_addon_kit::hex::encode(public_key.serialize_compressed());

        Ok(Value::string(format!("0x{}", public_key_hex)))
    }
}
