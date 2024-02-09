// use txtx_addon_kit::{
//     hcl::structure::Block,
//     helpers::{fs::FileLocation, hcl::VisitorError},
//     Addon, AddonConstruct,
// };

// #[derive(Debug)]
// pub struct BitcoinNetworkAddon {}

// impl BitcoinNetworkAddon {
//     pub fn new() -> Self {
//         Self {}
//     }
// }

// impl Addon for BitcoinNetworkAddon {
//     fn get_name(self: &Self) -> String {
//         unimplemented!()
//     }
//     fn get_construct_from_block_and_name(
//         self: &Self,
//         _name: &String,
//         _block: &Block,
//         _location: &FileLocation,
//     ) -> Result<Option<Box<dyn AddonConstruct>>, VisitorError> {
//         unimplemented!()
//     }
//     fn supports_construct(self: &Self, _name: &String) -> bool {
//         unimplemented!()
//     }
//     fn index_node(self: &Self) {
//         unimplemented!()
//     }
// }
