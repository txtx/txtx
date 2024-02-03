use super::ConstructUuid;
use daggy::{Dag, NodeIndex};
use std::collections::{HashMap, HashSet};
use txtx_addon_kit::helpers::fs::FileLocation;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PackageUuid {
    Local(Uuid),
}

impl PackageUuid {
    pub fn new() -> Self {
        Self::Local(Uuid::new_v4())
    }

    pub fn value(&self) -> Uuid {
        match &self {
            Self::Local(v) => v.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Package {
    pub uuid: PackageUuid,
    pub name: String,
    pub location: FileLocation,
    pub constructs_graph_root: NodeIndex<u32>,
    pub constructs_graph: Dag<Uuid, u32, u32>,
    pub variables_uuids: HashSet<ConstructUuid>,
    pub variables_uuids_lookup: HashMap<String, ConstructUuid>,
    pub outputs_uuids: HashSet<ConstructUuid>,
    pub outputs_uuids_lookup: HashMap<String, ConstructUuid>,
    pub modules_uuids: HashSet<ConstructUuid>,
    pub modules_uuids_lookup: HashMap<String, ConstructUuid>,
    pub imports_uuids: HashSet<ConstructUuid>,
    pub imports_uuids_lookup: HashMap<String, ConstructUuid>,
    pub exts_uuids: HashSet<ConstructUuid>,
    pub exts_uuids_lookup: HashMap<String, ConstructUuid>,
}

impl Package {
    pub fn new(package_name: &str, package_location: &FileLocation) -> Self {
        let uuid = PackageUuid::new();
        let mut constructs_graph = Dag::new();
        let constructs_graph_root = constructs_graph.add_node(uuid.value());
        Self {
            uuid,
            name: package_name.to_string(),
            location: package_location.clone(),
            constructs_graph,
            constructs_graph_root,
            variables_uuids: HashSet::new(),
            variables_uuids_lookup: HashMap::new(),
            outputs_uuids: HashSet::new(),
            outputs_uuids_lookup: HashMap::new(),
            modules_uuids: HashSet::new(),
            modules_uuids_lookup: HashMap::new(),
            imports_uuids: HashSet::new(),
            imports_uuids_lookup: HashMap::new(),
            exts_uuids: HashSet::new(),
            exts_uuids_lookup: HashMap::new(),
        }
    }
}
