#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

use txtx_addon_kit::{types::commands::PreCommandSpecification, Addon};


#[derive(Debug)]
pub struct Sp1Addon;

impl Sp1Addon {
    pub fn new() -> Self {
        Self {}
    }
}

impl Addon for Sp1Addon {
    fn get_name(&self) -> &str {
        "SP1 (alpha)"
    }

    fn get_description(&self) -> &str {
        txtx_addon_kit::indoc! {r#"
            Lorem ipsum 
            "#}
    }

    fn get_namespace(&self) -> &str {
        "sp1"
    }

    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        vec![]
    }
}
