use std::collections::HashMap;

use txtx_addon_kit::hcl::{expr::Expression, structure::Block};
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::helpers::hcl::{
    collect_dependencies_from_expression, visit_label, visit_optional_untyped_attribute,
    visit_required_string_literal_attribute, VisitorError,
};
use txtx_addon_kit::types::diagnostics::Diagnostic;

use crate::types::PackageUuid;

#[derive(Clone, Debug)]
pub struct ImportConstruct {
    pub name: String,
    pub description: Option<Expression>,
    pub path: String,
    pub diagnostics: Vec<Diagnostic>,
    pub package_uuid: PackageUuid,
}

impl ImportConstruct {
    pub fn from_block(
        block: &Block,
        location: &FileLocation,
        packages_uuids: &HashMap<FileLocation, PackageUuid>,
    ) -> Result<ImportConstruct, VisitorError> {
        // Retrieve name
        let name = visit_label(0, "name", &block)?;

        // Retrieve description
        let description = visit_optional_untyped_attribute("description", &block)?;

        // Retrieve path
        let path = visit_required_string_literal_attribute("path", &block)?;

        let diagnostics = vec![];

        let mut parent_location = location.get_parent_location().unwrap(); // todo(lgalabru)
        parent_location.append_path(&path).unwrap();
        let package_uuid = packages_uuids.get(&parent_location).unwrap(); // todo(lgalabru)

        Ok(ImportConstruct {
            name,
            description,
            path,
            diagnostics,
            package_uuid: package_uuid.clone(),
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
