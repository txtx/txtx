use crate::{Context, ContextData};
use juniper_codegen::graphql_object;
use serde_json::json;
use txtx_core::eval::run_constructs_evaluation;
use txtx_core::kit::types::types::{PrimitiveValue, Value};
use txtx_core::types::ConstructUuid;
use uuid::Uuid;

pub struct Mutation;

#[graphql_object(
    context = Context,
)]
impl Mutation {
    fn api_version() -> &'static str {
        "1.0"
    }

    fn update_command_input<'ctx>(
        context: &'ctx Context,
        manual_name: String,
        command_uuid: Uuid,
        input_name: String,
        value: String,
    ) -> Result<String, String> {
        println!("mutation!!! value: {}", value);
        let ContextData {
            manual,
            runtime_context,
        } = context
            .data
            .get(&manual_name)
            .ok_or(format!("could not fine manual {manual_name}"))?;

        let command_uuid = ConstructUuid::Local(command_uuid);
        let manual_did_mutate = match manual.write() {
            Ok(mut manual) => match manual.commands_instances.get_mut(&command_uuid) {
                Some(command_instance) => {
                    match command_instance
                        .specification
                        .inputs
                        .iter()
                        .find(|i| i.name == input_name)
                    {
                        Some(input) => {
                            command_instance.input_evaluation_result.insert(
                                input.clone(),
                                Value::Primitive(PrimitiveValue::UnsignedInteger(
                                    value.parse().unwrap(),
                                )),
                            );
                            true
                        }
                        None => false,
                    }
                }
                None => false,
            },
            Err(e) => unimplemented!("could not acquire lock: {e}"),
        };
        if manual_did_mutate {
            // todo: optimization: rather than rerunning the whole eval, we can
            // walk the graph to see what other commands are impacted by updating _this_ input,
            // and only reevaluate those
            match run_constructs_evaluation(&manual, runtime_context) {
                Ok(()) => println!("successfully reevaluated constructs after mutation"),
                Err(e) => println!("error reevaluating constructs after mutation: {:?}", e),
            }
        }
        let result = match manual.read() {
            Ok(manual) => {
                let mut result = vec![];
                for (construct_uuid, command_instance) in manual.commands_instances.iter() {
                    let constructs_execution_results =
                        manual.constructs_execution_results.get(&construct_uuid);
                    let command_inputs_evaluation_results = manual
                        .command_inputs_evaluation_results
                        .get(&construct_uuid);
                    result.push(json!({
                        "constructUuid": construct_uuid,
                        "commandInstance": command_instance,
                        "commandInputsEvaluationResult": command_inputs_evaluation_results,
                        "constructsExecutionResult": constructs_execution_results
                    }));
                }
                result
            }
            Err(e) => unimplemented!("could not acquire lock: {e}"),
        };

        serde_json::to_string(&result).map_err(|e| format!("failed to serialize manual data {e}"))
    }
}
