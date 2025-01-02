use txtx_addon_kit::types::types::Value;

pub const SP1_ELF: &str = "sp1::elf";
pub const SP1_PUBLIC_VALUES: &str = "sp1::public_values";
pub const SP1_PROOF: &str = "sp1::proof";
pub const SP1_VERIFICATION_KEY: &str = "sp1::verification_key";

pub struct Sp1Value {}

impl Sp1Value {
    pub fn elf(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SP1_ELF)
    }

    pub fn public_values(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SP1_PUBLIC_VALUES)
    }

    pub fn proof(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SP1_PROOF)
    }

    pub fn verification_key(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SP1_VERIFICATION_KEY)
    }
}
