use kit::types::PackageId;
use std::collections::{HashMap, HashSet};
use txtx_addon_kit::types::ConstructDid;

#[derive(Clone, Debug)]
pub struct Package {
    /// Id of the Package
    pub package_id: PackageId,
    pub variables_dids: HashSet<ConstructDid>,
    pub inputs_did_lookup: HashMap<String, ConstructDid>,
    pub outputs_dids: HashSet<ConstructDid>,
    pub outputs_did_lookup: HashMap<String, ConstructDid>,
    pub modules_dids: HashSet<ConstructDid>,
    pub modules_did_lookup: HashMap<String, ConstructDid>,
    pub imports_dids: HashSet<ConstructDid>,
    pub imports_did_lookup: HashMap<String, ConstructDid>,
    pub commands_dids: HashSet<ConstructDid>,
    pub addons_did_lookup: HashMap<String, ConstructDid>,
    pub signing_commands_dids: HashSet<ConstructDid>,
    pub signing_commands_did_lookup: HashMap<String, ConstructDid>,
}

impl Package {
    pub fn new(package_id: &PackageId) -> Self {
        Self {
            package_id: package_id.clone(),
            variables_dids: HashSet::new(),
            inputs_did_lookup: HashMap::new(),
            outputs_dids: HashSet::new(),
            outputs_did_lookup: HashMap::new(),
            modules_dids: HashSet::new(),
            modules_did_lookup: HashMap::new(),
            imports_dids: HashSet::new(),
            imports_did_lookup: HashMap::new(),
            commands_dids: HashSet::new(),
            addons_did_lookup: HashMap::new(),
            signing_commands_dids: HashSet::new(),
            signing_commands_did_lookup: HashMap::new(),
        }
    }
}
