use txtx_ext_kit::Codec;

pub struct BitcoinCodec {}

impl BitcoinCodec {
    pub fn new() -> Self {
        Self {}
    }
}

impl Codec for BitcoinCodec {
    fn get_supported_network(&self) -> String {
        "bitcoin".to_string()
    }

    fn get_supported_decoders(&self) -> Vec<String> {
        vec![
            "bitcoin_script".to_string(),
            "bitcoin_descriptor".to_string(),
        ]
    }

    fn get_supported_encoders(&self) -> Vec<String> {
        vec![]
    }
}
