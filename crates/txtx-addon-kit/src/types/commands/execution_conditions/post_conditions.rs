use crate::{
    constants::RE_EXECUTE_COMMAND,
    indoc,
    types::{
        commands::CommandExecutionResult,
        frontend::{Block, Panel, StatusUpdater},
        types::ObjectProperty,
    },
};
use uuid::Uuid;

use super::{
    AssertionResult, CommandSpecification, ASSERTION, BACKOFF, BEHAVIOR, POST_CONDITION,
    POST_CONDITION_ATTEMPTS, RETRIES,
};
use crate::types::{
    diagnostics::Diagnostic,
    frontend::BlockEvent,
    stores::ValueStore,
    types::{Type, Value},
    ConstructDid, EvaluatableInput,
};

lazy_static! {
    pub static ref POST_CONDITION_TYPE: Type = Type::strict_map(vec![
        ObjectProperty {
            name: "retries".into(),
            documentation: indoc! {r#"
                If the post-condition assertion fails, the number of times to re-execute the command before executing the post-condition behavior. The default is 0.
            "#}
            .into(),
            typing: Type::integer(),
            optional: true,
            tainting: false,
            internal: false,
        },
        ObjectProperty {
            name: "backoff".into(),
            documentation: indoc! {r#"
                If the post-condition assertion fails, the number of milliseconds to wait before re-executing the command.
                If not specified, the default is 1000 milliseconds (1 second).
            "#}
            .into(),
            typing: Type::integer(),
            optional: true,
            tainting: false,
            internal: false,
        },
        ObjectProperty {
            name: "behavior".into(),
            documentation: indoc! {r#"
                The behavior if the post-condition assertion does not pass. Possible values are:
                - "halt": Throws an error and halts execution of the runbook
                - "log": Logs a warning and continues execution of the runbook
                - "skip": Skips execution of this command and all downstream commands
                - "continue": Continues execution without any action
                If not specified, the default is "halt".
            "#}
            .into(),
            typing: Type::string(),
            optional: true,
            tainting: false,
            internal: false,
        },
        ObjectProperty {
            name: "assertion".into(),
            documentation: "The assertion to check to determine if the command should be re-executed or if the post-condition behavior should be executed."
                .into(),
            typing: Type::bool(),
            optional: false,
            tainting: false,
            internal: false,
        }
    ]);
}

#[derive(Clone)]
pub struct PostConditionEvaluatableInput;
impl PostConditionEvaluatableInput {
    pub fn new() -> impl EvaluatableInput {
        Self {}
    }
}
impl EvaluatableInput for PostConditionEvaluatableInput {
    fn optional(&self) -> bool {
        true
    }

    fn typing(&self) -> &Type {
        &POST_CONDITION_TYPE
    }

    fn name(&self) -> String {
        "post_condition".into()
    }
}

#[derive(Debug, Clone)]
pub enum PostConditionEvaluationResult {
    Noop,
    SkipDownstream,
    Halt(Vec<Diagnostic>),
    Retry(u16),
}

pub fn evaluate_post_conditions(
    construct_did: &ConstructDid,
    instance_name: &str,
    spec: &CommandSpecification,
    values: &ValueStore,
    execution_results: &mut CommandExecutionResult,
    progress_tx: &channel::Sender<BlockEvent>,
    background_tasks_uuid: &Uuid,
) -> Result<PostConditionEvaluationResult, Diagnostic> {
    let Some(post_conditions) = values.get_map(POST_CONDITION) else {
        return Ok(PostConditionEvaluationResult::Noop);
    };

    let post_conditions = PostCondition::from_map(post_conditions)?;

    let mut diags = vec![];
    let mut do_skip = false;

    let _ = progress_tx.send(BlockEvent::ProgressBar(Block::new(
        &background_tasks_uuid,
        Panel::ProgressBar(vec![]),
    )));

    let mut status_updater = StatusUpdater::new(background_tasks_uuid, construct_did, progress_tx);
    for (i, post_condition) in post_conditions.iter().enumerate() {
        if let AssertionResult::Failure(assertion_msg) = &post_condition.assertion {
            if post_condition.retries > 0 {
                let attempts = execution_results
                    .outputs
                    .get(POST_CONDITION_ATTEMPTS)
                    .map(|v| v.as_integer().unwrap())
                    .unwrap_or(0);

                if attempts < post_condition.retries as i128 {
                    // If the assertion is false, we will retry the command
                    status_updater.propagate_warning_status(&format!(
                        "'{}' command '{}': post_condition #{} attempt {}/{}: retrying execution of command in {} milliseconds",
                        spec.matcher,
                        instance_name,
                        i + 1,
                        attempts + 1,
                        post_condition.retries,
                        post_condition.backoff
                    ));

                    execution_results
                        .outputs
                        .insert(POST_CONDITION_ATTEMPTS.into(), Value::Integer(attempts + 1));
                    execution_results.outputs.insert(RE_EXECUTE_COMMAND.into(), Value::bool(true));
                    return Ok(PostConditionEvaluationResult::Retry(post_condition.backoff));
                } else {
                    execution_results.outputs.entry(RE_EXECUTE_COMMAND.into()).and_modify(|v| {
                        *v = Value::bool(false);
                    });
                }
            }

            match post_condition.behavior {
                PostConditionBehavior::Halt => {
                    // Additional context is already added to diagnostics, so no need to include here
                    diags.push(Diagnostic::error_from_string(format!(
                        "post_condition #{}: {}",
                        i + 1,
                        assertion_msg
                    )));
                }
                PostConditionBehavior::Log => {
                    status_updater.propagate_warning_status(&format!(
                        "'{}' command '{}': post_condition #{}: {}",
                        spec.matcher,
                        instance_name,
                        i + 1,
                        assertion_msg
                    ));
                }
                PostConditionBehavior::Skip => {
                    do_skip = true;
                    status_updater.propagate_warning_status(&format!(
                        "'{}' command '{}': post_condition #{}: {}: skipping execution of this command and all downstream commands",
                        spec.matcher,
                        instance_name,
                        i + 1,
                        assertion_msg
                    ));
                }
                PostConditionBehavior::Continue => {}
            }
        } else {
            execution_results.outputs.entry(RE_EXECUTE_COMMAND.into()).and_modify(|v| {
                *v = Value::bool(false);
            });
        }
    }
    if !diags.is_empty() {
        return Ok(PostConditionEvaluationResult::Halt(diags));
    }

    if do_skip {
        return Ok(PostConditionEvaluationResult::SkipDownstream);
    }

    Ok(PostConditionEvaluationResult::Noop)
}

#[derive(Debug, Clone)]
pub struct PostCondition {
    pub behavior: PostConditionBehavior,
    pub assertion: AssertionResult,
    pub retries: u8,
    pub backoff: u16,
}

impl PostCondition {
    const ERROR_PREFIX: &str = "error evaluating post-conditions";
    pub fn from_map(post_condition_map_entries: &Vec<Value>) -> Result<Vec<Self>, Diagnostic> {
        let mut results = Vec::with_capacity(post_condition_map_entries.len());

        for (i, post_condition_entry) in post_condition_map_entries.iter().enumerate() {
            let err_prefix = format!("{}: error in post_condition #{}", Self::ERROR_PREFIX, i + 1);

            let post_condition_values = post_condition_entry.as_object().ok_or_else(|| {
                Diagnostic::error_from_string(format!("{err_prefix}: not a valid map type",))
            })?;

            let behavior = post_condition_values
                .get(BEHAVIOR)
                .map(|v| {
                    v.as_string()
                        .ok_or(Diagnostic::error_from_string(format!(
                            "{err_prefix}: behavior field must be a string",
                        )))
                        .and_then(|s| {
                            PostConditionBehavior::from_str(s).map_err(|e| {
                                Diagnostic::error_from_string(format!("{err_prefix}: {e}",))
                            })
                        })
                })
                .transpose()?
                .unwrap_or_default();

            let assertion = post_condition_values
                .get(ASSERTION)
                .map(|v| AssertionResult::from_value(v))
                .ok_or(Diagnostic::error_from_string(format!(
                    "{err_prefix}: missing required 'assertion' field"
                )))?
                .map_err(|e| {
                    Diagnostic::error_from_string(format!(
                        "{err_prefix}: invalid 'assertion' value: {e}"
                    ))
                })?;

            let retries = post_condition_values
                .get(RETRIES)
                .map(|v| {
                    v.as_u8().ok_or(Diagnostic::error_from_string(format!(
                        "{err_prefix}: 'retries' field must be an unsigned 8-bit integer"
                    )))
                })
                .transpose()?
                .transpose()?
                .unwrap_or(0 as u8);

            let backoff = post_condition_values
                .get(BACKOFF)
                .map(|v| {
                    v.as_u16().ok_or(Diagnostic::error_from_string(format!(
                        "{err_prefix}: 'backoff' field must be an unsigned 16-bit integer"
                    )))
                })
                .transpose()?
                .transpose()?
                .unwrap_or(1000 as u16);

            results.push(Self { behavior, assertion, retries, backoff });
        }

        Ok(results)
    }
}

#[derive(Debug, Clone)]
pub enum PostConditionBehavior {
    Halt,
    Log,
    Skip,
    Continue,
}

impl Default for PostConditionBehavior {
    fn default() -> Self {
        Self::Halt
    }
}
impl PostConditionBehavior {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "halt" => Ok(PostConditionBehavior::Halt),
            "log" => Ok(PostConditionBehavior::Log),
            "skip" => Ok(PostConditionBehavior::Skip),
            "continue" => Ok(PostConditionBehavior::Continue),
            _ => Err(format!(
                "invalid behavior '{}'; valid options are 'halt', 'log', 'skip', and 'continue'",
                s
            )),
        }
    }
}
