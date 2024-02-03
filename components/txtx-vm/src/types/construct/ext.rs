use txtx_ext_kit::hcl::structure::Block;

#[derive(Debug)]
pub struct ExtPreConstructData {
    pub block: Block,
    pub extension_name: String,
    pub construct_name: String,
}
