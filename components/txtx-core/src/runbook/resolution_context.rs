use crate::std::commands;
use crate::types::Package;
use crate::types::PreConstructData;
use crate::types::RuntimeContext;
use daggy::{Dag, NodeIndex};
use kit::helpers::fs::FileLocation;
use kit::types::commands::CommandInstance;
use kit::types::commands::CommandInstanceType;
use kit::types::types::Value;
use kit::types::ConstructDid;
use kit::types::ConstructId;
use kit::types::Did;
use kit::types::PackageDid;
use kit::types::PackageId;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::VecDeque;
use txtx_addon_kit::hcl::expr::{Expression, TraversalOperator};
use txtx_addon_kit::types::commands::CommandId;

use super::execution_context::RunbookExecutionContext;

#[derive(Debug, Clone)]
pub struct Construct {
    /// Id of the Construct
    pub construct_id: ConstructId,
}

#[derive(Debug, Clone)]
pub struct RunbookResolutionContext {
    /// Map of packages. A package is either a standalone .tx file, or a directory enclosing multiple .tx files
    pub packages: HashMap<PackageId, Package>,
    /// Direct Acyclic Graph keeping track of the dependencies between packages
    pub packages_dag: Dag<PackageDid, u32, u32>,
    /// Lookup: Retrieve the DAG node id of a given package did
    pub packages_dag_node_lookup: HashMap<PackageDid, NodeIndex<u32>>,
    /// Map of constructs. A construct refers to root level objects (input, action, output, wallet, import, ...)
    pub constructs: HashMap<ConstructId, Construct>,
    /// Direct Acyclic Graph keeping track of the dependencies between constructs
    pub constructs_dag: Dag<ConstructDid, u32, u32>,
    /// Lookup: Retrieve the DAG node id of a given construct did
    pub constructs_dag_node_lookup: HashMap<ConstructDid, NodeIndex<u32>>,
    /// Keep track of the signing commands (wallet) instantiated (ordered)
    pub instantiated_signing_commands: VecDeque<(ConstructDid, bool)>,
    /// Keep track of the root DAGs (temporary - to be removed)
    pub graph_root: NodeIndex<u32>,
    /// Lookup: Retrieve a construct did, given an environment name (mainnet, testnet, etc)
    pub environment_variables_did_lookup: BTreeMap<String, ConstructDid>,
    /// Lookup: Retrieve a construct did, given an environment name (mainnet, testnet, etc)
    pub environment_variables_values: BTreeMap<ConstructDid, String>,
}

impl RunbookResolutionContext {
    pub fn new() -> Self {
        // Initialize DAGs
        let mut packages_dag = Dag::new();
        let _ = packages_dag.add_node(PackageDid(Did::zero()));
        let mut constructs_dag = Dag::new();
        let graph_root = constructs_dag.add_node(ConstructDid(Did::zero()));

        Self {
            packages: HashMap::new(),
            packages_dag,
            packages_dag_node_lookup: HashMap::new(),
            constructs: HashMap::new(),
            constructs_dag,
            constructs_dag_node_lookup: HashMap::new(),
            instantiated_signing_commands: VecDeque::new(),
            graph_root,
            environment_variables_did_lookup: BTreeMap::new(),
            environment_variables_values: BTreeMap::new(),
        }
    }

    pub fn find_or_create_package_did(&mut self, package_id: &PackageId) -> PackageDid {
        // Retrieve existing module_uuid, create otherwise
        let package_did = loop {
            if let Some(_) = self.packages.get(&package_id) {
                break package_id.did();
            }
            let package = Package::new(package_id);
            self.packages.insert(package_id.clone(), package);
            self.packages_dag
                .add_child(self.graph_root, 0, package_id.did());
            continue;
        };
        package_did
    }

    pub fn index_construct(
        &mut self,
        construct_name: String,
        construct_location: FileLocation,
        construct_data: PreConstructData,
        package_id: &PackageId,
        execution_context: &mut RunbookExecutionContext,
    ) -> Result<(), String> {
        let Some(package) = self.packages.get_mut(&package_id) else {
            unreachable!()
        };

        let construct_id = ConstructId {
            package_id: package_id.clone(),
            construct_type: construct_data.construct_type().into(),
            construct_location,
            construct_name: construct_name.clone(),
        };
        let construct_did = construct_id.did();
        // Update module
        match &construct_data {
            PreConstructData::Module(block) => {
                // if construct_name.eq("runbook") && self.runbook_metadata_construct_did.is_none() {
                //     self.runbook_metadata_construct_did = Some(construct_did.clone());
                // }
                package.modules_dids.insert(construct_did.clone());
                package
                    .modules_did_lookup
                    .insert(construct_name.clone(), construct_did.clone());
                execution_context.commands_instances.insert(
                    construct_did.clone(),
                    CommandInstance {
                        specification: commands::new_module_specification(),
                        name: construct_name.clone(),
                        block: block.clone(),
                        package_id: package_id.clone(),
                        namespace: construct_name.clone(),
                        typing: CommandInstanceType::Module,
                    },
                );
            }
            PreConstructData::Input(block) => {
                package.variables_dids.insert(construct_did.clone());
                package
                    .inputs_did_lookup
                    .insert(construct_name.clone(), construct_did.clone());
                execution_context.commands_instances.insert(
                    construct_did.clone(),
                    CommandInstance {
                        specification: commands::new_input_specification(),
                        name: construct_name.clone(),
                        block: block.clone(),
                        package_id: package_id.clone(),
                        namespace: construct_name.clone(),
                        typing: CommandInstanceType::Input,
                    },
                );
            }
            PreConstructData::Output(block) => {
                package.outputs_dids.insert(construct_did.clone());
                package
                    .outputs_did_lookup
                    .insert(construct_name.clone(), construct_did.clone());
                execution_context.commands_instances.insert(
                    construct_did.clone(),
                    CommandInstance {
                        specification: commands::new_output_specification(),
                        name: construct_name.clone(),
                        block: block.clone(),
                        package_id: package_id.clone(),
                        namespace: construct_name.clone(),
                        typing: CommandInstanceType::Output,
                    },
                );
            }
            PreConstructData::Import(_) => {
                package.imports_dids.insert(construct_did.clone());
                package
                    .imports_did_lookup
                    .insert(construct_name.clone(), construct_did.clone());
            }
            PreConstructData::Action(command_instance) => {
                package.addons_dids.insert(construct_did.clone());
                package.addons_did_lookup.insert(
                    CommandId::Action(construct_name).to_string(),
                    construct_did.clone(),
                );
                execution_context
                    .commands_instances
                    .insert(construct_did.clone(), command_instance.clone());
            }
            PreConstructData::Wallet(wallet_instance) => {
                package.signing_commands_dids.insert(construct_did.clone());
                package
                    .signing_commands_did_lookup
                    .insert(construct_name, construct_did.clone());
                execution_context
                    .signing_commands_instances
                    .insert(construct_did.clone(), wallet_instance.clone());
            }
            PreConstructData::Root => unreachable!(),
        }
        let (_, node_index) =
            self.constructs_dag
                .add_child(self.graph_root.clone(), 100, construct_did.clone());
        self.constructs_dag_node_lookup
            .insert(construct_did, node_index);
        Ok(())
    }

    pub fn seed_environment_variables(&mut self, runtime_context: &RuntimeContext) {
        for (k, v) in runtime_context
            .get_active_environment_variables()
            .into_iter()
        {
            let construct_did = ConstructDid(Did::from_components(vec![
                "environment_variable".as_bytes(),
                k.as_bytes(),
            ]));
            self.environment_variables_values
                .insert(construct_did.clone(), v);
            self.environment_variables_did_lookup
                .insert(k, construct_did.clone());
            let (_, node_index) =
                self.constructs_dag
                    .add_child(self.graph_root.clone(), 100, construct_did.clone());
            self.constructs_dag_node_lookup
                .insert(construct_did, node_index);
        }
    }

    /// Expects `expression` to be a traversal and `package_did_source` to be indexed in the runbook's `packages`.
    /// Iterates over the operators of `expression` to see if any of the blocks it references are cached as a
    /// `module`, `output`, `input`, `action`, or `prompt` in the package.
    ///
    pub fn try_resolve_construct_reference_in_expression(
        &self,
        source_package_id: &PackageId,
        expression: &Expression,
        execution_context: &RunbookExecutionContext,
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
                .and_then(|c| execution_context.commands_instances.get(c))
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
