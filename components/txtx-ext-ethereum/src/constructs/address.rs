use std::str::FromStr;

use alloy_primitives::Address as EthereumAddress;
use txtx_ext_kit::{
    hcl::{expr::Expression, structure::Block},
    helpers::{
        fs::FileLocation,
        hcl::{collect_dependencies_from_expression, visit_label, VisitorError},
    },
    ExtensionConstruct,
};

pub struct Address {
    pub name: String,
    pub value: Expression,
}

impl ExtensionConstruct for Address {
    fn get_name(self: &Self) -> &str {
        &self.name
    }

    fn from_block(block: &Block, _location: &FileLocation) -> Result<Box<Self>, VisitorError>
    where
        Self: Sized,
    {
        let name = visit_label(1, "name", &block)?;
        let value = Address::visit("value", &block)?;
        Ok(Box::new(Address { name, value }))
    }

    fn collect_dependencies(self: &Self) -> Vec<Expression> {
        let mut dependencies = vec![];

        collect_dependencies_from_expression(&self.value, &mut dependencies);
        dependencies
    }
}

impl std::fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "address {}: {}", self.name, "value")
    }
}

impl Address {
    pub fn visit(field_name: &str, block: &Block) -> Result<Expression, VisitorError> {
        let Some(attribute) = block.body.get_attribute(field_name) else {
            return Err(VisitorError::MissingAttribute(field_name.into()));
        };
        match attribute.value.clone() {
            Expression::String(value) => {
                // validate that it is a valid address
                let _ = EthereumAddress::from_str(&value).map_err(|e| {
                    VisitorError::TypeExpected(format!(
                        "could not convert to value to Ethereum Address: {e}"
                    ))
                })?;

                Ok(Expression::String(value))
            }
            _ => Err(VisitorError::TypeExpected("string".into())),
        }
    }
}
