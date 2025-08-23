use crate::{
    indoc,
    types::{frontend::LogDispatcher, types::ObjectProperty},
};
use uuid::Uuid;

use super::{AssertionResult, CommandSpecification, ASSERTION, BEHAVIOR, PRE_CONDITION};
use crate::types::{
    diagnostics::Diagnostic,
    frontend::BlockEvent,
    stores::ValueStore,
    types::{Type, Value},
    ConstructDid, EvaluatableInput,
};

lazy_static! {
    pub static ref PRE_CONDITION_TYPE: Type = Type::strict_map(
        vec![
            ObjectProperty {
                name: BEHAVIOR.into(),
                documentation: indoc! {r#"
                        The behavior if the pre-condition assertion does not pass. Possible values are:
                            - **halt** (default): Throws an error and halts execution of the runbook
                            - **log**: Logs a warning and continues execution of the runbook
                            - **skip**: Skips execution of this command and all downstream commands
                    "#}
                .into(),
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false,
            },
            ObjectProperty {
                name: ASSERTION.into(),
                documentation: "The assertion to check to determine if the pre-condition behavior should be executed. This value should evaluate to a boolean, or the `std::assert_eq` and other assertions from the standard library can be used."
                .into(),
                typing: Type::bool(),
                optional: false,
                tainting: false,
                internal: false,
            }
        ]
    );
}

#[derive(Clone)]
pub struct PreConditionEvaluatableInput;
impl PreConditionEvaluatableInput {
    pub fn new() -> impl EvaluatableInput {
        Self {}
    }
}
impl EvaluatableInput for PreConditionEvaluatableInput {
    fn documentation(&self) -> String {
        "Pre-conditions are assertions that are evaluated before a command is executed. They can be used to determine if the command should be executed or if a specific behavior should be executed based on the result of the assertion.".into()
    }

    fn optional(&self) -> bool {
        true
    }

    fn typing(&self) -> &Type {
        &PRE_CONDITION_TYPE
    }

    fn name(&self) -> String {
        "pre_condition".into()
    }
}

#[derive(Debug, Clone)]
pub enum PreConditionEvaluationResult {
    Noop,
    Halt(Vec<Diagnostic>),
    SkipDownstream,
}

pub fn evaluate_pre_conditions(
    _construct_did: &ConstructDid,
    instance_name: &str,
    spec: &CommandSpecification,
    values: &ValueStore,
    progress_tx: &channel::Sender<BlockEvent>,
    background_tasks_uuid: &Uuid,
) -> Result<PreConditionEvaluationResult, Diagnostic> {
    let Some(pre_conditions) = values.get_map(PRE_CONDITION) else {
        return Ok(PreConditionEvaluationResult::Noop);
    };

    let pre_conditions = PreCondition::from_map(pre_conditions)?;

    let mut diags = vec![];
    let mut do_skip = false;

    let logger = LogDispatcher::new(*background_tasks_uuid, "svm::pre_conditions", progress_tx);
    for (i, pre_condition) in pre_conditions.iter().enumerate() {
        if let AssertionResult::Failure(assertion_msg) = &pre_condition.assertion {
            match pre_condition.behavior {
                PreConditionBehavior::Halt => {
                    // Additional context is already added to diagnostics, so no need to include here
                    diags.push(Diagnostic::error_from_string(format!(
                        "pre_condition #{}: {}",
                        i + 1,
                        assertion_msg
                    )));
                }
                PreConditionBehavior::Log => {
                    logger.warn(
                        "Pre-condition failed",
                        format!(
                            "'{}' command '{}': pre_condition #{}: {}",
                            spec.matcher,
                            instance_name,
                            i + 1,
                            assertion_msg
                        ),
                    );
                }
                PreConditionBehavior::Skip => {
                    do_skip = true;
                    logger.warn(
                        "Pre-condition failed",
                        format!(
                        "'{}' command '{}': pre_condition #{}: {}: skipping execution of this command and all downstream commands",
                        spec.matcher,
                        instance_name,
                        i + 1,
                        assertion_msg
                    ));
                }
            }
        }
    }
    if !diags.is_empty() {
        return Ok(PreConditionEvaluationResult::Halt(diags));
    }

    if do_skip {
        return Ok(PreConditionEvaluationResult::SkipDownstream);
    }

    Ok(PreConditionEvaluationResult::Noop)
}

#[derive(Debug, Clone)]
pub struct PreCondition {
    pub behavior: PreConditionBehavior,
    pub assertion: AssertionResult,
}

impl PreCondition {
    const ERROR_PREFIX: &str = "error evaluating pre conditions";

    pub fn from_map(pre_condition_map_entries: &Vec<Value>) -> Result<Vec<Self>, Diagnostic> {
        let mut results = Vec::with_capacity(pre_condition_map_entries.len());

        for (i, pre_condition_entry) in pre_condition_map_entries.iter().enumerate() {
            let err_prefix = format!("{}: error in pre_condition #{}", Self::ERROR_PREFIX, i + 1);

            let pre_condition_values = pre_condition_entry.as_object().ok_or_else(|| {
                Diagnostic::error_from_string(format!("{err_prefix}: not a valid map type",))
            })?;

            let behavior = pre_condition_values
                .get(BEHAVIOR)
                .map(|v| {
                    v.as_string()
                        .ok_or(Diagnostic::error_from_string(format!(
                            "{err_prefix}: behavior field must be a string",
                        )))
                        .and_then(|s| {
                            PreConditionBehavior::from_str(s).map_err(|e| {
                                Diagnostic::error_from_string(format!("{err_prefix}: {e}",))
                            })
                        })
                })
                .transpose()?
                .unwrap_or_default();

            let assertion = pre_condition_values
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

            results.push(Self { behavior, assertion });
        }

        Ok(results)
    }
}

#[derive(Debug, Clone)]
pub enum PreConditionBehavior {
    Halt,
    Log,
    Skip,
}

impl Default for PreConditionBehavior {
    fn default() -> Self {
        Self::Halt
    }
}
impl PreConditionBehavior {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "halt" => Ok(PreConditionBehavior::Halt),
            "log" => Ok(PreConditionBehavior::Log),
            "skip" => Ok(PreConditionBehavior::Skip),
            _ => Err(format!(
                "invalid behavior '{}'; valid options are 'halt', 'log', and 'skip'",
                s
            )),
        }
    }
}
