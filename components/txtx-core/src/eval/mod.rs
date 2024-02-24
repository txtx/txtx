use std::collections::{BTreeSet, HashMap, VecDeque};

use crate::types::{Manual, RuntimeContext};
use daggy::Walker;
use txtx_addon_kit::{
    hcl::expr::{BinaryOperator, Expression, UnaryOperator},
    types::{
        commands::{CommandExecutionResult, CommandInputsEvaluationResult, CommandInstance},
        diagnostics::{Diagnostic, DiagnosticLevel},
        types::Value,
        ConstructUuid, PackageUuid,
    },
};

pub fn run_constructs_evaluation(
    manual: &mut Manual,
    runtime_ctx: &RuntimeContext,
) -> Result<(), Diagnostic> {
    let root = manual.graph_root;
    let g = &manual.constructs_graph;

    let mut nodes_to_visit = VecDeque::new();
    let mut visited_nodes_to_process = BTreeSet::new();

    nodes_to_visit.push_front(root);
    while let Some(node) = nodes_to_visit.pop_front() {
        // All the parents must have been visited first
        for (_, parent) in g.parents(node).iter(&g) {
            if !visited_nodes_to_process.contains(&parent) {
                nodes_to_visit.push_back(node)
            }
        }
        // Enqueue all the children
        for (_, child) in g.children(node).iter(&g) {
            nodes_to_visit.push_back(child);
        }
        // Mark node as visited
        visited_nodes_to_process.insert(node);
    }

    visited_nodes_to_process.remove(&root);

    for node in visited_nodes_to_process.into_iter() {
        let uuid = g.node_weight(node).expect("unable to retrieve construct");
        let construct_uuid = ConstructUuid::Local(uuid.clone());

        let Some(command_instance) = manual.get_command_instance(&construct_uuid) else {
            // runtime_ctx.addons.index_command_instance(namespace, package_uuid, block)
            continue;
        };

        let mut dependencies_execution_results: HashMap<ConstructUuid, &CommandExecutionResult> =
            HashMap::new();

        // Retrieve the construct_uuid of the inputs
        // Collect the outputs
        let references_expressions: Vec<Expression> = command_instance
            .get_expressions_referencing_commands_from_inputs()
            .unwrap();
        let (package_uuid, _) = manual.constructs_locations.get(&construct_uuid).unwrap();
        for expr in references_expressions.into_iter() {
            let res = manual
                .try_resolve_construct_reference_in_expression(package_uuid, &expr, &runtime_ctx)
                .unwrap();
            if let Some((dependency, _)) = res {
                let evaluation_result_opt = manual.constructs_execution_results.get(&dependency);
                if let Some(evaluation_result) = evaluation_result_opt {
                    dependencies_execution_results.insert(dependency, evaluation_result);
                }
            }
        }

        let evaluated_inputs_res = perform_inputs_evaluation(
            command_instance,
            &dependencies_execution_results,
            package_uuid,
            manual,
            runtime_ctx,
        );
        let evaluated_inputs = match evaluated_inputs_res {
            Ok(evaluated_inputs) => evaluated_inputs,
            Err(e) => {
                todo!("build input evaluation diagnostic: {}", e)
            }
        };

        let execution_result = command_instance
            .perform_execution(&evaluated_inputs)
            .unwrap(); // todo(lgalabru): return Diagnostic instead

        manual
            .command_inputs_evaluation_results
            .insert(construct_uuid.clone(), evaluated_inputs.clone());

        manual
            .constructs_execution_results
            .insert(construct_uuid, execution_result);
    }

    for (_, package) in manual.packages.iter() {
        for construct_uuid in package.outputs_uuids.iter() {
            let construct = manual.commands_instances.get(construct_uuid).unwrap();
            println!("Output '{}'", construct.name);

            for (key, value) in manual
                .constructs_execution_results
                .get(construct_uuid)
                .unwrap()
                .outputs
                .iter()
            {
                println!("- {}: {:?}", key, value);
            }
        }
    }

    Ok(())
}

pub fn eval_expression(
    expr: &Expression,
    dependencies_execution_results: &HashMap<ConstructUuid, &CommandExecutionResult>,
    package_uuid: &PackageUuid,
    manual: &Manual,
    runtime_ctx: &RuntimeContext,
) -> Result<Value, Diagnostic> {
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
        Expression::Array(_array) => {
            unimplemented!()
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
                let value = eval_expression(
                    expr,
                    dependencies_execution_results,
                    package_uuid,
                    manual,
                    runtime_ctx,
                )?;
                args.push(value);
            }
            runtime_ctx.execute_function(&func, &args)?
        }
        // Represents an attribute or element traversal.
        Expression::Traversal(_) => {
            let Ok(Some((dependency, mut components))) = manual
                .try_resolve_construct_reference_in_expression(package_uuid, expr, &runtime_ctx)
            else {
                todo!("implement diagnostic for unresolvable references")
            };
            let res = dependencies_execution_results.get(&dependency).unwrap();
            let attribute = components.pop_front().unwrap();
            res.outputs.get(&attribute).unwrap().clone()
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
            let lhs = eval_expression(
                &binary_op.lhs_expr,
                dependencies_execution_results,
                package_uuid,
                manual,
                runtime_ctx,
            )?;
            let rhs = eval_expression(
                &binary_op.rhs_expr,
                dependencies_execution_results,
                package_uuid,
                manual,
                runtime_ctx,
            )?;
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
                BinaryOperator::Plus => {
                    runtime_ctx.execute_function("add_uint", &vec![lhs, rhs])?
                }
                BinaryOperator::NotEq => unimplemented!(),
                BinaryOperator::Or => unimplemented!(),
            }
        }
        // Represents a construct for constructing a collection by projecting the items from another collection.
        Expression::ForExpr(_for_expr) => {
            unimplemented!()
        }
    };

    Ok(value)
}

// pub struct EvaluatedExpression {
//     value: Value,
// }

pub fn perform_inputs_evaluation(
    command_instance: &CommandInstance,
    dependencies_execution_results: &HashMap<ConstructUuid, &CommandExecutionResult>,
    package_uuid: &PackageUuid,
    manual: &Manual,
    runtime_ctx: &RuntimeContext,
) -> Result<CommandInputsEvaluationResult, String> {
    let mut results = CommandInputsEvaluationResult::new();
    let inputs = command_instance.specification.inputs.clone();
    let mut fatal_error = false;

    for input in inputs.into_iter() {
        match input.as_object() {
            Some(object_props) => {
                let mut object_values = HashMap::new();
                for prop in object_props.iter() {
                    let Some(expr) =
                        command_instance.get_expression_from_object_property(&input, &prop)?
                    else {
                        continue;
                    };
                    let value = eval_expression(
                        &expr,
                        dependencies_execution_results,
                        package_uuid,
                        manual,
                        runtime_ctx,
                    );
                    if let Err(ref diag) = value {
                        if let DiagnosticLevel::Error = diag.level {
                            fatal_error = true;
                        }
                    }

                    let res = match value {
                        Ok(Value::Primitive(p)) => Ok(p),
                        Ok(_) => unreachable!(),
                        Err(diag) => Err(diag),
                    };

                    object_values.insert(prop.name.to_string(), res);
                }
                results.insert(input, Ok(Value::Object(object_values)));
            }
            None => {
                let Some(expr) = command_instance.get_expression_from_input(&input)? else {
                    continue;
                };
                let value = eval_expression(
                    &expr,
                    dependencies_execution_results,
                    package_uuid,
                    manual,
                    runtime_ctx,
                );
                if let Err(ref diag) = value {
                    if let DiagnosticLevel::Error = diag.level {
                        fatal_error = true;
                    }
                }
                results.insert(input, value);
            }
        }
    }

    if fatal_error {
        return Err(format!("fatal error"));
    }
    Ok(results)
}
