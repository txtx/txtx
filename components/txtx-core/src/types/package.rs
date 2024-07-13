use kit::types::PackageId;
use std::collections::{HashMap, HashSet};
use txtx_addon_kit::types::ConstructDid;

#[derive(Clone, Debug)]
pub struct Package {
    /// Id of the Package
    pub package_id: PackageId,
    pub variables_uuids: HashSet<ConstructDid>,
    pub inputs_uuids_lookup: HashMap<String, ConstructDid>,
    pub outputs_uuids: HashSet<ConstructDid>,
    pub outputs_uuids_lookup: HashMap<String, ConstructDid>,
    pub modules_uuids: HashSet<ConstructDid>,
    pub modules_uuids_lookup: HashMap<String, ConstructDid>,
    pub imports_uuids: HashSet<ConstructDid>,
    pub imports_uuids_lookup: HashMap<String, ConstructDid>,
    pub addons_uuids: HashSet<ConstructDid>,
    pub addons_uuids_lookup: HashMap<String, ConstructDid>,
    pub signing_commands_uuids: HashSet<ConstructDid>,
    pub signing_commands_uuids_lookup: HashMap<String, ConstructDid>,
}

impl Package {
    pub fn new(package_id: &PackageId) -> Self {
        Self {
            package_id: package_id.clone(),
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
            signing_commands_uuids: HashSet::new(),
            signing_commands_uuids_lookup: HashMap::new(),
        }
    }
}
