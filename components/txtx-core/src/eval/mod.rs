use std::collections::{btree_map::Entry, BTreeMap, HashMap, VecDeque};

use crate::types::{Runbook, RuntimeContext};
use daggy::{Dag, NodeIndex, Walker};
use indexmap::IndexSet;
use petgraph::algo::toposort;
use txtx_addon_kit::{
    hcl::{
        expr::{BinaryOperator, Expression, UnaryOperator},
        template::Element,
    },
    types::{
        commands::{
            CommandExecutionContext, CommandExecutionResult, CommandInputsEvaluationResult,
            CommandInstance, CommandInstanceStateMachineInput, CommandInstanceStateMachineState,
        },
        diagnostics::Diagnostic,
        frontend::{
            ActionItemRequest, ActionItemRequestType, ActionItemResponseType, ActionItemStatus,
            DisplayOutputRequest,
        },
        types::{PrimitiveValue, Value},
        wallets::WalletInstance,
        ConstructUuid, PackageUuid,
    },
    uuid::Uuid,
    AddonDefaults,
};

/// Gets all descendants of `node` within `graph`.
pub fn get_descendants_of_node(node: NodeIndex, graph: Dag<Uuid, u32, u32>) -> IndexSet<NodeIndex> {
    let mut descendant_nodes = VecDeque::new();
    descendant_nodes.push_front(node);
    let mut descendants = IndexSet::new();
    while let Some(node) = descendant_nodes.pop_front() {
        for (_, child) in graph.children(node).iter(&graph) {
            descendant_nodes.push_back(child);
            descendants.insert(child);
        }
    }
    descendants
}

/// Gets all descendants of `node` within `graph` and returns them, topologically sorted.
pub fn get_sorted_descendants_of_node(
    node: NodeIndex,
    graph: Dag<Uuid, u32, u32>,
) -> IndexSet<NodeIndex> {
    let sorted = toposort(&graph, None)
        .unwrap()
        .into_iter()
        .collect::<IndexSet<NodeIndex>>();

    let start_node_descendants = get_descendants_of_node(node, graph);
    let mut sorted_descendants = IndexSet::new();

    for this_node in sorted.into_iter() {
        let is_descendant = start_node_descendants.iter().any(|d| d == &this_node);
        let is_start_node = this_node == node;
        if is_descendant || is_start_node {
            sorted_descendants.insert(this_node);
        }
    }
    sorted_descendants
}

/// Returns a topologically sorted set of all nodes in the graph.
pub fn get_sorted_nodes(graph: Dag<Uuid, u32, u32>) -> IndexSet<NodeIndex> {
    toposort(&graph, None)
        .unwrap()
        .into_iter()
        .collect::<IndexSet<NodeIndex>>()
}

pub fn is_child_of_node(
    node: NodeIndex,
    maybe_child: NodeIndex,
    graph: &Dag<Uuid, u32, u32>,
) -> bool {
    graph
        .children(node)
        .iter(graph)
        .any(|(_, child)| child == maybe_child)
}

pub fn log_evaluated_outputs(runbook: &Runbook) {
    for (_, package) in runbook.packages.iter() {
        for construct_uuid in package.outputs_uuids.iter() {
            let _construct = runbook.commands_instances.get(construct_uuid).unwrap();
            if let Some(result) = runbook.constructs_execution_results.get(construct_uuid) {
                match result {
                    Ok(result) => {
                        for (key, value) in result.outputs.iter() {
                            println!("- {}: {:?}", key, value);
                        }
                    }
                    Err(e) => {
                        println!(" - {e}")
                    }
                }
            } else {
                println!(" - (no execution results)")
            }
        }
    }
}
pub enum ConstructEvaluationStatus {
    Complete,
    NeedsUserInteraction(Vec<NodeIndex>),
}

/// Prepares for a reevaluation of all of `start_node`'s dependents within the `runbook`.
/// This involves setting the command instance state to `New` for all commands _except_
/// the start node. The `New` state indicates to the evaluation loop that the data
/// should be recomputed, and this should occur for all dependents of the updated
/// start node, but not the start node itself.
pub fn prepare_constructs_reevaluation(runbook: &mut Runbook, start_node: NodeIndex) {
    let g = runbook.constructs_graph.clone();
    let nodes_to_reevaluate =
        get_sorted_descendants_of_node(start_node, runbook.constructs_graph.clone());

    for node in nodes_to_reevaluate.into_iter() {
        let uuid = g.node_weight(node).expect("unable to retrieve construct");
        let construct_uuid = ConstructUuid::Local(uuid.clone());

        let Some(command_instance) = runbook.commands_instances.get_mut(&construct_uuid) else {
            continue;
        };
        if node == start_node {
            continue;
        }

        match command_instance.state_machine.state() {
            CommandInstanceStateMachineState::New | CommandInstanceStateMachineState::Failed => {}
            _ => {
                command_instance
                    .state_machine
                    .consume(&CommandInstanceStateMachineInput::ReEvaluate)
                    .unwrap();
            }
        };
    }
}

pub async fn run_wallets_evaluation(
    runbook: &mut Runbook,
    runtime_ctx: &mut RuntimeContext,
    execution_context: &CommandExecutionContext,
    action_item_requests: &mut BTreeMap<Uuid, Vec<&mut ActionItemRequest>>,
    action_item_responses: &BTreeMap<Uuid, Vec<ActionItemResponseType>>,
    progress_tx: &txtx_addon_kit::channel::Sender<(ConstructUuid, Diagnostic)>,
) -> Result<BTreeMap<String, Vec<ActionItemRequest>>, Vec<Diagnostic>> {
    let mut action_items: BTreeMap<String, Vec<ActionItemRequest>> = BTreeMap::new();

    let environments_variables = runbook.environment_variables_values.clone();
    for (env_variable_uuid, value) in environments_variables.into_iter() {
        let mut res = CommandExecutionResult::new();
        res.outputs.insert("value".into(), Value::string(value));
        runbook
            .constructs_execution_results
            .insert(env_variable_uuid, Ok(res));
    }

    let constructs_locations = runbook.constructs_locations.clone();

    for (construct_uuid, wallet_instance) in runbook.wallet_instances.clone() {
        let (package_uuid, _) = constructs_locations.get(&construct_uuid).unwrap();
        let mut cached_dependency_execution_results: HashMap<
            ConstructUuid,
            Result<&CommandExecutionResult, &Diagnostic>,
        > = HashMap::new();

        let references_expressions: Vec<Expression> = wallet_instance
            .get_expressions_referencing_commands_from_inputs()
            .unwrap();

        for expr in references_expressions.into_iter() {
            let res = runbook
                .try_resolve_construct_reference_in_expression(package_uuid, &expr, &runtime_ctx)
                .unwrap();

            if let Some((dependency, _)) = res {
                let evaluation_result_opt = runbook.constructs_execution_results.get(&dependency);

                if let Some(evaluation_result) = evaluation_result_opt {
                    match cached_dependency_execution_results.get(&dependency) {
                        None => match evaluation_result {
                            Ok(evaluation_result) => {
                                cached_dependency_execution_results
                                    .insert(dependency, Ok(evaluation_result));
                            }
                            Err(e) => {
                                cached_dependency_execution_results.insert(dependency, Err(e));
                            }
                        },
                        Some(Err(_)) => continue,
                        Some(Ok(_)) => {}
                    }
                }
            }
        }

        let input_evaluation_results = runbook
            .command_inputs_evaluation_results
            .get(&construct_uuid.clone());

        let evaluated_inputs_res = perform_wallet_inputs_evaluation(
            &wallet_instance,
            &cached_dependency_execution_results,
            &input_evaluation_results,
            package_uuid,
            &runbook,
            runtime_ctx,
        );

        let Some(wallet_instance) = runbook.wallet_instances.get_mut(&construct_uuid) else {
            // runtime_ctx.addons.index_command_instance(namespace, package_uuid, block)
            continue;
        };

        let mut evaluated_inputs = match evaluated_inputs_res {
            Ok(result) => match result {
                CommandInputEvaluationStatus::Complete(result) => result,
                CommandInputEvaluationStatus::NeedsUserInteraction => {
                    wallet_instance
                        .state
                        .consume(&CommandInstanceStateMachineInput::NeedsUserInput)
                        .unwrap();
                    continue;
                }
                CommandInputEvaluationStatus::Aborted(result) => {
                    let mut diags = vec![];
                    for (_k, res) in result.inputs.into_iter() {
                        if let Err(diag) = res {
                            diags.push(diag);
                        }
                    }
                    return Err(diags);
                }
            },
            Err(e) => {
                todo!("build input evaluation diagnostic: {}", e)
            }
        };

        let addon_context_key = (package_uuid.clone(), wallet_instance.namespace.clone());
        let addon_defaults = runtime_ctx
            .addons_ctx
            .contexts
            .get(&addon_context_key)
            .and_then(|addon| Some(addon.defaults.clone()))
            .unwrap_or(AddonDefaults::new());

        let mut empty_vec = vec![];
        let action_items_requests = action_item_requests
            .get_mut(&construct_uuid.value())
            .unwrap_or(&mut empty_vec);

        if let Err(ref mut new_items) = wallet_instance.check_executability(
            &construct_uuid,
            &mut evaluated_inputs,
            addon_defaults.clone(),
            action_items_requests,
            &action_item_responses.get(&construct_uuid.value()),
            execution_context,
        ) {
            match action_items.entry(wallet_instance.get_group()) {
                Entry::Occupied(mut e) => {
                    e.get_mut().append(new_items);
                }
                Entry::Vacant(e) => {
                    e.insert(new_items.clone());
                }
            };
            continue;
        }

        runbook
            .command_inputs_evaluation_results
            .insert(construct_uuid.clone(), evaluated_inputs.clone());

        let execution_result = {
            wallet_instance
                .perform_execution(
                    &construct_uuid,
                    &evaluated_inputs,
                    addon_defaults.clone(),
                    progress_tx,
                )
                .await
        };

        let execution_result = match execution_result {
            // todo(lgalabru): return Diagnostic instead
            Ok(result) => {
                if wallet_instance.specification.update_addon_defaults {
                    let addon_context_key =
                        (package_uuid.clone(), wallet_instance.namespace.clone());
                    if let Some(ref mut addon_context) =
                        runtime_ctx.addons_ctx.contexts.get_mut(&addon_context_key)
                    {
                        for (k, v) in result.outputs.iter() {
                            addon_context.defaults.keys.insert(k.clone(), v.to_string());
                        }
                    }
                }
                wallet_instance
                    .state
                    .consume(&CommandInstanceStateMachineInput::Successful)
                    .unwrap();
                Ok(result)
            }
            Err(e) => {
                wallet_instance
                    .state
                    .consume(&CommandInstanceStateMachineInput::Unsuccessful)
                    .unwrap();
                Err(e)
            }
        };
        runbook
            .constructs_execution_results
            .insert(construct_uuid, execution_result);
    }
    Ok(action_items)
}

pub async fn run_constructs_evaluation(
    runbook: &mut Runbook,
    runtime_ctx: &mut RuntimeContext,
    start_node: Option<NodeIndex>,
    execution_context: &CommandExecutionContext,
    action_item_responses: &BTreeMap<Uuid, Vec<ActionItemResponseType>>,
    progress_tx: &txtx_addon_kit::channel::Sender<(ConstructUuid, Diagnostic)>,
) -> Result<BTreeMap<String, Vec<ActionItemRequest>>, Vec<Diagnostic>> {
    let g = runbook.constructs_graph.clone();

    let mut action_items: BTreeMap<String, Vec<ActionItemRequest>> = BTreeMap::new();
    let wallet_instances = runbook.wallet_instances.clone();

    let environments_variables = runbook.environment_variables_values.clone();
    for (env_variable_uuid, value) in environments_variables.into_iter() {
        let mut res = CommandExecutionResult::new();
        res.outputs.insert("value".into(), Value::string(value));
        runbook
            .constructs_execution_results
            .insert(env_variable_uuid, Ok(res));
    }

    let ordered_nodes_to_process = match start_node {
        Some(start_node) => {
            // if we are walking the graph from a given start node, we only add the
            // node and its dependents (not its parents) to the nodes we visit.
            get_sorted_descendants_of_node(start_node, runbook.constructs_graph.clone())
        }
        None => get_sorted_nodes(runbook.constructs_graph.clone()),
    };

    let constructs_locations = runbook.constructs_locations.clone();

    for node in ordered_nodes_to_process.into_iter() {
        let uuid = g.node_weight(node).expect("unable to retrieve construct");
        let construct_uuid = ConstructUuid::Local(uuid.clone());

        let Some(command_instance) = runbook.commands_instances.get(&construct_uuid) else {
            // runtime_ctx.addons.index_command_instance(namespace, package_uuid, block)
            continue;
        };

        let (package_uuid, _) = constructs_locations.get(&construct_uuid).unwrap();

        let addon_context_key = (package_uuid.clone(), command_instance.namespace.clone());
        let addon_defaults = runtime_ctx
            .addons_ctx
            .contexts
            .get(&addon_context_key)
            .and_then(|addon| Some(addon.defaults.clone()))
            .unwrap_or(AddonDefaults::new()); // todo(lgalabru): to investigate

        match command_instance.state_machine.state() {
            CommandInstanceStateMachineState::Failed => {
                println!("continuing past failed state command");
                continue;
            }
            _ => {}
        }
        // in general we want to ignore previous input evaluation results when evaluating for outputs.
        // we want to recompute the whole graph in case anything has changed since our last traversal.
        // however, if there was a start_node provided, this evaluation was initiated from a user interaction
        // that is stored in the input evaluation results, and we want to keep that data to evaluate that
        // commands dependents
        let input_evaluation_results = if let Some(start_node) = start_node {
            if start_node == node {
                runbook
                    .command_inputs_evaluation_results
                    .get(&construct_uuid.clone())
            } else {
                None
            }
        } else {
            None
        };

        let mut cached_dependency_execution_results: HashMap<
            ConstructUuid,
            Result<&CommandExecutionResult, &Diagnostic>,
        > = HashMap::new();

        // Retrieve the construct_uuid of the inputs
        // Collect the outputs
        let references_expressions: Vec<Expression> = command_instance
            .get_expressions_referencing_commands_from_inputs()
            .unwrap();

        for expr in references_expressions.into_iter() {
            let res = runbook
                .try_resolve_construct_reference_in_expression(package_uuid, &expr, &runtime_ctx)
                .unwrap();

            if let Some((dependency, _)) = res {
                let evaluation_result_opt = runbook.constructs_execution_results.get(&dependency);

                if let Some(evaluation_result) = evaluation_result_opt {
                    match cached_dependency_execution_results.get(&dependency) {
                        None => match evaluation_result {
                            Ok(evaluation_result) => {
                                cached_dependency_execution_results
                                    .insert(dependency, Ok(evaluation_result));
                            }
                            Err(e) => {
                                cached_dependency_execution_results.insert(dependency, Err(e));
                            }
                        },
                        Some(Err(_)) => continue,
                        Some(Ok(_)) => {}
                    }
                }
            }
        }

        match command_instance.state_machine.state() {
            CommandInstanceStateMachineState::Evaluated
            | CommandInstanceStateMachineState::Failed
            | CommandInstanceStateMachineState::AwaitingAsyncRequest => {
                continue;
            }
            _ => {}
        }

        let evaluated_inputs_res = perform_inputs_evaluation(
            command_instance,
            &cached_dependency_execution_results,
            &input_evaluation_results,
            package_uuid,
            &runbook,
            runtime_ctx,
        );

        let Some(command_instance) = runbook.commands_instances.get_mut(&construct_uuid) else {
            // runtime_ctx.addons.index_command_instance(namespace, package_uuid, block)
            continue;
        };

        let mut evaluated_inputs = match evaluated_inputs_res {
            Ok(result) => match result {
                CommandInputEvaluationStatus::Complete(result) => result,
                CommandInputEvaluationStatus::NeedsUserInteraction => {
                    command_instance
                        .state_machine
                        .consume(&CommandInstanceStateMachineInput::NeedsUserInput)
                        .unwrap();
                    continue;
                }
                CommandInputEvaluationStatus::Aborted(result) => {
                    let mut diags = vec![];
                    for (_k, res) in result.inputs.into_iter() {
                        if let Err(diag) = res {
                            diags.push(diag);
                        }
                    }
                    return Err(diags);
                }
            },
            Err(e) => {
                todo!("build input evaluation diagnostic: {}", e)
            }
        };

        if let Err(ref mut new_items) = command_instance.check_executability(
            &construct_uuid,
            &mut evaluated_inputs,
            addon_defaults.clone(),
            &wallet_instances,
            &action_item_responses.get(&construct_uuid.value()),
            execution_context,
        ) {
            match action_items.entry(command_instance.get_group()) {
                Entry::Occupied(mut e) => {
                    e.get_mut().append(new_items);
                }
                Entry::Vacant(e) => {
                    e.insert(new_items.clone());
                }
            };
            continue;
        }

        runbook
            .command_inputs_evaluation_results
            .insert(construct_uuid.clone(), evaluated_inputs.clone());

        let execution_result = {
            command_instance
                .perform_execution(
                    &construct_uuid,
                    &evaluated_inputs,
                    addon_defaults.clone(),
                    &wallet_instances,
                    progress_tx,
                )
                .await
        };

        let execution_result = match execution_result {
            // todo(lgalabru): return Diagnostic instead
            Ok(result) => {
                if command_instance.specification.update_addon_defaults {
                    let addon_context_key =
                        (package_uuid.clone(), command_instance.namespace.clone());
                    if let Some(ref mut addon_context) =
                        runtime_ctx.addons_ctx.contexts.get_mut(&addon_context_key)
                    {
                        for (k, v) in result.outputs.iter() {
                            addon_context.defaults.keys.insert(k.clone(), v.to_string());
                        }
                    }
                }
                command_instance
                    .state_machine
                    .consume(&CommandInstanceStateMachineInput::Successful)
                    .unwrap();
                Ok(result)
            }
            Err(e) => {
                command_instance
                    .state_machine
                    .consume(&CommandInstanceStateMachineInput::Unsuccessful)
                    .unwrap();
                Err(e)
            }
        };
        runbook
            .constructs_execution_results
            .insert(construct_uuid, execution_result);
    }

    Ok(action_items)
}

pub fn collect_runbook_outputs(
    block_uuid: &Uuid,
    runbook: &Runbook,
    _runtime_ctx: &RuntimeContext,
) -> BTreeMap<String, Vec<ActionItemRequest>> {
    let g = runbook.constructs_graph.clone();

    let mut action_items = BTreeMap::new();

    let ordered_nodes_to_process = get_sorted_nodes(runbook.constructs_graph.clone());

    for node in ordered_nodes_to_process.into_iter() {
        let uuid = g.node_weight(node).expect("unable to retrieve construct");
        let construct_uuid = ConstructUuid::Local(uuid.clone());

        let Some(command_instance) = runbook.commands_instances.get(&construct_uuid) else {
            // runtime_ctx.addons.index_command_instance(namespace, package_uuid, block)
            continue;
        };

        if command_instance
            .specification
            .name
            .to_lowercase()
            .eq("output")
        {
            let Some(Ok(execution_result)) =
                runbook.constructs_execution_results.get(&construct_uuid)
            else {
                unreachable!()
            };

            let Some(value) = execution_result.outputs.get("value") else {
                unreachable!()
            };

            action_items
                .entry(command_instance.get_group())
                .or_insert_with(Vec::new)
                .push(ActionItemRequest {
                    uuid: Uuid::new_v4(),
                    construct_uuid: Some(construct_uuid.value()),
                    index: 0,
                    title: command_instance.name.to_string(),
                    description: "".to_string(),
                    action_status: ActionItemStatus::Todo,
                    action_type: ActionItemRequestType::DisplayOutput(DisplayOutputRequest {
                        name: command_instance.name.to_string(),
                        description: None,
                        value: value.clone(),
                    }),
                });
        }
    }

    action_items
}

pub enum ExpressionEvaluationStatus {
    CompleteOk(Value),
    CompleteErr(Diagnostic),
    DependencyNotComputed,
}

pub fn eval_expression(
    expr: &Expression,
    dependencies_execution_results: &HashMap<
        ConstructUuid,
        Result<&CommandExecutionResult, &Diagnostic>,
    >,
    package_uuid: &PackageUuid,
    runbook: &Runbook,
    runtime_ctx: &RuntimeContext,
) -> Result<ExpressionEvaluationStatus, Diagnostic> {
    let value = match expr {
        // Represents a null value.
        Expression::Null(_decorated_null) => Value::null(),
        // Represents a boolean.
        Expression::Bool(decorated_bool) => Value::bool(*decorated_bool.value()),
        // Represents a number, either integer or float.
        Expression::Number(formatted_number) => {
            match (
                formatted_number.value().as_u64(),
                formatted_number.value().as_i64(),
                formatted_number.value().as_f64(),
            ) {
                (Some(value), _, _) => Value::uint(value),
                (_, Some(value), _) => Value::int(value),
                (_, _, Some(value)) => Value::float(value),
                (None, None, None) => unreachable!(), // todo(lgalabru): return Diagnostic
            }
        }
        // Represents a string that does not contain any template interpolations or template directives.
        Expression::String(decorated_string) => Value::string(decorated_string.to_string()),
        // Represents an HCL array.
        Expression::Array(entries) => {
            let mut res = vec![];
            for entry_expr in entries {
                let value = match eval_expression(
                    entry_expr,
                    dependencies_execution_results,
                    package_uuid,
                    runbook,
                    runtime_ctx,
                )? {
                    ExpressionEvaluationStatus::CompleteOk(result) => result,
                    ExpressionEvaluationStatus::CompleteErr(e) => {
                        return Ok(ExpressionEvaluationStatus::CompleteErr(e))
                    }
                    ExpressionEvaluationStatus::DependencyNotComputed => {
                        return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                    }
                };
                res.push(value);
            }
            Value::array(res)
        }
        // Represents an HCL object.
        Expression::Object(object) => {
            let mut map = HashMap::new();
            for (k, v) in object.into_iter() {
                let key = match k {
                    txtx_addon_kit::hcl::expr::ObjectKey::Expression(k_expr) => {
                        match eval_expression(
                            k_expr,
                            dependencies_execution_results,
                            package_uuid,
                            runbook,
                            runtime_ctx,
                        )? {
                            ExpressionEvaluationStatus::CompleteOk(result) => match result {
                                Value::Primitive(PrimitiveValue::String(result)) => result,
                                Value::Primitive(_)
                                | Value::Addon(_)
                                | Value::Array(_)
                                | Value::Object(_) => {
                                    return Ok(ExpressionEvaluationStatus::CompleteErr(
                                        Diagnostic::error_from_string(
                                            "object key must evaluate to a string".to_string(),
                                        ),
                                    ))
                                }
                            },
                            ExpressionEvaluationStatus::CompleteErr(e) => {
                                return Ok(ExpressionEvaluationStatus::CompleteErr(e))
                            }
                            ExpressionEvaluationStatus::DependencyNotComputed => {
                                return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                            }
                        }
                    }
                    txtx_addon_kit::hcl::expr::ObjectKey::Ident(k_ident) => k_ident.to_string(),
                };
                let value = match eval_expression(
                    v.expr(),
                    dependencies_execution_results,
                    package_uuid,
                    runbook,
                    runtime_ctx,
                )? {
                    ExpressionEvaluationStatus::CompleteOk(result) => Ok(result),
                    ExpressionEvaluationStatus::CompleteErr(e) => Err(e),
                    ExpressionEvaluationStatus::DependencyNotComputed => {
                        return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                    }
                };
                map.insert(key, value);
            }
            Value::Object(map)
        }
        // Represents a string containing template interpolations and template directives.
        Expression::StringTemplate(string_template) => {
            let mut res = String::new();
            for element in string_template.into_iter() {
                match element {
                    Element::Literal(literal) => {
                        res.push_str(literal.value());
                    }
                    Element::Interpolation(interpolation) => {
                        let value = match eval_expression(
                            &interpolation.expr,
                            dependencies_execution_results,
                            package_uuid,
                            runbook,
                            runtime_ctx,
                        )? {
                            ExpressionEvaluationStatus::CompleteOk(result) => result.to_string(),
                            ExpressionEvaluationStatus::CompleteErr(e) => {
                                return Ok(ExpressionEvaluationStatus::CompleteErr(e))
                            }
                            ExpressionEvaluationStatus::DependencyNotComputed => {
                                return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                            }
                        };
                        res.push_str(&value);
                    }
                    Element::Directive(_) => {
                        unimplemented!("string templates with directives not yet supported")
                    }
                };
            }
            Value::string(res)
        }
        // Represents an HCL heredoc template.
        Expression::HeredocTemplate(_heredoc_template) => {
            unimplemented!()
        }
        // Represents a sub-expression wrapped in parenthesis.
        Expression::Parenthesis(_sub_expr) => {
            unimplemented!()
        }
        // Represents a variable identifier.
        Expression::Variable(_decorated_var) => {
            unimplemented!()
        }
        // Represents conditional operator which selects one of two expressions based on the outcome of a boolean expression.
        Expression::Conditional(_conditional) => {
            unimplemented!()
        }
        // Represents a function call.
        Expression::FuncCall(function_call) => {
            let func_namespace = function_call.name.namespace.first().map(|n| n.to_string());
            let func_name = function_call.name.name.to_string();
            let mut args = vec![];
            for expr in function_call.args.iter() {
                let value = match eval_expression(
                    expr,
                    dependencies_execution_results,
                    package_uuid,
                    runbook,
                    runtime_ctx,
                )? {
                    ExpressionEvaluationStatus::CompleteOk(result) => result,
                    ExpressionEvaluationStatus::CompleteErr(e) => {
                        return Ok(ExpressionEvaluationStatus::CompleteErr(e))
                    }
                    ExpressionEvaluationStatus::DependencyNotComputed => {
                        return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                    }
                };
                args.push(value);
            }
            runtime_ctx
                .execute_function(package_uuid.clone(), func_namespace, &func_name, &args)
                .map_err(|e| {
                    // todo: add more context to error
                    e
                })?
        }
        // Represents an attribute or element traversal.
        Expression::Traversal(_) => {
            let Ok(Some((dependency, mut components))) = runbook
                .try_resolve_construct_reference_in_expression(package_uuid, expr, runtime_ctx)
            else {
                todo!("implement diagnostic for unresolvable references")
            };
            let res: &CommandExecutionResult = match dependencies_execution_results.get(&dependency)
            {
                Some(res) => match res.clone() {
                    Ok(res) => res,
                    Err(e) => return Ok(ExpressionEvaluationStatus::CompleteErr(e.clone())),
                },
                None => return Ok(ExpressionEvaluationStatus::DependencyNotComputed),
            };
            let attribute = components.pop_front().unwrap_or("value".into());
            match res.outputs.get(&attribute) {
                Some(output) => output.clone(),
                None => return Ok(ExpressionEvaluationStatus::DependencyNotComputed),
            }
        }
        // Represents an operation which applies a unary operator to an expression.
        Expression::UnaryOp(unary_op) => {
            let _expr = eval_expression(
                &unary_op.expr,
                dependencies_execution_results,
                package_uuid,
                runbook,
                runtime_ctx,
            )?;
            match &unary_op.operator.value() {
                UnaryOperator::Neg => {}
                UnaryOperator::Not => {}
            }
            unimplemented!()
        }
        // Represents an operation which applies a binary operator to two expressions.
        Expression::BinaryOp(binary_op) => {
            let lhs = match eval_expression(
                &binary_op.lhs_expr,
                dependencies_execution_results,
                package_uuid,
                runbook,
                runtime_ctx,
            )? {
                ExpressionEvaluationStatus::CompleteOk(result) => result,
                ExpressionEvaluationStatus::CompleteErr(e) => {
                    return Ok(ExpressionEvaluationStatus::CompleteErr(e))
                }
                ExpressionEvaluationStatus::DependencyNotComputed => {
                    return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                }
            };
            let rhs = match eval_expression(
                &binary_op.rhs_expr,
                dependencies_execution_results,
                package_uuid,
                runbook,
                runtime_ctx,
            )? {
                ExpressionEvaluationStatus::CompleteOk(result) => result,
                ExpressionEvaluationStatus::CompleteErr(e) => {
                    return Ok(ExpressionEvaluationStatus::CompleteErr(e))
                }
                ExpressionEvaluationStatus::DependencyNotComputed => {
                    return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                }
            };
            if !lhs.is_type_eq(&rhs) {
                unimplemented!() // todo(lgalabru): return diagnostic
            }

            let func = match &binary_op.operator.value() {
                BinaryOperator::And => "and_bool",
                BinaryOperator::Div => match rhs {
                    Value::Primitive(PrimitiveValue::SignedInteger(_)) => "div_int",
                    _ => "div_uint",
                },
                BinaryOperator::Eq => "eq",
                BinaryOperator::Greater => "gt",
                BinaryOperator::GreaterEq => "gte",
                BinaryOperator::Less => "lt",
                BinaryOperator::LessEq => "lte",
                BinaryOperator::Minus => "minus_uint",
                BinaryOperator::Mod => "modulo_uint",
                BinaryOperator::Mul => "multiply_uint",
                BinaryOperator::Plus => "add_uint",
                BinaryOperator::NotEq => "neq",
                BinaryOperator::Or => "or_bool",
            };
            runtime_ctx.execute_function(package_uuid.clone(), None, func, &vec![lhs, rhs])?
        }
        // Represents a construct for constructing a collection by projecting the items from another collection.
        Expression::ForExpr(_for_expr) => {
            unimplemented!()
        }
    };

    Ok(ExpressionEvaluationStatus::CompleteOk(value))
}

// pub struct EvaluatedExpression {
//     value: Value,
// }

pub enum CommandInputEvaluationStatus {
    Complete(CommandInputsEvaluationResult),
    NeedsUserInteraction,
    Aborted(CommandInputsEvaluationResult),
}

pub fn perform_inputs_evaluation(
    command_instance: &CommandInstance,
    dependencies_execution_results: &HashMap<
        ConstructUuid,
        Result<&CommandExecutionResult, &Diagnostic>,
    >,
    input_evaluation_results: &Option<&CommandInputsEvaluationResult>,
    package_uuid: &PackageUuid,
    runbook: &Runbook,
    runtime_ctx: &RuntimeContext,
) -> Result<CommandInputEvaluationStatus, Diagnostic> {
    let mut results = CommandInputsEvaluationResult::new();
    let inputs = command_instance.specification.inputs.clone();
    let mut fatal_error = false;

    for input in inputs.into_iter() {
        // todo(micaiah): this value still needs to be for inputs that are objects
        let previously_evaluated_input = match input_evaluation_results {
            Some(input_evaluation_results) => input_evaluation_results.inputs.get(&input.name),
            None => None,
        };
        if let Some(object_props) = input.as_object() {
            // todo(micaiah) - figure out how user-input values work for this branch
            let mut object_values = HashMap::new();
            for prop in object_props.iter() {
                if let Some(value) = previously_evaluated_input {
                    match value.clone() {
                        Ok(Value::Object(obj)) => {
                            for (k, v) in obj.into_iter() {
                                object_values.insert(k, v);
                            }
                        }
                        Ok(v) => {
                            object_values.insert(prop.name.to_string(), Ok(v));
                        }
                        Err(diag) => {
                            object_values.insert(prop.name.to_string(), Err(diag));
                        }
                    };
                }

                let Some(expr) =
                    command_instance.get_expression_from_object_property(&input, &prop)?
                else {
                    continue;
                };
                let value = match eval_expression(
                    &expr,
                    dependencies_execution_results,
                    package_uuid,
                    runbook,
                    runtime_ctx,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => Ok(result),
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => Err(e),
                    Err(e) => Err(e),
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                    }
                };

                if let Err(ref diag) = value {
                    if diag.is_error() {
                        fatal_error = true;
                    }
                }

                match value.clone() {
                    Ok(Value::Object(obj)) => {
                        for (k, v) in obj.into_iter() {
                            object_values.insert(k, v);
                        }
                    }
                    Ok(v) => {
                        object_values.insert(prop.name.to_string(), Ok(v));
                    }
                    Err(diag) => {
                        object_values.insert(prop.name.to_string(), Err(diag));
                    }
                };
            }
            if !object_values.is_empty() {
                results.insert(&input.name, Ok(Value::Object(object_values)));
            }
        } else if let Some(_) = input.as_array() {
            let mut array_values = vec![];
            if let Some(value) = previously_evaluated_input {
                match value.clone() {
                    Ok(Value::Array(entries)) => {
                        array_values.extend::<Vec<Value>>(entries.into_iter().collect());
                    }
                    Err(diag) => {
                        results.insert(&input.name, Err(diag));
                        continue;
                    }
                    Ok(Value::Primitive(_)) | Ok(Value::Object(_)) | Ok(Value::Addon(_)) => {
                        unreachable!()
                    }
                }
            }

            let Some(expr) = command_instance.get_expression_from_input(&input)? else {
                continue;
            };
            let value = match eval_expression(
                &expr,
                dependencies_execution_results,
                package_uuid,
                runbook,
                runtime_ctx,
            ) {
                Ok(ExpressionEvaluationStatus::CompleteOk(result)) => match result {
                    Value::Primitive(_) | Value::Object(_) | Value::Addon(_) => unreachable!(),
                    Value::Array(entries) => {
                        for (i, entry) in entries.into_iter().enumerate() {
                            array_values.insert(i, entry); // todo: is it okay that we possibly overwrite array values from previous input evals?
                        }
                        Ok(Value::array(array_values))
                    }
                },
                Ok(ExpressionEvaluationStatus::CompleteErr(e)) => Err(e),
                Err(e) => Err(e),
                Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                    return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                }
            };

            if let Err(ref diag) = value {
                if diag.is_error() {
                    fatal_error = true;
                }
            }

            results.insert(&input.name, value);
        } else if let Some(_) = input.as_action() {
            let value = if let Some(value) = previously_evaluated_input {
                value.clone()
            } else {
                let Some(expr) = command_instance.get_expression_from_input(&input)? else {
                    continue;
                };
                match eval_expression(
                    &expr,
                    dependencies_execution_results,
                    package_uuid,
                    runbook,
                    runtime_ctx,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => Ok(result),
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => Err(e),
                    Err(e) => Err(e),
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                    }
                }
            };

            if let Err(ref diag) = value {
                if diag.is_error() {
                    fatal_error = true;
                }
            }

            results.insert(&input.name, value);
        } else {
            let value = if let Some(value) = previously_evaluated_input {
                value.clone()
            } else {
                let Some(expr) = command_instance.get_expression_from_input(&input)? else {
                    continue;
                };
                match eval_expression(
                    &expr,
                    dependencies_execution_results,
                    package_uuid,
                    runbook,
                    runtime_ctx,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => Ok(result),
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => Err(e),
                    Err(e) => Err(e),
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                    }
                }
            };

            if let Err(ref diag) = value {
                if diag.is_error() {
                    fatal_error = true;
                }
            }

            results.insert(&input.name, value);
        }
    }

    let status = match fatal_error {
        false => CommandInputEvaluationStatus::Complete(results),
        true => CommandInputEvaluationStatus::Aborted(results),
    };
    Ok(status)
}

pub fn perform_wallet_inputs_evaluation(
    wallet_instance: &WalletInstance,
    dependencies_execution_results: &HashMap<
        ConstructUuid,
        Result<&CommandExecutionResult, &Diagnostic>,
    >,
    input_evaluation_results: &Option<&CommandInputsEvaluationResult>,
    package_uuid: &PackageUuid,
    runbook: &Runbook,
    runtime_ctx: &RuntimeContext,
) -> Result<CommandInputEvaluationStatus, Diagnostic> {
    let mut results = CommandInputsEvaluationResult::new();
    let inputs = wallet_instance.specification.inputs.clone();
    let mut fatal_error = false;

    for input in inputs.into_iter() {
        // todo(micaiah): this value still needs to be for inputs that are objects
        let previously_evaluated_input = match input_evaluation_results {
            Some(input_evaluation_results) => input_evaluation_results.inputs.get(&input.name),
            None => None,
        };
        if let Some(object_props) = input.as_object() {
            // todo(micaiah) - figure out how user-input values work for this branch
            let mut object_values = HashMap::new();
            for prop in object_props.iter() {
                if let Some(value) = previously_evaluated_input {
                    match value.clone() {
                        Ok(Value::Object(obj)) => {
                            for (k, v) in obj.into_iter() {
                                object_values.insert(k, v);
                            }
                        }
                        Ok(v) => {
                            object_values.insert(prop.name.to_string(), Ok(v));
                        }
                        Err(diag) => {
                            object_values.insert(prop.name.to_string(), Err(diag));
                        }
                    };
                }

                let Some(expr) =
                    wallet_instance.get_expression_from_object_property(&input, &prop)?
                else {
                    continue;
                };
                let value = match eval_expression(
                    &expr,
                    dependencies_execution_results,
                    package_uuid,
                    runbook,
                    runtime_ctx,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => Ok(result),
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => Err(e),
                    Err(e) => Err(e),
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                    }
                };

                if let Err(ref diag) = value {
                    if diag.is_error() {
                        fatal_error = true;
                    }
                }

                match value.clone() {
                    Ok(Value::Object(obj)) => {
                        for (k, v) in obj.into_iter() {
                            object_values.insert(k, v);
                        }
                    }
                    Ok(v) => {
                        object_values.insert(prop.name.to_string(), Ok(v));
                    }
                    Err(diag) => {
                        object_values.insert(prop.name.to_string(), Err(diag));
                    }
                };
            }
            if !object_values.is_empty() {
                results.insert(&input.name, Ok(Value::Object(object_values)));
            }
        } else if let Some(_) = input.as_array() {
            let mut array_values = vec![];
            if let Some(value) = previously_evaluated_input {
                match value.clone() {
                    Ok(Value::Array(entries)) => {
                        array_values.extend::<Vec<Value>>(entries.into_iter().collect());
                    }
                    Err(diag) => {
                        results.insert(&input.name, Err(diag));
                        continue;
                    }
                    Ok(Value::Primitive(_)) | Ok(Value::Object(_)) | Ok(Value::Addon(_)) => {
                        unreachable!()
                    }
                }
            }

            let Some(expr) = wallet_instance.get_expression_from_input(&input)? else {
                continue;
            };
            let value = match eval_expression(
                &expr,
                dependencies_execution_results,
                package_uuid,
                runbook,
                runtime_ctx,
            ) {
                Ok(ExpressionEvaluationStatus::CompleteOk(result)) => match result {
                    Value::Primitive(_) | Value::Object(_) | Value::Addon(_) => unreachable!(),
                    Value::Array(entries) => {
                        for (i, entry) in entries.into_iter().enumerate() {
                            array_values.insert(i, entry); // todo: is it okay that we possibly overwrite array values from previous input evals?
                        }
                        Ok(Value::array(array_values))
                    }
                },
                Ok(ExpressionEvaluationStatus::CompleteErr(e)) => Err(e),
                Err(e) => Err(e),
                Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                    return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                }
            };

            if let Err(ref diag) = value {
                if diag.is_error() {
                    fatal_error = true;
                }
            }

            results.insert(&input.name, value);
        } else if let Some(_) = input.as_action() {
            let value = if let Some(value) = previously_evaluated_input {
                value.clone()
            } else {
                let Some(expr) = wallet_instance.get_expression_from_input(&input)? else {
                    continue;
                };
                match eval_expression(
                    &expr,
                    dependencies_execution_results,
                    package_uuid,
                    runbook,
                    runtime_ctx,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => Ok(result),
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => Err(e),
                    Err(e) => Err(e),
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                    }
                }
            };

            if let Err(ref diag) = value {
                if diag.is_error() {
                    fatal_error = true;
                }
            }

            results.insert(&input.name, value);
        } else {
            let value = if let Some(value) = previously_evaluated_input {
                value.clone()
            } else {
                let Some(expr) = wallet_instance.get_expression_from_input(&input)? else {
                    continue;
                };
                match eval_expression(
                    &expr,
                    dependencies_execution_results,
                    package_uuid,
                    runbook,
                    runtime_ctx,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => Ok(result),
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => Err(e),
                    Err(e) => Err(e),
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                    }
                }
            };

            if let Err(ref diag) = value {
                if diag.is_error() {
                    fatal_error = true;
                }
            }

            results.insert(&input.name, value);
        }
    }

    let status = match fatal_error {
        false => CommandInputEvaluationStatus::Complete(results),
        true => CommandInputEvaluationStatus::Aborted(results),
    };
    Ok(status)
}
