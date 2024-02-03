use txtx_ext_kit::{
    hcl::{expr::Expression, structure::Block},
    helpers::{
        fs::FileLocation,
        hcl::{
            collect_dependencies_from_expression, visit_label, visit_optional_untyped_attribute,
            VisitorError,
        },
    },
    ExtensionConstruct,
};

pub struct Uint256 {
    pub name: String,
    pub value: Option<Expression>,
}

impl ExtensionConstruct for Uint256 {
    fn get_name(self: &Self) -> &str {
        &self.name
    }

    fn from_block(block: &Block, _location: &FileLocation) -> Result<Box<Self>, VisitorError>
    where
        Self: Sized,
    {
        let name = visit_label(1, "name", &block)?;
        let value = visit_optional_untyped_attribute("value", &block)?;
        Ok(Box::new(Uint256 { name, value }))
    }

    fn collect_dependencies(self: &Self) -> Vec<Expression> {
        let mut dependencies = vec![];

        if let Some(ref expr) = self.value {
            collect_dependencies_from_expression(expr, &mut dependencies)
        }
        dependencies
    }
}

impl std::fmt::Debug for Uint256 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "uint256 {}: {}", self.name, "value")
    }
}
