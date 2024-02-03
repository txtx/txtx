pub mod errors;
pub mod types;
pub mod visitor;

pub use txtx_ext_kit as kit;

use std::collections::HashMap;

use txtx_ext_kit::Codec;
use types::Manual;
use visitor::run_node_indexer;
use visitor::run_node_processor;

pub fn simulate_manual(
    manual: &mut Manual,
    codec_manager: &mut CodecManager,
) -> Result<(), String> {
    manual
        .errors
        .iter()
        .enumerate()
        .for_each(|(i, e)| println!("Error {}: {:?}", i + 1, e));
    Ok(())
}

pub struct CodecManager {
    registered_constructs: HashMap<String, usize>,
    registered_codecs: HashMap<usize, Box<dyn Codec>>,
}

impl CodecManager {
    pub fn new() -> Self {
        Self {
            registered_constructs: HashMap::new(),
            registered_codecs: HashMap::new(),
        }
    }

    pub fn register(&mut self, codec: Box<dyn Codec>) {
        let codec_id = self.registered_codecs.len();
        // Register decoders
        for decoder in codec.get_supported_decoders().into_iter() {
            self.registered_constructs.insert(decoder, codec_id);
        }
        // Register encoders
        for encoder in codec.get_supported_encoders().into_iter() {
            self.registered_constructs.insert(encoder, codec_id);
        }
        self.registered_constructs
            .insert(codec.get_supported_network(), codec_id);
        self.registered_codecs.insert(codec_id, codec);
    }
}
