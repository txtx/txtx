use std::{
    collections::{HashMap, VecDeque},
    sync::{mpsc::Sender, Arc, RwLock},
};

use crate::{
    types::{Manual, RuntimeContext},
    EvalEvent,
};
use daggy::{Dag, NodeIndex, Walker};
use txtx_addon_kit::{
    hcl::expr::{BinaryOperator, Expression, UnaryOperator},
    types::{
        commands::{
            CommandExecutionResult, CommandExecutionStatus, CommandInputsEvaluationResult,
            CommandInstance, CommandInstanceStateMachineInput, CommandInstanceStateMachineState,
        },
        diagnostics::{Diagnostic, DiagnosticLevel},
        types::Value,
        ConstructUuid, PackageUuid,
    },
    uuid::Uuid,
};

pub fn order_dependent_nodes(start_node: NodeIndex, graph: Dag<Uuid, u32, u32>) -> Vec<NodeIndex> {
    let mut nodes_to_visit = VecDeque::new();
    let mut visited_nodes_to_process = Vec::new();

    nodes_to_visit.push_front(start_node);
    while let Some(node) = nodes_to_visit.pop_front() {
        // Enqueue all the children
        for (_, child) in graph.children(node).iter(&graph) {
            nodes_to_visit.push_back(child);
        }
        // Mark node as visited
        visited_nodes_to_process.push(node);
    }

    visited_nodes_to_process
}

pub fn order_nodes(root_node: NodeIndex, graph: Dag<Uuid, u32, u32>) -> Vec<NodeIndex> {
    let mut nodes_to_visit = VecDeque::new();
    let mut visited_nodes_to_process = Vec::new();

    nodes_to_visit.push_front(root_node);
    while let Some(node) = nodes_to_visit.pop_front() {
        // All the parents must have been visited first
        for (_, parent) in graph.parents(node).iter(&graph) {
            if !visited_nodes_to_process.contains(&parent) {
                nodes_to_visit.push_back(node)
            }
        }
        // Enqueue all the children
        for (_, child) in graph.children(node).iter(&graph) {
            nodes_to_visit.push_back(child);
        }
        // Mark node as visited
        visited_nodes_to_process.push(node);
    }

    visited_nodes_to_process.remove(0); // remove root
    visited_nodes_to_process
}

pub fn log_evaluated_outputs(manual: &Manual) {
    for (_, package) in manual.packages.iter() {
        for construct_uuid in package.outputs_uuids.iter() {
            let construct = manual.commands_instances.get(construct_uuid).unwrap();
            println!("Output '{}'", construct.name);

            if let Some(result) = manual.constructs_execution_results.get(construct_uuid) {
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

/// Prepares for a reevaluation of all of `start_node`'s dependents within the `manual`.
/// This involves setting the command instance state to `New` for all commands _except_
/// the start node. The `New` state indicates to the evaluation loop that the data
/// should be recomputed, and this should occur for all dependents of the updated
/// start node, but not the start node itself.
pub fn prepare_constructs_reevaluation(manual: &Arc<RwLock<Manual>>, start_node: NodeIndex) {
    match manual.read() {
        Ok(manual) => {
            let g = manual.constructs_graph.clone();
            let nodes_to_reevaluate =
                order_dependent_nodes(start_node, manual.constructs_graph.clone());

            for node in nodes_to_reevaluate.into_iter() {
                let uuid = g.node_weight(node).expect("unable to retrieve construct");
                let construct_uuid = ConstructUuid::Local(uuid.clone());

                let Some(command_instance) = manual.commands_instances.get(&construct_uuid) else {
                    continue;
                };
                if node == start_node {
                    continue;
                }

                if let Ok(mut state_machine) = command_instance.state.lock() {
                    match state_machine.state() {
                        CommandInstanceStateMachineState::New
                        | CommandInstanceStateMachineState::Failed => {}
                        _ => {
                            state_machine
                                .consume(&CommandInstanceStateMachineInput::ReEvaluate)
                                .unwrap();
                        }
                    };
                }
            }
        }
        Err(e) => unimplemented!("could not acquire lock: {e}"),
    }
}

pub fn run_constructs_evaluation(
    manual: &Arc<RwLock<Manual>>,
    runtime_ctx: &Arc<RwLock<RuntimeContext>>,
    start_node: Option<NodeIndex>,
    eval_tx: Sender<EvalEvent>,
) -> Result<(), Diagnostic> {
    match manual.write() {
        Ok(mut manual) => {
            let g = manual.constructs_graph.clone();

            let ordered_nodes_to_process = match start_node {
                Some(start_node) => {
                    // if we are walking the graph from a given start node, we only add the
                    // node and its dependents (not its parents) to the nodes we visit.
                    order_dependent_nodes(start_node, manual.constructs_graph.clone())
                }
                None => order_nodes(manual.graph_root, manual.constructs_graph.clone()),
            };

            let commands_instances = manual.commands_instances.clone();
            let constructs_locations = manual.constructs_locations.clone();

            for node in ordered_nodes_to_process.into_iter() {
                let uuid = g.node_weight(node).expect("unable to retrieve construct");
                let construct_uuid = ConstructUuid::Local(uuid.clone());

                let Some(command_instance) = commands_instances.get(&construct_uuid) else {
                    // runtime_ctx.addons.index_command_instance(namespace, package_uuid, block)
                    continue;
                };
                // in general we want to ignore previous input evaluation results when evaluating for outputs.
                // we want to recompute the whole graph in case anything has changed since our last traversal.
                // however, if there was a start_node provided, this evaluation was initiated from a user interaction
                // that is stored in the input evaluation results, and we want to keep that data to evaluate that
                // commands dependents
                let input_evaluation_results = if let Some(start_node) = start_node {
                    if start_node == node {
                        manual
                            .command_inputs_evaluation_results
                            .get(&construct_uuid.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };

                match command_instance.state.lock() {
                    Ok(state_machine) => match state_machine.state() {
                        CommandInstanceStateMachineState::Failed => {
                            println!("continuing past failed state command");
                            continue;
                        }
                        _ => {}
                    },
                    Err(e) => unimplemented!("unable to acquire lock {e}"),
                }
                // in general we want to ignore previous input evaluation results when evaluating for outputs.
                // we want to recompute the whole graph in case anything has changed since our last traversal.
                // however, if there was a start_node provided, this evaluation was initiated from a user interaction
                // that is stored in the input evaluation results, and we want to keep that data to evaluate that
                // commands dependents
                let input_evaluation_results = if let Some(start_node) = start_node {
                    if start_node == node {
                        manual
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
                let (package_uuid, _) = constructs_locations.get(&construct_uuid).unwrap();

                for expr in references_expressions.into_iter() {
                    let res = manual
                        .try_resolve_construct_reference_in_expression(
                            package_uuid,
                            &expr,
                            &runtime_ctx,
                        )
                        .unwrap();
                    if let Some((dependency, _)) = res {
                        let evaluation_result_opt =
                            manual.constructs_execution_results.get(&dependency);

                        if let Some(evaluation_result) = evaluation_result_opt {
                            match cached_dependency_execution_results.get(&dependency) {
                                None => match evaluation_result {
                                    Ok(evaluation_result) => {
                                        cached_dependency_execution_results
                                            .insert(dependency, Ok(evaluation_result));
                                    }
                                    Err(e) => {
                                        cached_dependency_execution_results
                                            .insert(dependency, Err(e));
                                    }
                                },
                                Some(Err(_)) => continue,
                                Some(Ok(_)) => {}
                            }
                        }
                    }
                }

                if let Ok(mut state_machine) = command_instance.state.lock() {
                    match state_machine.state() {
                        CommandInstanceStateMachineState::Evaluated => {
                            println!("current state is evaluated");
                        }
                        CommandInstanceStateMachineState::Failed
                        | CommandInstanceStateMachineState::AwaitingAsyncRequest => {
                            continue;
                        }
                        state => {
                            println!("current state: {:?}", state);

                            let evaluated_inputs_res = perform_inputs_evaluation(
                                command_instance,
                                &cached_dependency_execution_results,
                                &input_evaluation_results,
                                package_uuid,
                                &manual,
                                runtime_ctx,
                            );

                            let evaluated_inputs = match evaluated_inputs_res {
                                Ok(result) => match result {
                                    CommandInputEvaluationStatus::Complete(result) => result,
                                    CommandInputEvaluationStatus::NeedsUserInteraction => {
                                        state_machine
                                            .consume(
                                                &CommandInstanceStateMachineInput::NeedsUserInput,
                                            )
                                            .unwrap();
                                        continue;
                                    }
                                },
                                Err(e) => {
                                    todo!("build input evaluation diagnostic: {}", e)
                                }
                            };

                            manual
                                .command_inputs_evaluation_results
                                .insert(construct_uuid.clone(), evaluated_inputs.clone());

                            let execution_result = match command_instance.perform_execution(
                                &evaluated_inputs,
                                manual.uuid.clone(),
                                construct_uuid.clone(),
                                eval_tx.clone(),
                            ) {
                                // todo(lgalabru): return Diagnostic instead
                                Ok(CommandExecutionStatus::Complete(result)) => {
                                    state_machine
                                        .consume(&CommandInstanceStateMachineInput::Successful)
                                        .unwrap();
                                    result
                                }
                                Ok(CommandExecutionStatus::NeedsAsyncRequest) => {
                                    state_machine
                                        .consume(
                                            &CommandInstanceStateMachineInput::NeedsAsyncRequest,
                                        )
                                        .unwrap();
                                    continue;
                                }
                                Err(e) => {
                                    state_machine
                                        .consume(&CommandInstanceStateMachineInput::Unsuccessful)
                                        .unwrap();
                                    Err(e)
                                }
                            };
                            manual
                                .constructs_execution_results
                                .insert(construct_uuid, execution_result);
                        }
                    }
                }
            }
        }
        Err(e) => unimplemented!("could not acquire lock: {e}"),
    }

    match manual.read() {
        Ok(readonly_manual) => log_evaluated_outputs(&readonly_manual),
        Err(e) => unimplemented!("could not acquire lock: {e}"),
    }

    Ok(())
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
    manual: &Manual,
    runtime_ctx: &Arc<RwLock<RuntimeContext>>,
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
                    manual,
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
        Expression::Object(_object) => {
            unimplemented!()
        }
        // Represents a string containing template interpolations and template directives.
        Expression::StringTemplate(_string_template) => {
            unimplemented!()
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
            let func = function_call.ident.to_string();
            let mut args = vec![];
            for expr in function_call.args.iter() {
                let value = match eval_expression(
                    expr,
                    dependencies_execution_results,
                    package_uuid,
                    manual,
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
            match runtime_ctx.write() {
                Ok(runtime_ctx) => runtime_ctx.execute_function(&func, &args)?,
                Err(e) => unimplemented!("could not acquire lock: {e}"),
            }
        }
        // Represents an attribute or element traversal.
        Expression::Traversal(_) => {
            let Ok(Some((dependency, mut components))) = manual
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
            let attribute = components.pop_front().unwrap();
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
                manual,
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
                manual,
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
                manual,
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
            match &binary_op.operator.value() {
                BinaryOperator::And => unimplemented!(),
                BinaryOperator::Div => unimplemented!(),
                BinaryOperator::Eq => unimplemented!(),
                BinaryOperator::Greater => unimplemented!(),
                BinaryOperator::GreaterEq => unimplemented!(),
                BinaryOperator::Less => unimplemented!(),
                BinaryOperator::LessEq => unimplemented!(),
                BinaryOperator::Minus => unimplemented!(),
                BinaryOperator::Mod => unimplemented!(),
                BinaryOperator::Mul => unimplemented!(),
                BinaryOperator::Plus => match runtime_ctx.write() {
                    Ok(runtime_ctx) => runtime_ctx.execute_function("add_uint", &vec![lhs, rhs])?,
                    Err(e) => unimplemented!("could not acquire lock: {e}"),
                },
                BinaryOperator::NotEq => unimplemented!(),
                BinaryOperator::Or => unimplemented!(),
            }
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
}

pub fn perform_inputs_evaluation(
    command_instance: &CommandInstance,
    dependencies_execution_results: &HashMap<
        ConstructUuid,
        Result<&CommandExecutionResult, &Diagnostic>,
    >,
    input_evaluation_results: &Option<&CommandInputsEvaluationResult>,
    package_uuid: &PackageUuid,
    manual: &Manual,
    runtime_ctx: &Arc<RwLock<RuntimeContext>>,
) -> Result<CommandInputEvaluationStatus, String> {
    let mut results = CommandInputsEvaluationResult::new();
    let inputs = command_instance.specification.inputs.clone();
    let mut _fatal_error = false;

    for input in inputs.into_iter() {
        // todo(micaiah): this value still needs to be for inputs that are objects
        let previously_evaluated_input = match input_evaluation_results {
            Some(input_evaluation_results) => input_evaluation_results.inputs.get(&input),
            None => None,
        };
        if let Some(object_props) = input.as_object() {
            // todo(micaiah) - figure out how user-input values work for this branch
            let mut object_values = HashMap::new();
            for prop in object_props.iter() {
                if let Some(value) = previously_evaluated_input {
                    match value.clone() {
                        Ok(Value::Primitive(p)) => {
                            object_values.insert(prop.name.to_string(), Ok(p));
                        }
                        Ok(Value::Object(obj)) => {
                            for (k, v) in obj.into_iter() {
                                object_values.insert(k, v);
                            }
                        }
                        Ok(Value::Array(_)) => {
                            unreachable!("received array in object") // currently objects can only contain primitives. this probably will need to change
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
                    manual,
                    runtime_ctx,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => Ok(result),
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => Err(e),
                    Err(e) => Err(e),
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        println!("returning early because eval expression needs user interaction");
                        return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                    }
                };

                if let Err(ref diag) = value {
                    if let DiagnosticLevel::Error = diag.level {
                        _fatal_error = true;
                    }
                }

                match value {
                    Ok(Value::Primitive(p)) => {
                        object_values.insert(prop.name.to_string(), Ok(p));
                    }
                    Ok(Value::Object(obj)) => {
                        for (k, v) in obj.into_iter() {
                            object_values.insert(k, v);
                        }
                    }
                    Ok(Value::Array(_)) => {
                        unreachable!("received array in object") // currently objects can only contain primitives. this probably will need to change
                    }
                    Err(diag) => {
                        object_values.insert(prop.name.to_string(), Err(diag));
                    }
                };
            }
            if !object_values.is_empty() {
                results.insert(input, Ok(Value::Object(object_values)));
            }
        } else if let Some(_) = input.as_array() {
            let mut array_values = vec![];
            if let Some(value) = previously_evaluated_input {
                match value.clone() {
                    Ok(Value::Array(entries)) => {
                        array_values.extend::<Vec<Value>>(entries.into_iter().collect());
                    }
                    Err(diag) => {
                        results.insert(input, Err(diag));
                        continue;
                    }
                    Ok(Value::Primitive(_)) | Ok(Value::Object(_)) => unreachable!(),
                }
            }

            let Some(expr) = command_instance.get_expression_from_input(&input)? else {
                continue;
            };
            let value = match eval_expression(
                &expr,
                dependencies_execution_results,
                package_uuid,
                manual,
                runtime_ctx,
            ) {
                Ok(ExpressionEvaluationStatus::CompleteOk(result)) => match result {
                    Value::Primitive(_) | Value::Object(_) => unreachable!(),
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
                    println!("returning early because eval expression needs user interaction");
                    return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                }
            };

            if let Err(ref diag) = value {
                if let DiagnosticLevel::Error = diag.level {
                    _fatal_error = true;
                }
            }

            results.insert(input, value);
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
                    manual,
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
                if let DiagnosticLevel::Error = diag.level {
                    _fatal_error = true;
                }
            }

            results.insert(input, value);
        }
    }

    // if fatal_error {
    //     return Err(format!("fatal error"));
    // }
    Ok(CommandInputEvaluationStatus::Complete(results))
}
