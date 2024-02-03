use super::{
    ConstructData, ConstructUuid, ModuleConstruct, Package, PackageUuid, PreConstruct,
    PreConstructData,
};
use crate::errors::ConstructErrors;
use crate::ExtensionManager;
use daggy::Dag;
use std::{collections::HashMap, ops::Range};
use txtx_ext_kit::hcl::expr::{Expression, TraversalOperator};
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
    pub packages_uuid_lookup: HashMap<(String, FileLocation), PackageUuid>,
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

    pub fn inspect_constructs(&self, extension_manager: &ExtensionManager) {
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
                    let result =
                        self.resolve_construct_reference(package_uuid, dep, &extension_manager);
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
                    let result =
                        self.resolve_construct_reference(package_uuid, dep, &extension_manager);
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

            if !package.outputs_uuids.is_empty() {
                println!("Outputs:");
            }
            for construct_uuid in package.outputs_uuids.iter() {
                let construct = self.constructs.get(construct_uuid).unwrap();
                println!("- {}", construct.as_output().unwrap().name);
                for dep in construct.collect_dependencies().iter() {
                    let result =
                        self.resolve_construct_reference(package_uuid, dep, &extension_manager);
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

            if !package.exts_uuids.is_empty() {
                println!("Extensions:");
            }
            for construct_uuid in package.exts_uuids.iter() {
                let construct = self.constructs.get(construct_uuid).unwrap();
                println!("- {}", construct.as_ext().unwrap().get_name());
                for dep in construct.collect_dependencies().iter() {
                    let result =
                        self.resolve_construct_reference(package_uuid, dep, &extension_manager);
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

    pub fn index_node(
        &mut self,
        name: String,
        location: FileLocation,
        data: PreConstructData,
        span: Range<usize>,
        package_uri: &(String, FileLocation),
    ) {
        // Retrieve existing module_uuid, create otherwise
        let package_uuid = loop {
            match self.packages_uuid_lookup.get(&package_uri) {
                Some(uuid) => break uuid,
                None => {
                    let package = Package::new(&package_uri);
                    self.packages_uuid_lookup
                        .insert(package_uri.clone(), package.uuid.clone());
                    self.packages.insert(package.uuid.clone(), package);
                    continue;
                }
            }
        };

        let Some(package) = self.packages.get_mut(package_uuid) else {
            unreachable!()
        };

        let construct_uuid = ConstructUuid::new();
        // todo: should we be returning a discovery error if the name is already added to a uuid lookup?
        // Update module
        match data {
            PreConstructData::Module(_) => {
                if name.eq("manual") && self.manual_metadata_construct_uuid.is_none() {
                    self.manual_metadata_construct_uuid = Some(construct_uuid.clone());
                }
                package.modules_uuids.insert(construct_uuid.clone());
                package
                    .modules_uuids_lookup
                    .insert(name.clone(), construct_uuid.clone());
            }
            PreConstructData::Variable(_) => {
                package.variables_uuids.insert(construct_uuid.clone());
                package
                    .variables_uuids_lookup
                    .insert(name.clone(), construct_uuid.clone());
            }
            PreConstructData::Output(_) => {
                package.outputs_uuids.insert(construct_uuid.clone());
                package
                    .outputs_uuids_lookup
                    .insert(name.clone(), construct_uuid.clone());
            }
            PreConstructData::Import(_) => {
                package.imports_uuids.insert(construct_uuid.clone());
                package
                    .imports_uuids_lookup
                    .insert(name.clone(), construct_uuid.clone());
            }
            PreConstructData::Ext(ref data) => {
                package.exts_uuids.insert(construct_uuid.clone());
                if let Some(ext_uuids_lookup) =
                    package.exts_uuids_lookup.get_mut(&data.extension_name)
                {
                    ext_uuids_lookup.insert(name.clone(), construct_uuid.clone());
                } else {
                    package.exts_uuids_lookup.insert(
                        data.extension_name.clone(),
                        HashMap::from([(name.clone(), construct_uuid.clone())]),
                    );
                }
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
            name,
            data,
            span,
        };
        self.pre_constructs
            .insert(construct_uuid.clone(), pre_construct);
        self.constructs_locations
            .insert(construct_uuid.clone(), (package_uuid.clone(), location));
    }

    pub fn add_construct(&mut self, uuid: &ConstructUuid, construct: ConstructData) {
        self.constructs.insert(uuid.clone(), construct);
    }

    pub fn resolve_construct_reference(
        &self,
        package_uuid_source: &PackageUuid,
        expression: &Expression,
        extension_manager: &ExtensionManager,
    ) -> Result<Option<ConstructUuid>, String> {
        let Some(traversal) = expression.as_traversal() else {
            return Ok(None);
        };

        let Some(namespace) = traversal.expr.as_variable() else {
            return Ok(None);
        };

        let subsequent_component = match traversal.operators.first().and_then(|o| Some(o.value())) {
            Some(TraversalOperator::GetAttr(value)) => Some(value.value().to_string()),
            _ => None,
        };

        if let Some(subsequent_component) = subsequent_component {
            // Check the reserve keywords (module, output, variable)
            let module = self.packages.get(package_uuid_source).unwrap();

            // Look for modules
            if namespace.eq_ignore_ascii_case("module") {
                if let Some(construct_uuid) = module.modules_uuids_lookup.get(&subsequent_component)
                {
                    return Ok(Some(construct_uuid.clone()));
                }
            }

            // Look for outputs
            if namespace.eq_ignore_ascii_case("output") {
                if let Some(construct_uuid) = module.outputs_uuids_lookup.get(&subsequent_component)
                {
                    return Ok(Some(construct_uuid.clone()));
                }
            }

            // Look for variables
            if namespace.eq_ignore_ascii_case("variable") {
                if let Some(construct_uuid) =
                    module.variables_uuids_lookup.get(&subsequent_component)
                {
                    return Ok(Some(construct_uuid.clone()));
                }
            }
            let namespace_str = namespace.as_str();
            // Look for variables
            if extension_manager
                .registered_extensions
                .get(namespace_str)
                .is_some()
            {
                if let Some(constructs_for_extension) = module.exts_uuids_lookup.get(namespace_str)
                {
                    if let Some(construct_uuid) =
                        constructs_for_extension.get(&subsequent_component)
                    {
                        return Ok(Some(construct_uuid.clone()));
                    }
                }
            }
        }

        Ok(None)
    }
}
