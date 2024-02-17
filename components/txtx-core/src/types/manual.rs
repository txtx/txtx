use super::{Package, PreConstructData};
use crate::errors::ConstructErrors;
use crate::std::commands;
use daggy::{Dag, NodeIndex};
use std::collections::HashMap;
use std::collections::VecDeque;
use txtx_addon_kit::hcl::expr::{Expression, TraversalOperator};
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::types::commands::CommandInstance;
use txtx_addon_kit::types::{ConstructUuid, PackageUuid};
use txtx_addon_kit::uuid::Uuid;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct Manual {
    pub source_tree: Option<SourceTree>,
    pub packages_uuid_lookup: HashMap<FileLocation, PackageUuid>,
    pub manual_metadata_construct_uuid: Option<ConstructUuid>,
    pub packages: HashMap<PackageUuid, Package>,
    pub graph_root: NodeIndex<u32>,
    pub packages_graph: Dag<Uuid, u32, u32>,
    pub constructs_graph: Dag<Uuid, u32, u32>,
    pub constructs_graph_nodes: HashMap<Uuid, NodeIndex<u32>>,
    pub packages_graph_nodes: HashMap<Uuid, NodeIndex<u32>>,
    pub commands_instances: HashMap<ConstructUuid, CommandInstance>,
    pub constructs_locations: HashMap<ConstructUuid, (PackageUuid, FileLocation)>,
    pub errors: Vec<ConstructErrors>,
}

impl Manual {
    pub fn new(source_tree: Option<SourceTree>) -> Self {
        let uuid = PackageUuid::new();
        let mut packages_graph = Dag::new();
        let _ = packages_graph.add_node(uuid.value());
        let mut constructs_graph = Dag::new();
        let graph_root = constructs_graph.add_node(uuid.value());

        Self {
            source_tree,
            packages: HashMap::new(),
            packages_uuid_lookup: HashMap::new(),
            packages_graph,
            constructs_graph,
            constructs_graph_nodes: HashMap::new(),
            packages_graph_nodes: HashMap::new(),
            graph_root,
            manual_metadata_construct_uuid: None,
            errors: vec![],
            constructs_locations: HashMap::new(),
            commands_instances: HashMap::new(),
        }
    }

    pub fn get_metadata_module(&self) -> Option<&CommandInstance> {
        None
    }

    pub fn index_construct(
        &mut self,
        construct_name: String,
        construct_location: FileLocation,
        construct_data: PreConstructData,
        package_name: &String,
        package_location: &FileLocation,
    ) -> Result<PackageUuid, String> {
        // Retrieve existing module_uuid, create otherwise
        let package_uuid = loop {
            if let Some(uuid) = self.packages_uuid_lookup.get(&package_location) {
                break uuid.clone();
            }
            let package = Package::new(&package_name, &package_location);
            self.packages_uuid_lookup
                .insert(package_location.clone(), package.uuid.clone());
            let package_uuid = package.uuid.clone();
            self.packages.insert(package_uuid.clone(), package);
            self.packages_graph
                .add_child(self.graph_root, 0, package_uuid.value());
            continue;
        };

        let Some(package) = self.packages.get_mut(&package_uuid) else {
            unreachable!()
        };

        let construct_uuid = ConstructUuid::new();
        // Update module
        match &construct_data {
            PreConstructData::Module(block) => {
                if construct_name.eq("manual") && self.manual_metadata_construct_uuid.is_none() {
                    self.manual_metadata_construct_uuid = Some(construct_uuid.clone());
                }
                package.modules_uuids.insert(construct_uuid.clone());
                package
                    .modules_uuids_lookup
                    .insert(construct_name.clone(), construct_uuid.clone());
                self.commands_instances.insert(
                    construct_uuid.clone(),
                    CommandInstance {
                        specification: commands::new_module_specification(),
                        name: construct_name.clone(),
                        block: block.clone(),
                        package_uuid: package_uuid.clone(),
                    },
                );
            }
            PreConstructData::Variable(block) => {
                package.variables_uuids.insert(construct_uuid.clone());
                package
                    .variables_uuids_lookup
                    .insert(construct_name.clone(), construct_uuid.clone());
                self.commands_instances.insert(
                    construct_uuid.clone(),
                    CommandInstance {
                        specification: commands::new_variable_specification(),
                        name: construct_name.clone(),
                        block: block.clone(),
                        package_uuid: package_uuid.clone(),
                    },
                );
            }
            PreConstructData::Output(block) => {
                package.outputs_uuids.insert(construct_uuid.clone());
                package
                    .outputs_uuids_lookup
                    .insert(construct_name.clone(), construct_uuid.clone());
                self.commands_instances.insert(
                    construct_uuid.clone(),
                    CommandInstance {
                        specification: commands::new_output_specification(),
                        name: construct_name.clone(),
                        block: block.clone(),
                        package_uuid: package_uuid.clone(),
                    },
                );
            }
            PreConstructData::Import(_) => {
                package.imports_uuids.insert(construct_uuid.clone());
                package
                    .imports_uuids_lookup
                    .insert(construct_name.clone(), construct_uuid.clone());
            }
            PreConstructData::Addon(_) => {
                package.addons_uuids.insert(construct_uuid.clone());
                package
                    .addons_uuids_lookup
                    .insert(construct_name.clone(), construct_uuid.clone());
            }
            PreConstructData::Root => unreachable!(),
        }
        let (_, node_index) =
            self.constructs_graph
                .add_child(self.graph_root.clone(), 100, construct_uuid.value());
        self.constructs_graph_nodes
            .insert(construct_uuid.value(), node_index);
        // Update plan
        self.constructs_locations.insert(
            construct_uuid.clone(),
            (package_uuid.clone(), construct_location),
        );
        Ok(package_uuid)
    }

    pub fn try_resolve_construct_reference_in_expression(
        &self,
        package_uuid_source: &PackageUuid,
        expression: &Expression,
    ) -> Result<Option<(ConstructUuid, VecDeque<String>)>, String> {
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
            // Look for modules
            if component.eq_ignore_ascii_case("module") {
                let Some(module_name) = components.pop_front() else {
                    continue;
                };
                if let Some(construct_uuid) = current_package.modules_uuids_lookup.get(&module_name)
                {
                    return Ok(Some((construct_uuid.clone(), components)));
                }
            }

            // Look for outputs
            if component.eq_ignore_ascii_case("output") {
                let Some(output_name) = components.pop_front() else {
                    continue;
                };
                if let Some(construct_uuid) = current_package.outputs_uuids_lookup.get(&output_name)
                {
                    return Ok(Some((construct_uuid.clone(), components)));
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
                    return Ok(Some((construct_uuid.clone(), components)));
                }
            }

            let imported_package = current_package
                .imports_uuids_lookup
                .get(&component.to_string())
                .and_then(|c| self.commands_instances.get(c))
                .and_then(|c| Some(&c.package_uuid))
                .and_then(|p| self.packages.get(&p));

            if let Some(imported_package) = imported_package {
                current_package = imported_package;
                continue;
            }
        }
        Ok(None)
    }
}
