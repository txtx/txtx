use txtx_addon_kit::types::types::Value;

pub const SP1_ELF: &str = "sp1::elf";

pub struct Sp1Value {}

impl Sp1Value {
    pub fn elf(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SP1_ELF)
    }
}
