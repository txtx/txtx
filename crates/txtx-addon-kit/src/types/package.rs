use std::collections::{HashMap, HashSet};

use super::{ConstructDid, PackageId};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Package {
    /// Id of the Package
    pub package_id: PackageId,
    pub flow_inputs_dids: HashSet<ConstructDid>,
    pub flow_inputs_did_lookup: HashMap<String, ConstructDid>,
    pub variables_dids: HashSet<ConstructDid>,
    pub variables_did_lookup: HashMap<String, ConstructDid>,
    pub outputs_dids: HashSet<ConstructDid>,
    pub outputs_did_lookup: HashMap<String, ConstructDid>,
    pub modules_dids: HashSet<ConstructDid>,
    pub modules_did_lookup: HashMap<String, ConstructDid>,
    pub imports_dids: HashSet<ConstructDid>,
    pub imports_did_lookup: HashMap<String, ConstructDid>,
    pub commands_dids: HashSet<ConstructDid>,
    pub commands_did_lookup: HashMap<String, ConstructDid>,
    pub addons_dids: HashSet<ConstructDid>,
    pub addons_did_lookup: HashMap<String, ConstructDid>,
    pub signers_dids: HashSet<ConstructDid>,
    pub signers_did_lookup: HashMap<String, ConstructDid>,
    pub embeddable_runbooks_dids: HashSet<ConstructDid>,
    pub embeddable_runbooks_did_lookup: HashMap<String, ConstructDid>,
}

impl Package {
    pub fn new(package_id: &PackageId) -> Self {
        Self {
            package_id: package_id.clone(),
            flow_inputs_dids: HashSet::new(),
            flow_inputs_did_lookup: HashMap::new(),
            variables_dids: HashSet::new(),
            variables_did_lookup: HashMap::new(),
            outputs_dids: HashSet::new(),
            outputs_did_lookup: HashMap::new(),
            modules_dids: HashSet::new(),
            modules_did_lookup: HashMap::new(),
            imports_dids: HashSet::new(),
            imports_did_lookup: HashMap::new(),
            commands_dids: HashSet::new(),
            commands_did_lookup: HashMap::new(),
            addons_dids: HashSet::new(),
            addons_did_lookup: HashMap::new(),
            signers_dids: HashSet::new(),
            signers_did_lookup: HashMap::new(),
            embeddable_runbooks_dids: HashSet::new(),
            embeddable_runbooks_did_lookup: HashMap::new(),
        }
    }
}
