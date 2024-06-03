use kit::types::types::{PrimitiveValue, TypeImplementation, TypeSpecification};
use ripemd::{Digest, Ripemd160 as LibRipemd160};
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
        Ripemd160 => {
            name: "ripemd160",
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
    },];
    pub static ref HASH_BUFFER: TypeSpecification = define_addon_type! {
        HashBuffer => {
            name: "hash_buffer",
            documentation: "Hash Buffer",
        }
    };
}

pub struct Ripemd160;
impl FunctionImplementation for Ripemd160 {
    fn check_instantiability(
        _ctx: &FunctionSpecification,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(_ctx: &FunctionSpecification, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        if let Some(Value::Primitive(PrimitiveValue::Buffer(buf))) = args.get(0) {
            let mut hasher = LibRipemd160::new();
            hasher.update(&buf.bytes[..]);
            let result = hasher.finalize();
            let value = Value::buffer(result[..].to_vec(), HASH_BUFFER.clone());
            return Ok(value);
        }
        if let Some(Value::Array(buffers)) = args.get(0) {
            let mut joined = vec![];
            for maybe_buffer in buffers.iter() {
                let Value::Primitive(PrimitiveValue::Buffer(buf)) = maybe_buffer else {
                    return Err(Diagnostic::error_from_string("wrong inputs".to_string()));
                };
                joined.extend(buf.bytes.clone());
            }
            let mut hasher = LibRipemd160::new();
            hasher.update(&joined[..]);
            let result = hasher.finalize();
            let value = Value::buffer(result[..].to_vec(), HASH_BUFFER.clone());
            return Ok(value);
        }
        Err(Diagnostic::error_from_string("wrong inputs".to_string()))
    }
}

pub struct HashBuffer;
impl TypeImplementation for HashBuffer {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}
