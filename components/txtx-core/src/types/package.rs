use super::ConstructUuid;
use std::collections::{HashMap, HashSet};
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::uuid::Uuid;

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
    pub variables_uuids: HashSet<ConstructUuid>,
    pub variables_uuids_lookup: HashMap<String, ConstructUuid>,
    pub outputs_uuids: HashSet<ConstructUuid>,
    pub outputs_uuids_lookup: HashMap<String, ConstructUuid>,
    pub modules_uuids: HashSet<ConstructUuid>,
    pub modules_uuids_lookup: HashMap<String, ConstructUuid>,
    pub imports_uuids: HashSet<ConstructUuid>,
    pub imports_uuids_lookup: HashMap<String, ConstructUuid>,
    pub addons_uuids: HashSet<ConstructUuid>,
    pub addons_uuids_lookup: HashMap<String, ConstructUuid>,
}

impl Package {
    pub fn new(package_name: &str, package_location: &FileLocation) -> Self {
        let uuid = PackageUuid::new();
        Self {
            uuid,
            name: package_name.to_string(),
            location: package_location.clone(),
            variables_uuids: HashSet::new(),
            variables_uuids_lookup: HashMap::new(),
            outputs_uuids: HashSet::new(),
            outputs_uuids_lookup: HashMap::new(),
            modules_uuids: HashSet::new(),
            modules_uuids_lookup: HashMap::new(),
            imports_uuids: HashSet::new(),
            imports_uuids_lookup: HashMap::new(),
            addons_uuids: HashSet::new(),
            addons_uuids_lookup: HashMap::new(),
        }
    }
}
