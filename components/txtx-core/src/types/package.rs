use std::collections::{HashMap, HashSet};
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::types::{ConstructUuid, PackageUuid};

#[derive(Clone, Debug)]
pub struct Package {
    pub uuid: PackageUuid,
    pub name: String,
    pub location: FileLocation,
    pub variables_uuids: HashSet<ConstructUuid>,
    pub inputs_uuids_lookup: HashMap<String, ConstructUuid>,
    pub outputs_uuids: HashSet<ConstructUuid>,
    pub outputs_uuids_lookup: HashMap<String, ConstructUuid>,
    pub modules_uuids: HashSet<ConstructUuid>,
    pub modules_uuids_lookup: HashMap<String, ConstructUuid>,
    pub imports_uuids: HashSet<ConstructUuid>,
    pub imports_uuids_lookup: HashMap<String, ConstructUuid>,
    pub addons_uuids: HashSet<ConstructUuid>,
    pub addons_uuids_lookup: HashMap<String, ConstructUuid>,
    pub wallets_uuids: HashSet<ConstructUuid>,
    pub wallets_uuids_lookup: HashMap<String, ConstructUuid>,
}

impl Package {
    pub fn new(package_name: &str, package_location: &FileLocation) -> Self {
        let uuid = PackageUuid::new();
        Self {
            uuid,
            name: package_name.to_string(),
            location: package_location.clone(),
            variables_uuids: HashSet::new(),
            inputs_uuids_lookup: HashMap::new(),
            outputs_uuids: HashSet::new(),
            outputs_uuids_lookup: HashMap::new(),
            modules_uuids: HashSet::new(),
            modules_uuids_lookup: HashMap::new(),
            imports_uuids: HashSet::new(),
            imports_uuids_lookup: HashMap::new(),
            addons_uuids: HashSet::new(),
            addons_uuids_lookup: HashMap::new(),
            wallets_uuids: HashSet::new(),
            wallets_uuids_lookup: HashMap::new(),
        }
    }
}
