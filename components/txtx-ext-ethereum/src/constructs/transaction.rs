use txtx_ext_kit::{
    hcl::{expr::Expression, structure::Block},
    helpers::{
        fs::FileLocation,
        hcl::{collect_dependencies_from_expression, visit_label, VisitorError},
    },
    ExtensionConstruct,
};

use super::address::Address;

pub struct Transaction {
    pub name: String,
    pub from: Expression,
    pub to: Expression,
}

impl ExtensionConstruct for Transaction {
    fn get_name(self: &Self) -> &str {
        &self.name
    }

    fn from_block(block: &Block, _location: &FileLocation) -> Result<Box<Self>, VisitorError>
    where
        Self: Sized,
    {
        let name = visit_label(1, "name", &block)?;
        let from = visit_address_or_traversal_attribute("from", &block)?;
        let to = visit_address_or_traversal_attribute("to", &block)?;
        Ok(Box::new(Transaction { name, from, to }))
    }

    fn collect_dependencies(self: &Self) -> Vec<Expression> {
        let mut dependencies = vec![];

        collect_dependencies_from_expression(&self.from, &mut dependencies);
        collect_dependencies_from_expression(&self.to, &mut dependencies);
        dependencies
    }
}

impl std::fmt::Debug for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", "tx-id")
    }
}
fn visit_address_or_traversal_attribute(
    field_name: &str,
    block: &Block,
) -> Result<Expression, VisitorError> {
    let Some(attribute) = block.body.get_attribute(field_name) else {
        return Err(VisitorError::MissingAttribute(field_name.into()));
    };
    match attribute.value.clone() {
        Expression::Traversal(value) => Ok(Expression::Traversal(value)),
        Expression::String(_) => Address::visit(field_name, block),
        _ => Err(VisitorError::TypeExpected("string or traversal".into())),
    }
}

impl Transaction {
    fn _to_ethereum_transaction(&self) {}
}
