use std::collections::BTreeMap;
use txtx_addon_kit::hcl::{expr::Expression, structure::Block};
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::helpers::hcl::{
    collect_dependencies_from_expression, visit_label, visit_optional_untyped_attribute,
    VisitorError,
};
use txtx_addon_kit::types::diagnostics::Diagnostic;

#[derive(Clone, Debug)]
pub struct ModuleConstruct {
    pub id: String,
    pub name: Option<Expression>,
    pub description: Option<Expression>,
    pub fields: BTreeMap<String, Expression>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ModuleConstruct {
    pub fn from_block(
        block: &Block,
        _location: &FileLocation,
    ) -> Result<ModuleConstruct, VisitorError> {
        // Retrieve id
        let id = visit_label(0, "name", &block)?;

        // Retrieve name
        let name = visit_optional_untyped_attribute("name", &block)?;

        // Retrieve description
        let description = visit_optional_untyped_attribute("description", &block)?;

        // Retrieve fields
        let fields = BTreeMap::new();

        let diagnostics = vec![];

        Ok(ModuleConstruct {
            id,
            name,
            description,
            fields,
            diagnostics,
        })
    }

    pub fn collect_dependencies(&self) -> Vec<Expression> {
        let mut dependencies = vec![];

        if let Some(ref expr) = self.description {
            collect_dependencies_from_expression(expr, &mut dependencies)
        }
        dependencies
    }
}
