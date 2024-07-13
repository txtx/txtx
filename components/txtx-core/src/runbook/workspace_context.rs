use std::collections::{BTreeMap, HashMap, VecDeque};

use kit::hcl::expr::{Expression, TraversalOperator};
use kit::helpers::fs::FileLocation;
use kit::types::commands::{CommandId, CommandInstance, CommandInstanceType};
use kit::types::types::Value;
use kit::types::wallets::WalletInstance;
use kit::types::{ConstructDid, ConstructId, Did, PackageId};

use crate::std::commands;
use crate::types::{Package, PreConstructData};

pub enum ConstructInstanceType {
    Executable(CommandInstance),
    Signing(WalletInstance),
    Import,
}

#[derive(Debug, Clone)]
pub struct RunbookWorkspaceContext {
    /// Map of packages. A package is either a standalone .tx file, or a directory enclosing multiple .tx files
    pub packages: HashMap<PackageId, Package>,
    /// Map of constructs. A construct refers to root level objects (input, action, output, wallet, import, ...)
    pub constructs: HashMap<ConstructDid, ConstructId>,
    /// Lookup: Retrieve a construct did, given an environment name (mainnet, testnet, etc)
    pub environment_variables_did_lookup: BTreeMap<String, ConstructDid>,
    /// Lookup: Retrieve a construct did, given an environment name (mainnet, testnet, etc)
    pub environment_variables_values: BTreeMap<ConstructDid, String>,
}

impl RunbookWorkspaceContext {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
            constructs: HashMap::new(),
            environment_variables_did_lookup: BTreeMap::new(),
            environment_variables_values: BTreeMap::new(),
        }
    }

    pub fn index_environment_variable(&mut self, key: &String, value: &String) -> ConstructDid {
        let construct_did = ConstructDid(Did::from_components(vec![
            "environment_variable".as_bytes(),
            key.as_bytes(),
        ]));
        self.environment_variables_values
            .insert(construct_did.clone(), value.clone());
        self.environment_variables_did_lookup
            .insert(key.clone(), construct_did.clone());
        construct_did
    }

    pub fn index_package(&mut self, package_id: &PackageId) {
        loop {
            if let Some(_) = self.packages.get(&package_id) {
                break;
            }
            let package = Package::new(package_id);
            self.packages.insert(package_id.clone(), package);
            continue;
        }
    }

    pub fn index_construct(
        &mut self,
        construct_name: String,
        construct_location: FileLocation,
        construct_data: PreConstructData,
        package_id: &PackageId,
    ) -> (ConstructId, ConstructInstanceType) {
        let package = self
            .packages
            .get_mut(&package_id)
            .expect("internal error: unable to retrieve package");
        let construct_id = ConstructId {
            package_id: package_id.clone(),
            construct_type: construct_data.construct_type().into(),
            construct_location,
            construct_name: construct_name.clone(),
        };
        let construct_did = construct_id.did();

        let construct_instance_type = match construct_data {
            PreConstructData::Module(block) => {
                // if construct_name.eq("runbook") && self.runbook_metadata_construct_did.is_none() {
                //     self.runbook_metadata_construct_did = Some(construct_did.clone());
                // }
                package.modules_dids.insert(construct_id.did());
                package
                    .modules_did_lookup
                    .insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Executable(CommandInstance {
                    specification: commands::new_module_specification(),
                    name: construct_name.clone(),
                    block: block.clone(),
                    package_id: package_id.clone(),
                    namespace: construct_name.clone(),
                    typing: CommandInstanceType::Module,
                })
            }
            PreConstructData::Input(block) => {
                package.variables_dids.insert(construct_did.clone());
                package
                    .inputs_did_lookup
                    .insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Executable(CommandInstance {
                    specification: commands::new_input_specification(),
                    name: construct_name.clone(),
                    block: block.clone(),
                    package_id: package_id.clone(),
                    namespace: construct_name.clone(),
                    typing: CommandInstanceType::Input,
                })
            }
            PreConstructData::Output(block) => {
                package.outputs_dids.insert(construct_did.clone());
                package
                    .outputs_did_lookup
                    .insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Executable(CommandInstance {
                    specification: commands::new_output_specification(),
                    name: construct_name.clone(),
                    block: block.clone(),
                    package_id: package_id.clone(),
                    namespace: construct_name.clone(),
                    typing: CommandInstanceType::Output,
                })
            }
            PreConstructData::Import(_) => {
                package.imports_dids.insert(construct_did.clone());
                package
                    .imports_did_lookup
                    .insert(construct_name.clone(), construct_did.clone());
                ConstructInstanceType::Import
            }
            PreConstructData::Action(command_instance) => {
                package.addons_dids.insert(construct_did.clone());
                package.addons_did_lookup.insert(
                    CommandId::Action(construct_name).to_string(),
                    construct_did.clone(),
                );
                ConstructInstanceType::Executable(command_instance)
            }
            PreConstructData::Wallet(wallet_instance) => {
                package.signing_commands_dids.insert(construct_did.clone());
                package
                    .signing_commands_did_lookup
                    .insert(construct_name, construct_did.clone());
                ConstructInstanceType::Signing(wallet_instance)
            }
            PreConstructData::Root => unreachable!(),
        };

        (construct_id, construct_instance_type)
    }

    /// Expects `expression` to be a traversal and `package_did_source` to be indexed in the runbook's `packages`.
    /// Iterates over the operators of `expression` to see if any of the blocks it references are cached as a
    /// `module`, `output`, `input`, `action`, or `prompt` in the package.
    ///
    pub fn try_resolve_construct_reference_in_expression(
        &self,
        source_package_id: &PackageId,
        expression: &Expression,
    ) -> Result<Option<(ConstructDid, VecDeque<String>, VecDeque<Value>)>, String> {
        let Some(traversal) = expression.as_traversal() else {
            return Ok(None);
        };

        let Some(mut current_package) = self.packages.get(source_package_id) else {
            return Ok(None);
        };

        let Some(root) = traversal.expr.as_variable() else {
            return Ok(None);
        };

        let mut subpath = VecDeque::new();

        let mut components = VecDeque::new();
        components.push_front(root.to_string());

        for op in traversal.operators.iter() {
            if let TraversalOperator::GetAttr(value) = op.value() {
                components.push_back(value.to_string());
            }
            if let TraversalOperator::Index(expr) = op.value() {
                match expr {
                    Expression::Number(value) => {
                        subpath.push_back(Value::int(value.as_i64().unwrap()));
                    }
                    Expression::String(value) => {
                        subpath.push_back(Value::string(value.to_string()));
                    }
                    Expression::Bool(value) => {
                        subpath.push_back(Value::bool(**value));
                    }
                    _ => unimplemented!(),
                }
            }
        }

        let mut is_root = true;

        while let Some(component) = components.pop_front() {
            // Look for modules
            if is_root {
                if component.eq_ignore_ascii_case("module") {
                    is_root = false;
                    let Some(module_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) =
                        current_package.modules_did_lookup.get(&module_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for outputs
                if component.eq_ignore_ascii_case("output") {
                    is_root = false;
                    let Some(output_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) =
                        current_package.outputs_did_lookup.get(&output_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for inputs
                if component.eq_ignore_ascii_case("input") {
                    is_root = false;
                    let Some(input_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) = current_package.inputs_did_lookup.get(&input_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for actions
                if component.eq_ignore_ascii_case("action") {
                    is_root = false;
                    let Some(action_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) = current_package
                        .addons_did_lookup
                        .get(&CommandId::Action(action_name).to_string())
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for wallets
                if component.eq_ignore_ascii_case("wallet") {
                    is_root = false;
                    let Some(wallet_name) = components.pop_front() else {
                        continue;
                    };
                    if let Some(construct_did) = current_package
                        .signing_commands_did_lookup
                        .get(&wallet_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }

                // Look for env variables
                if component.eq_ignore_ascii_case("env") {
                    let Some(env_variable_name) = components.pop_front() else {
                        continue;
                    };

                    if let Some(construct_did) = self
                        .environment_variables_did_lookup
                        .get(&env_variable_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }
            }

            let imported_package = current_package
                .imports_did_lookup
                .get(&component.to_string())
                .and_then(|c| self.constructs.get(c))
                .and_then(|c| Some(&c.package_id))
                .and_then(|p| self.packages.get(&p));

            if let Some(imported_package) = imported_package {
                current_package = imported_package;
                continue;
            }
        }
        Ok(None)
    }
}
