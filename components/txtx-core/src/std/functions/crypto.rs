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
            documentation: "Coming soon",
            example: indoc!{r#"
          "#},
            inputs: [
                value: {
                    documentation: "Coming soon",
                    typing: vec![Type::buffer(), Type::array(Type::buffer())]
                }
            ],
            output: {
                documentation: "",
                typing: Type::string()
            },
        }
    }];
}

pub struct Secp256k1Recover;
impl FunctionImplementation for Secp256k1Recover {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        use libsecp256k1::{recover, Message, RecoveryId, Signature};

        let (Some(message), Some(signature)) = (args.get(0), args.get(1)) else {
            return Err(diagnosed_error!("{}: expected 2 arguments", ctx.name));
        };

        let signature_bytes = signature.to_bytes();
        let message = Message::parse_slice(&message.to_bytes()).unwrap();
        let recovery_id = RecoveryId::parse(signature_bytes[0]).unwrap();
        let signature = Signature::parse_standard_slice(&signature_bytes[1..]).unwrap();
        let public_key = recover(&message, &signature, &recovery_id).unwrap();
        let public_key_hex = txtx_addon_kit::hex::encode(public_key.serialize_compressed());
        println!("==> {}", public_key_hex);

        Ok(Value::string(format!("0x{}", public_key_hex)))
    }
}
