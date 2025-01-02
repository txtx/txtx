use txtx_addon_kit::types::types::Value;

pub const BITCOIN_OPCODE: &str = "btc::opcode";
pub const BITCOIN_SCRIPT: &str = "btc::script";

pub struct BitcoinValue {}

impl BitcoinValue {
    pub fn opcode(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, BITCOIN_OPCODE)
    }
    pub fn script(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, BITCOIN_SCRIPT)
    }
}
