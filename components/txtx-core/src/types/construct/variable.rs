use txtx_addon_kit::hcl::{expr::Expression, structure::Block};
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::helpers::hcl::{
    build_diagnostics_for_unused_fields, collect_constructs_references_from_expression,
    visit_label, visit_optional_untyped_attribute, VisitorError,
};
use txtx_addon_kit::types::diagnostics::Diagnostic;

#[derive(Clone, Debug)]
pub struct VariableConstruct {
    pub name: String,
    pub description: Option<Expression>,
    pub value: Option<Expression>,
    pub default: Option<Expression>,
    pub diagnostics: Vec<Diagnostic>,
}

impl VariableConstruct {
    pub fn from_block(
        block: &Block,
        location: &FileLocation,
    ) -> Result<VariableConstruct, VisitorError> {
        // Retrieve name
        let name = visit_label(0, "name", &block)?;

        // Retrieve description
        let description = visit_optional_untyped_attribute("description", &block)?;

        // Retrieve value
        let value = visit_optional_untyped_attribute("value", &block)?;

        // Retrieve value
        let default = visit_optional_untyped_attribute("default", &block)?;

        // Diagnose any unused additional field as such
        let diagnostics = build_diagnostics_for_unused_fields(
            vec!["description", "value", "default"],
            &block,
            location,
        );

        Ok(VariableConstruct {
            name,
            description,
            value,
            default,
            diagnostics,
        })
    }

    pub fn collect_dependencies(&self) -> Vec<Expression> {
        let mut dependencies = vec![];

        if let Some(ref expr) = self.description {
            collect_constructs_references_from_expression(expr, &mut dependencies)
        }

        if let Some(ref expr) = self.default {
            collect_constructs_references_from_expression(expr, &mut dependencies);
        }

        if let Some(ref expr) = self.value {
            collect_constructs_references_from_expression(expr, &mut dependencies);
        }
        dependencies
    }

    pub fn eval_inputs(&self) {
        let mut dependencies = vec![];

        if let Some(ref expr) = self.description {
            collect_constructs_references_from_expression(expr, &mut dependencies)
        }

        if let Some(ref expr) = self.default {
            collect_constructs_references_from_expression(expr, &mut dependencies);
        }

        if let Some(ref expr) = self.value {
            collect_constructs_references_from_expression(expr, &mut dependencies);
        }
    }
}
