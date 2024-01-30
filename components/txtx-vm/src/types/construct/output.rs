use txtx_ext_kit::hcl::{expr::Expression, structure::Block};
use txtx_ext_kit::helpers::fs::FileLocation;
use txtx_ext_kit::helpers::hcl::{
    build_diagnostics_for_unused_fields, collect_dependencies_from_expression, visit_label,
    visit_optional_untyped_attribute, VisitorError,
};
use txtx_ext_kit::types::diagnostics::Diagnostic;

#[derive(Clone, Debug)]
pub struct OutputConstruct {
    pub name: String,
    pub description: Option<Expression>,
    pub value: Option<Expression>,
    pub diagnostics: Vec<Diagnostic>,
}

impl OutputConstruct {
    pub fn from_block(
        block: &Block,
        location: &FileLocation,
    ) -> Result<OutputConstruct, VisitorError> {
        // Retrieve name
        let name = visit_label(0, "name", &block)?;

        // Retrieve description
        let description = visit_optional_untyped_attribute("description", &block)?;

        // Retrieve value
        let value = visit_optional_untyped_attribute("value", &block)?;

        // Diagnose any unused additional field as such
        let diagnostics =
            build_diagnostics_for_unused_fields(vec!["description", "value"], &block, location);

        Ok(OutputConstruct {
            name,
            description,
            value,
            diagnostics,
        })
    }

    pub fn collect_dependencies(&self) -> Vec<Expression> {
        let mut dependencies = vec![];

        if let Some(ref expr) = self.description {
            collect_dependencies_from_expression(expr, &mut dependencies)
        }

        if let Some(ref expr) = self.value {
            collect_dependencies_from_expression(expr, &mut dependencies);
        }
        dependencies
    }
}
