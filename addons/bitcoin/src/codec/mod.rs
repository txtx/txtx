pub enum BitcoinOpcode {
    // Constants
    Op0,
    OpPushData,
    OpPushData1,
    // Stack Opcodes
    OpDup,
    // Bitwise Logic Opcodes
    OpEqualVerify,
    // Crypto Opcodes
    OpHash160,
    OpCheckSig,
    // Control Flow Opcodes
}

impl BitcoinOpcode {
    pub fn get_code(&self) -> Vec<u8> {
        match self {
            BitcoinOpcode::Op0 => vec![0],
            BitcoinOpcode::OpPushData => vec![],
            BitcoinOpcode::OpPushData1 => vec![196],
            BitcoinOpcode::OpDup => vec![118],
            BitcoinOpcode::OpEqualVerify => vec![136],
            BitcoinOpcode::OpHash160 => vec![169],
            BitcoinOpcode::OpCheckSig => vec![172],
        }
    }
}
