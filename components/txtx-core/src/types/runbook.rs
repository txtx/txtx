use crate::std::commands;

use super::Package;
use super::PreConstructData;
use super::RuntimeContext;
use daggy::{Dag, NodeIndex};
use kit::types::commands::CommandInstanceType;
use kit::types::diagnostics::Diagnostic;
use kit::types::frontend::ActionItemRequest;
use kit::types::frontend::ActionItemRequestType;
use kit::types::frontend::ActionItemStatus;
use kit::types::frontend::DisplayOutputRequest;
use kit::types::types::Value;
use kit::types::wallets::SigningCommandsState;
use kit::types::ConstructDid;
use kit::types::ConstructId;
use kit::types::Did;
use kit::types::PackageDid;
use kit::types::PackageId;
use kit::types::RunbookId;
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::VecDeque;
use txtx_addon_kit::hcl::expr::{Expression, TraversalOperator};
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::types::commands::{CommandExecutionResult, CommandInstance};
use txtx_addon_kit::types::commands::{CommandId, CommandInputsEvaluationResult};
use txtx_addon_kit::types::wallets::WalletInstance;

pub struct RunbookSources {
    /// Map of files required to construct the runbook
    pub tree: HashMap<FileLocation, (String, String)>,
}

impl RunbookSources {
    pub fn new() -> Self {
        Self {
            tree: HashMap::new(),
        }
    }

    pub fn add_source(&mut self, name: String, location: FileLocation, content: String) {
        self.tree.insert(location, (name, content));
    }
}

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
    /// Lookup: Retrieve the DAG node id of a given package uuid
    pub packages_dag_node_lookup: HashMap<PackageDid, NodeIndex<u32>>,
    /// Map of constructs. A construct refers to root level objects (input, action, output, wallet, import, ...)
    pub constructs: HashMap<ConstructId, Construct>,
    /// Direct Acyclic Graph keeping track of the dependencies between constructs
    pub constructs_dag: Dag<ConstructDid, u32, u32>,
    /// Lookup: Retrieve the DAG node id of a given construct uuid
    pub constructs_dag_node_lookup: HashMap<ConstructDid, NodeIndex<u32>>,
    /// Keep track of the signing commands (wallet) instantiated (ordered)
    pub instantiated_signing_commands: VecDeque<(ConstructDid, bool)>,
    /// Keep track of the root DAGs (temporary - to be removed)
    pub graph_root: NodeIndex<u32>,
    /// Lookup: Retrieve a construct uuid, given an environment name (mainnet, testnet, etc)
    pub environment_variables_uuid_lookup: BTreeMap<String, ConstructDid>,
    /// Lookup: Retrieve a construct uuid, given an environment name (mainnet, testnet, etc)
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
            environment_variables_uuid_lookup: BTreeMap::new(),
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
                package.modules_uuids.insert(construct_did.clone());
                package
                    .modules_uuids_lookup
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
                package.variables_uuids.insert(construct_did.clone());
                package
                    .inputs_uuids_lookup
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
                package.outputs_uuids.insert(construct_did.clone());
                package
                    .outputs_uuids_lookup
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
                package.imports_uuids.insert(construct_did.clone());
                package
                    .imports_uuids_lookup
                    .insert(construct_name.clone(), construct_did.clone());
            }
            PreConstructData::Action(command_instance) => {
                package.addons_uuids.insert(construct_did.clone());
                package.addons_uuids_lookup.insert(
                    CommandId::Action(construct_name).to_string(),
                    construct_did.clone(),
                );
                execution_context
                    .commands_instances
                    .insert(construct_did.clone(), command_instance.clone());
            }
            PreConstructData::Wallet(wallet_instance) => {
                package.signing_commands_uuids.insert(construct_did.clone());
                package
                    .signing_commands_uuids_lookup
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
            self.environment_variables_uuid_lookup
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
                        current_package.modules_uuids_lookup.get(&module_name)
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
                        current_package.outputs_uuids_lookup.get(&output_name)
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
                    if let Some(construct_did) =
                        current_package.inputs_uuids_lookup.get(&input_name)
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
                        .addons_uuids_lookup
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
                        .signing_commands_uuids_lookup
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
                        .environment_variables_uuid_lookup
                        .get(&env_variable_name)
                    {
                        return Ok(Some((construct_did.clone(), components, subpath)));
                    }
                }
            }

            let imported_package = current_package
                .imports_uuids_lookup
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

#[derive(Debug, Clone)]
pub struct RunbookExecutionContext {
    /// Map of executable commands (input, output, action)
    pub commands_instances: HashMap<ConstructDid, CommandInstance>,
    /// Map of signing commands (wallet)
    pub signing_commands_instances: HashMap<ConstructDid, WalletInstance>,
    /// State of the signing commands states (stateful)
    pub signing_commands_state: Option<SigningCommandsState>,
    /// Results of commands executions
    pub commands_execution_results: HashMap<ConstructDid, CommandExecutionResult>,
    /// Results of commands inputs evaluations
    pub commands_inputs_evaluations_results: HashMap<ConstructDid, CommandInputsEvaluationResult>,
    /// Constructs depending on a given Construct. Keys are sorted in order of execution.
    pub commands_dependencies: BTreeMap<ConstructDid, Vec<ConstructDid>>,
    /// Constructs depending on a given Construct performing signing. Keys are sorted in order of execution.
    pub signing_commands_dependencies: BTreeMap<ConstructDid, Vec<ConstructDid>>,
    /// Commands execution order.
    pub order_for_commands_execution: Vec<ConstructDid>,
    /// Signing commands initialization order.
    pub order_for_signing_commands_initialization: Vec<ConstructDid>,
}

impl RunbookExecutionContext {
    pub fn new() -> Self {
        Self {
            commands_instances: HashMap::new(),
            signing_commands_instances: HashMap::new(),
            signing_commands_state: Some(SigningCommandsState::new()),
            commands_execution_results: HashMap::new(),
            commands_inputs_evaluations_results: HashMap::new(),
            commands_dependencies: BTreeMap::new(),
            signing_commands_dependencies: BTreeMap::new(),
            order_for_commands_execution: vec![],
            order_for_signing_commands_initialization: vec![],
        }
    }

    pub fn serialize_execution(&self) -> serde_json::Value {
        let mut serialized_nodes = vec![];

        for construct_did in self.order_for_commands_execution.iter() {
            let Some(command_instance) = self.commands_instances.get(&construct_did) else {
                // runtime_ctx.addons.index_command_instance(namespace, package_did, block)
                continue;
            };

            let inputs_results = &self
                .commands_inputs_evaluations_results
                .get(&construct_did)
                .unwrap()
                .inputs;

            let outputs_results = &self
                .commands_execution_results
                .get(&construct_did)
                .unwrap()
                .outputs;

            let inputs = command_instance.specification.inputs.iter().map(|i| {
                let value = match (inputs_results.get_value(&i.name), i.optional) {
                    (Some(v), _) => {
                        v.clone()
                    },
                    (None, true) => {
                        Value::null()
                    },
                    _ => panic!("corrupted execution, required input {} missing post execution - investigation required", i.name)
                };
                json!({
                    "name": i.name,
                    "type": value.get_type(),
                    "value": value.to_string()
                })
            }).collect::<Vec<_>>();

            let outputs = command_instance.specification.outputs.iter().map(|o| {
                let output_result = match outputs_results.get(&o.name) {
                    Some(v) => v,
                    None => panic!("corrupted execution, required output {} missing post execution - investigation required", o.name)
                };
                json!({
                    "name": o.name,
                    "value_type": output_result.get_type(),
                    "value": output_result.to_string()
                })
            }).collect::<Vec<_>>();

            serialized_nodes.push(json!({
                "action": command_instance.specification.matcher,
                "inputs": inputs,
                "outputs": outputs,
            }));
        }

        json!({
            "nodes": serialized_nodes
        })
    }

    pub fn collect_runbook_outputs(&self) -> BTreeMap<String, Vec<ActionItemRequest>> {
        let mut action_items = BTreeMap::new();

        for construct_did in self.order_for_commands_execution.iter() {
            let Some(command_instance) = self.commands_instances.get(&construct_did) else {
                // runtime_ctx.addons.index_command_instance(namespace, package_did, block)
                continue;
            };

            if command_instance
                .specification
                .name
                .to_lowercase()
                .eq("output")
            {
                let Some(execution_result) = self.commands_execution_results.get(&construct_did)
                else {
                    return action_items;
                };

                let Some(value) = execution_result.outputs.get("value") else {
                    unreachable!()
                };

                action_items
                    .entry(command_instance.get_group())
                    .or_insert_with(Vec::new)
                    .push(ActionItemRequest::new(
                        &Some(construct_did.clone()),
                        &command_instance.name,
                        None,
                        ActionItemStatus::Todo,
                        ActionItemRequestType::DisplayOutput(DisplayOutputRequest {
                            name: command_instance.name.to_string(),
                            description: None,
                            value: value.clone(),
                        }),
                        "output".into(),
                    ));
            }
        }

        action_items
    }
}

#[derive(Debug, Clone)]
pub struct Runbook {
    /// Id of the Runbook
    pub runbook_id: RunbookId,
    /// Description of the Runbook
    pub description: Option<String>,
    /// The resolution context contains all the data related to source code analysis and DAG construction
    pub resolution_context: RunbookResolutionContext,
    /// The execution context contains all the data related to the execution of the runbook
    pub execution_context: RunbookExecutionContext,
    /// Diagnostics collected over time, until hitting a fatal error
    pub diagnostics: Vec<Diagnostic>,
}

impl Runbook {
    pub fn new(runbook_id: RunbookId, description: Option<String>) -> Self {
        Self {
            runbook_id,
            description,
            resolution_context: RunbookResolutionContext::new(),
            execution_context: RunbookExecutionContext::new(),
            diagnostics: vec![],
        }
    }

    // add attribute "tainting_change"

    // -> Order the nodes
    // -> Compute canonical id for each node
    //      -> traverse all the inputs, same order, only considering the one with "tainting_change" set to true
    // The edges should be slightly differently created: only if a tainting_change is at stake. 5
    // -> Compute canonical id for each edge (?)
    //      ->
}
