use txtx_ext_kit::hcl::{expr::Expression, structure::Block};
use txtx_ext_kit::helpers::fs::FileLocation;
use txtx_ext_kit::helpers::hcl::{
    collect_dependencies_from_expression, visit_label, visit_optional_untyped_attribute,
    VisitorError,
};
use txtx_ext_kit::types::diagnostics::Diagnostic;

#[derive(Clone, Debug)]
pub struct ImportConstruct {
    pub name: String,
    pub description: Option<Expression>,
    pub path: Option<Expression>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ImportConstruct {
    pub fn from_block(
        block: &Block,
        _location: &FileLocation,
    ) -> Result<ImportConstruct, VisitorError> {
        // Retrieve name
        let name = visit_label(0, "name", &block)?;

        // Retrieve description
        let description = visit_optional_untyped_attribute("description", &block)?;

        // Retrieve path
        let path = visit_optional_untyped_attribute("path", &block)?;

        let diagnostics = vec![];

        Ok(ImportConstruct {
            name,
            description,
            path,
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
