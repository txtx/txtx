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
    manual: &Manual,
    runtime_ctx: &RuntimeContext,
) -> Result<(), String> {
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

    let mut constructs_execution_results: HashMap<ConstructUuid, CommandExecutionResult> =
        HashMap::new();
    for node in visited_nodes_to_process.into_iter() {
        let uuid = g.node_weight(node).expect("unable to retrieve construct");
        let construct_uuid = ConstructUuid::Local(uuid.clone());
        let command_instance = manual
            .commands_instances
            .get(&construct_uuid)
            .expect("unable to retrieve construct");

        let mut dependencies_execution_results: HashMap<ConstructUuid, &CommandExecutionResult> =
            HashMap::new();

        // Retrieve the construct_uuid of the inputs
        // Collect the outputs
        let references_expressions = command_instance
            .get_references_expressions_from_inputs()
            .unwrap();
        let (package_uuid, _) = manual.constructs_locations.get(&construct_uuid).unwrap();
        for expr in references_expressions.into_iter() {
            let res = manual
                .try_resolve_construct_reference_in_expression(package_uuid, &expr)
                .unwrap();
            if let Some((dependency, _)) = res {
                let evaluation_result_opt = constructs_execution_results.get(&dependency);
                if let Some(evaluation_result) = evaluation_result_opt {
                    dependencies_execution_results.insert(dependency, evaluation_result);
                }
            }
        }

        let evaluated_inputs = perform_inputs_evaluation(
            command_instance,
            &dependencies_execution_results,
            package_uuid,
            manual,
            runtime_ctx,
        )
        .unwrap(); // todo(lgalabru): return Diagnostic instead
        let execution_result = command_instance
            .perform_execution(&evaluated_inputs)
            .unwrap(); // todo(lgalabru): return Diagnostic instead
        constructs_execution_results.insert(construct_uuid, execution_result);
    }

    for (_, package) in manual.packages.iter() {
        for construct_uuid in package.outputs_uuids.iter() {
            let construct = manual.commands_instances.get(construct_uuid).unwrap();
            println!("Output '{}'", construct.name);

            for (key, value) in constructs_execution_results
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
        Expression::Null(_decorated_null) => Value::Null,
        // Represents a boolean.
        Expression::Bool(decorated_bool) => Value::Bool(*decorated_bool.value()),
        // Represents a number, either integer or float.
        Expression::Number(formattted_number) => {
            match (
                formattted_number.value().as_u64(),
                formattted_number.value().as_i64(),
                formattted_number.value().as_f64(),
            ) {
                (Some(value), _, _) => Value::UnsignedInteger(value),
                (_, Some(value), _) => Value::SignedInteger(value),
                (_, _, Some(value)) => Value::Float(value),
                (None, None, None) => unreachable!(), // todo(lgalabru): return Diagnostic
            }
        }
        // Represents a string that does not contain any template interpolations or template directives.
        Expression::String(decorated_string) => Value::String(decorated_string.to_string()),
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
        // Represents conditional operator which selects one of two rexpressions based on the outcome of a boolean expression.
        Expression::Conditional(_conditional) => {
            unimplemented!()
        }
        // Represents a function call.
        Expression::FuncCall(_function_call) => {
            unimplemented!()
        }
        // Represents an attribute or element traversal.
        Expression::Traversal(_) => {
            let Ok(Some((dependency, mut components))) =
                manual.try_resolve_construct_reference_in_expression(package_uuid, expr)
            else {
                unimplemented!()
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
            if !is_type_eq(&lhs, &rhs) {
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

pub fn is_type_eq(lhs: &Value, rhs: &Value) -> bool {
    match (lhs, rhs) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(_), Value::Bool(_)) => true,
        (Value::UnsignedInteger(_), Value::UnsignedInteger(_)) => true,
        (Value::SignedInteger(_), Value::SignedInteger(_)) => true,
        (Value::Float(_), Value::Float(_)) => true,
        (Value::String(_), Value::String(_)) => true,
        (Value::Buffer(_), Value::Buffer(_)) => true,
        (Value::Null, _) => false,
        (Value::Bool(_), _) => false,
        (Value::UnsignedInteger(_), _) => false,
        (Value::SignedInteger(_), _) => false,
        (Value::Float(_), _) => false,
        (Value::String(_), _) => false,
        (Value::Buffer(_), _) => false,
    }
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
        let expr = command_instance.get_expressions_from_input(&input)?;
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

    if fatal_error {
        return Err(format!("fatal error"));
    }
    Ok(results)
}
