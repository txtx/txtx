use super::{
    ConstructData, ConstructUuid, ModuleConstruct, Package, PackageUuid, PreConstruct,
    PreConstructData,
};
use crate::errors::ConstructErrors;
use daggy::Dag;
use std::collections::VecDeque;
use std::{collections::HashMap, ops::Range};
use txtx_ext_kit::hcl::expr::{Expression, Traversal, TraversalOperator};
use txtx_ext_kit::hcl::visit::visit_traversal_operator;
use txtx_ext_kit::helpers::fs::FileLocation;

#[derive(Debug)]
pub struct SourceTree {
    pub files: HashMap<FileLocation, (String, String)>,
}

impl SourceTree {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn add_source(&mut self, name: String, location: FileLocation, content: String) {
        self.files.insert(location, (name, content));
    }
}

#[derive(Debug)]
pub struct Manual {
    pub source_tree: Option<SourceTree>,
    pub packages_uuid_lookup: HashMap<FileLocation, PackageUuid>,
    pub manual_metadata_construct_uuid: Option<ConstructUuid>,
    pub packages: HashMap<PackageUuid, Package>,
    pub packages_graph: Dag<PackageUuid, u32, u32>,
    pub pre_constructs: HashMap<ConstructUuid, PreConstruct>,
    pub constructs: HashMap<ConstructUuid, ConstructData>,
    pub constructs_locations: HashMap<ConstructUuid, (PackageUuid, FileLocation)>,
    pub errors: Vec<ConstructErrors>,
}

impl Manual {
    pub fn new(source_tree: Option<SourceTree>) -> Self {
        Self {
            source_tree,
            packages: HashMap::new(),
            packages_uuid_lookup: HashMap::new(),
            packages_graph: Dag::new(),
            manual_metadata_construct_uuid: None,
            errors: vec![],
            pre_constructs: HashMap::new(),
            constructs_locations: HashMap::new(),
            constructs: HashMap::new(),
        }
    }

    pub fn get_metadata_module(&self) -> Option<&ModuleConstruct> {
        self.manual_metadata_construct_uuid
            .as_ref()
            .and_then(|c| self.constructs.get(&c))
            .and_then(|c| c.as_module())
    }

    pub fn inspect_constructs(&self) {
        for (package_uuid, package) in self.packages.iter() {
            println!("{} ({})", package.name, package.location.to_string());
            if !package.imports_uuids.is_empty() {
                println!("Imports:");
            }
            for construct_uuid in package.imports_uuids.iter() {
                let construct = self.constructs.get(construct_uuid).unwrap();
                println!("- {}", construct.as_import().unwrap().name);
                for dep in construct.collect_dependencies().iter() {
                    println!("  -> {}", dep);
                }
            }

            if !package.variables_uuids.is_empty() {
                println!("Variables:");
            }
            for construct_uuid in package.variables_uuids.iter() {
                let construct = self.constructs.get(construct_uuid).unwrap();
                println!("- {}", construct.as_variable().unwrap().name);
                for dep in construct.collect_dependencies().iter() {
                    let result = self.resolve_construct_reference(package_uuid, dep);
                    if let Ok(Some(resolved_construct_uuid)) = result {
                        println!(
                            "  -> {} resolving to {}",
                            dep,
                            resolved_construct_uuid.value()
                        );
                    } else {
                        println!("  -> {} (unable to resolve)", dep,);
                    }
                }
            }
            if !package.modules_uuids.is_empty() {
                println!("Modules:");
            }
            for construct_uuid in package.modules_uuids.iter() {
                let construct = self.constructs.get(construct_uuid).unwrap();
                println!("- {}", construct.as_module().unwrap().id);
                for dep in construct.collect_dependencies().iter() {
                    let result = self.resolve_construct_reference(package_uuid, dep);
                    if let Ok(Some(resolved_construct_uuid)) = result {
                        println!(
                            "  -> {} resolving to {}",
                            dep,
                            resolved_construct_uuid.value()
                        );
                    } else {
                        println!("  -> {} (unable to resolve)", dep,);
                    }
                }
            }

            if !package.variables_uuids.is_empty() {
                println!("Outputs:");
            }
            for construct_uuid in package.outputs_uuids.iter() {
                let construct = self.constructs.get(construct_uuid).unwrap();
                println!("- {}", construct.as_output().unwrap().name);
                for dep in construct.collect_dependencies().iter() {
                    let result = self.resolve_construct_reference(package_uuid, dep);
                    if let Ok(Some(resolved_construct_uuid)) = result {
                        println!(
                            "  -> {} resolving to {}",
                            dep,
                            resolved_construct_uuid.value()
                        );
                    } else {
                        println!("  -> {} (unable to resolve)", dep,);
                    }
                }
            }
            println!("");
        }
    }

    pub fn index_construct(
        &mut self,
        construct_name: String,
        construct_location: FileLocation,
        construct_data: PreConstructData,
        construct_span: Range<usize>,
        package_name: &String,
        package_location: &FileLocation,
    ) {
        // Retrieve existing module_uuid, create otherwise
        let package_uuid = loop {
            match self.packages_uuid_lookup.get(&package_location) {
                Some(uuid) => break uuid,
                None => {
                    let package = Package::new(&package_name, &package_location);
                    self.packages_uuid_lookup
                        .insert(package_location.clone(), package.uuid.clone());
                    self.packages.insert(package.uuid.clone(), package);
                    continue;
                }
            }
        };

        let Some(package) = self.packages.get_mut(package_uuid) else {
            unreachable!()
        };

        let construct_uuid = ConstructUuid::new();
        // Update module
        match construct_data {
            PreConstructData::Module(_) => {
                if construct_name.eq("manual") && self.manual_metadata_construct_uuid.is_none() {
                    self.manual_metadata_construct_uuid = Some(construct_uuid.clone());
                }
                package.modules_uuids.insert(construct_uuid.clone());
                package
                    .modules_uuids_lookup
                    .insert(construct_name.clone(), construct_uuid.clone());
            }
            PreConstructData::Variable(_) => {
                package.variables_uuids.insert(construct_uuid.clone());
                package
                    .variables_uuids_lookup
                    .insert(construct_name.clone(), construct_uuid.clone());
            }
            PreConstructData::Output(_) => {
                package.outputs_uuids.insert(construct_uuid.clone());
                package
                    .outputs_uuids_lookup
                    .insert(construct_name.clone(), construct_uuid.clone());
            }
            PreConstructData::Import(_) => {
                package.imports_uuids.insert(construct_uuid.clone());
                package
                    .imports_uuids_lookup
                    .insert(construct_name.clone(), construct_uuid.clone());
            }
            PreConstructData::Ext(_) => {
                package.exts_uuids.insert(construct_uuid.clone());
                package
                    .exts_uuids_lookup
                    .insert(construct_name.clone(), construct_uuid.clone());
            }
            PreConstructData::Root => unreachable!(),
        }
        package.constructs_graph.add_child(
            package.constructs_graph_root.clone(),
            0,
            construct_uuid.value(),
        );

        // Update plan
        let pre_construct = PreConstruct {
            uuid: construct_uuid.clone(),
            name: construct_name,
            data: construct_data,
            span: construct_span,
        };
        self.pre_constructs
            .insert(construct_uuid.clone(), pre_construct);
        self.constructs_locations.insert(
            construct_uuid.clone(),
            (package_uuid.clone(), construct_location),
        );
    }

    pub fn add_construct(&mut self, uuid: &ConstructUuid, construct: ConstructData) {
        self.constructs.insert(uuid.clone(), construct);
    }

    pub fn resolve_construct_reference(
        &self,
        package_uuid_source: &PackageUuid,
        expression: &Expression,
    ) -> Result<Option<ConstructUuid>, String> {
        let Some(traversal) = expression.as_traversal() else {
            return Ok(None);
        };

        let Some(mut current_package) = self.packages.get(package_uuid_source) else {
            return Ok(None);
        };

        let Some(root) = traversal.expr.as_variable() else {
            return Ok(None);
        };
        let mut components = VecDeque::new();
        components.push_front(root.to_string());

        for op in traversal.operators.iter() {
            if let TraversalOperator::GetAttr(value) = op.value() {
                components.push_back(value.to_string());
            }
        }

        while let Some(component) = components.pop_front() {

            println!("{component}");

            // Look for modules
            if component.eq_ignore_ascii_case("module") {
                let Some(module_name) = components.pop_front() else {
                    continue;
                };
                if let Some(construct_uuid) =
                current_package.modules_uuids_lookup.get(&module_name)
                {
                    return Ok(Some(construct_uuid.clone()));
                }
            }

            // Look for outputs
            if component.eq_ignore_ascii_case("output") {
                let Some(output_name) = components.pop_front() else {
                    continue;
                };
                if let Some(construct_uuid) =
                current_package.outputs_uuids_lookup.get(&output_name)
                {
                    return Ok(Some(construct_uuid.clone()));
                }
            }

            // Look for variables
            if component.eq_ignore_ascii_case("variable") {
                let Some(variable_name) = components.pop_front() else {
                    continue;
                };
                if let Some(construct_uuid) =
                current_package.variables_uuids_lookup.get(&variable_name)
                {
                    return Ok(Some(construct_uuid.clone()));
                }
            }

            let imported_package = current_package
                .imports_uuids_lookup
                .get(&component.to_string())
                .and_then(|c| self.constructs.get(c))
                .and_then(|c| c.as_import())
                .and_then(|i| Some(&i.package_uuid))
                .and_then(|p| self.packages.get(&p));

            if let Some(imported_package) = imported_package {
                current_package = imported_package;
                continue
            }
        }

        Ok(None)
    }
}
