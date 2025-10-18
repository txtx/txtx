use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_CHECK_BALANCE, CHECKED_ADDRESS, IS_BALANCE_CHECKED,
    PROVIDE_PUBLIC_KEY_ACTION_RESULT,
};
use crate::helpers::hcl::visit_optional_untyped_attribute;
use crate::types::stores::ValueStore;
use futures::future;
use hcl_edit::{expr::Expression, structure::Block, Span};
use std::{collections::HashMap, future::Future, pin::Pin};

use super::commands::ConstructInstance;
use super::{
    commands::{
        CommandExecutionResult, CommandInput, CommandInputsEvaluationResult, CommandOutput,
    },
    namespace::Namespace,
    diagnostics::Diagnostic,
    frontend::{
        ActionItemRequest, ActionItemResponse, ActionItemResponseType, Actions, BlockEvent,
    },
    types::{ObjectProperty, RunbookSupervisionContext, Type, Value},
    ConstructDid, PackageId,
};
use super::{AuthorizationContext, Did, EvaluatableInput};

#[derive(Debug, Clone)]
pub struct SignersState {
    pub store: HashMap<ConstructDid, ValueStore>,
}

impl SignersState {
    pub fn new() -> SignersState {
        SignersState { store: HashMap::new() }
    }

    pub fn get_first_signer(&self) -> Option<ValueStore> {
        self.store.values().next().cloned()
    }

    pub fn get_signer_state_mut(&mut self, signer_did: &ConstructDid) -> Option<&mut ValueStore> {
        self.store.get_mut(signer_did)
    }

    pub fn get_signer_state(&self, signer_did: &ConstructDid) -> Option<&ValueStore> {
        self.store.get(signer_did)
    }

    pub fn pop_signer_state(&mut self, signer_did: &ConstructDid) -> Option<ValueStore> {
        self.store.remove(signer_did)
    }

    pub fn push_signer_state(&mut self, signer_state: ValueStore) {
        self.store.insert(ConstructDid(signer_state.uuid.clone()), signer_state);
    }

    // pub fn get_mining_spend_amount<F, G>(
    //     config: &Config,
    //     keychain: &Keychain,
    //     burnchain: &Burnchain,
    //     sortdb: &SortitionDB,
    //     recipients: &[PoxAddress],
    //     start_mine_height: u64,
    //     at_burn_block: Option<u64>,
    //     mut get_prior_winning_prob: F,
    //     mut set_prior_winning_prob: G,
    // ) -> u64
    // where
    //     F: FnMut(u64) -> f64,
    //     G: FnMut(u64, f64),
    // {

    pub fn create_new_signer(&mut self, signer_did: &ConstructDid, signer_name: &str) {
        if !self.store.contains_key(&signer_did) {
            self.store
                .insert(signer_did.clone(), ValueStore::new(signer_name, &signer_did.value()));
        }
    }
}
pub type SignerActionOk = (SignersState, ValueStore, CommandExecutionResult);
pub type SignerActionErr = (SignersState, ValueStore, Diagnostic);
pub type SignerActivateFutureResult = Result<
    Pin<Box<dyn Future<Output = Result<SignerActionOk, SignerActionErr>> + Send>>,
    SignerActionErr,
>;

pub fn consolidate_signer_activate_result(
    res: Result<SignerActionOk, SignerActionErr>,
    block_span: Option<std::ops::Range<usize>>,
) -> Result<(SignersState, CommandExecutionResult), (SignersState, Diagnostic)> {
    match res {
        Ok((mut signers, signer_state, result)) => {
            signers.push_signer_state(signer_state);
            Ok((signers, result))
        }
        Err((mut signers, signer_state, diag)) => {
            signers.push_signer_state(signer_state);
            Err((signers, diag.set_span_range(block_span)))
        }
    }
}
pub async fn consolidate_signer_activate_future_result(
    future: SignerActivateFutureResult,
    block_span: Option<std::ops::Range<usize>>,
) -> Result<
    Result<(SignersState, CommandExecutionResult), (SignersState, Diagnostic)>,
    (SignersState, Diagnostic),
> {
    match future {
        Ok(res) => Ok(consolidate_signer_activate_result(res.await, block_span)),
        Err((mut signers, signer_state, diag)) => {
            signers.push_signer_state(signer_state);
            Err((signers, diag.set_span_range(block_span)))
        }
    }
}

pub type SignerActivateClosure = Box<
    fn(
        &ConstructDid,
        &SignerSpecification,
        &ValueStore,
        ValueStore,
        SignersState,
        &HashMap<ConstructDid, SignerInstance>,
        &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult,
>;

pub type SignerSignFutureResult = Result<
    Pin<Box<dyn Future<Output = Result<SignerActionOk, SignerActionErr>> + Send>>,
    SignerActionErr,
>;

pub type SignerSignClosure = Box<
    fn(
        &ConstructDid,
        &str,
        &Value,
        &SignerSpecification,
        &ValueStore,
        ValueStore,
        SignersState,
        &HashMap<ConstructDid, SignerInstance>,
    ) -> SignerSignFutureResult,
>;

pub type SignerCheckActivabilityClosure = fn(
    &ConstructDid,
    &str,
    &SignerSpecification,
    &ValueStore,
    ValueStore,
    SignersState,
    &HashMap<ConstructDid, SignerInstance>,
    &RunbookSupervisionContext,
    &AuthorizationContext,
    bool,
    bool,
) -> SignerActionsFutureResult;

pub type SignerActionsFutureResult = Result<
    Pin<Box<dyn Future<Output = Result<CheckSignabilityOk, SignerActionErr>> + Send>>,
    SignerActionErr,
>;

pub type PrepareSignedNestedExecutionResult = Result<
    Pin<Box<dyn Future<Output = Result<PrepareNestedExecutionOk, SignerActionErr>> + Send>>,
    SignerActionErr,
>;
pub type PrepareNestedExecutionOk = (SignersState, ValueStore, Vec<(ConstructDid, ValueStore)>);

pub type SignerCheckInstantiabilityClosure =
    fn(&SignerSpecification, Vec<Type>) -> Result<Type, Diagnostic>;

pub type CheckSignabilityOk = (SignersState, ValueStore, Actions);

pub type SignerCheckSignabilityClosure = fn(
    &ConstructDid,
    &str,
    &Option<String>,
    &Option<String>,
    &Option<String>,
    &Value,
    &SignerSpecification,
    &ValueStore,
    ValueStore,
    SignersState,
    &HashMap<ConstructDid, SignerInstance>,
    &RunbookSupervisionContext,
    &AuthorizationContext,
) -> Result<CheckSignabilityOk, SignerActionErr>;

pub type SignerOperationFutureResult = Result<
    Pin<Box<dyn Future<Output = Result<SignerActionOk, SignerActionErr>> + Send>>,
    SignerActionErr,
>;

pub fn return_synchronous<T>(
    res: T,
) -> Result<Pin<Box<dyn Future<Output = Result<T, SignerActionErr>> + Send>>, SignerActionErr>
where
    T: std::marker::Send + 'static,
{
    Ok(Box::pin(future::ready(Ok(res))))
}

pub fn return_synchronous_actions(
    res: Result<CheckSignabilityOk, SignerActionErr>,
) -> SignerActionsFutureResult {
    Ok(Box::pin(future::ready(res)))
}

pub fn return_synchronous_result(
    res: Result<SignerActionOk, SignerActionErr>,
) -> SignerOperationFutureResult {
    Ok(Box::pin(future::ready(res)))
}

pub fn return_synchronous_ok(
    signers: SignersState,
    signer_state: ValueStore,
    res: CommandExecutionResult,
) -> SignerOperationFutureResult {
    return_synchronous_result(Ok((signers, signer_state, res)))
}

pub fn return_synchronous_err(
    signers: SignersState,
    signer_state: ValueStore,
    diag: Diagnostic,
) -> SignerOperationFutureResult {
    return_synchronous_result(Err((signers, signer_state, diag)))
}

pub fn consolidate_signer_result(
    res: Result<CheckSignabilityOk, SignerActionErr>,
    block_span: Option<std::ops::Range<usize>>,
) -> Result<(SignersState, Actions), (SignersState, Diagnostic)> {
    match res {
        Ok((mut signers, signer_state, actions)) => {
            signers.push_signer_state(signer_state);
            Ok((signers, actions))
        }
        Err((mut signers, signer_state, diag)) => {
            signers.push_signer_state(signer_state);
            Err((signers, diag.set_span_range(block_span)))
        }
    }
}
pub async fn consolidate_signer_future_result(
    future: SignerActionsFutureResult,
    block_span: Option<std::ops::Range<usize>>,
) -> Result<(SignersState, Actions), (SignersState, Diagnostic)> {
    match future {
        Ok(res) => match res.await {
            Ok((mut signers, signer_state, actions)) => {
                signers.push_signer_state(signer_state);
                Ok((signers, actions))
            }
            Err((mut signers, signer_state, diag)) => {
                signers.push_signer_state(signer_state);
                Err((signers, diag.set_span_range(block_span)))
            }
        },
        Err((mut signers, signer_state, diag)) => {
            signers.push_signer_state(signer_state);
            Err((signers, diag.set_span_range(block_span)))
        }
    }
}

pub async fn consolidate_nested_execution_result(
    future: PrepareSignedNestedExecutionResult,
    block_span: Option<std::ops::Range<usize>>,
) -> Result<(SignersState, Vec<(ConstructDid, ValueStore)>), (SignersState, Diagnostic)> {
    match future {
        Ok(res) => match res.await {
            Ok((mut signers, signer_state, res)) => {
                signers.push_signer_state(signer_state);
                Ok((signers, res))
            }
            Err((mut signers, signer_state, diag)) => {
                signers.push_signer_state(signer_state);
                Err((signers, diag.set_span_range(block_span)))
            }
        },
        Err((mut signers, signer_state, diag)) => {
            signers.push_signer_state(signer_state);
            Err((signers, diag.set_span_range(block_span)))
        }
    }
}

#[derive(Debug, Clone)]
pub struct SignerSpecification {
    pub name: String,
    pub matcher: String,
    pub documentation: String,
    pub requires_interaction: bool,
    pub example: String,
    pub default_inputs: Vec<CommandInput>,
    pub inputs: Vec<CommandInput>,
    pub outputs: Vec<CommandOutput>,
    pub check_instantiability: SignerCheckInstantiabilityClosure,
    pub check_activability: SignerCheckActivabilityClosure,
    pub activate: SignerActivateClosure,
    pub check_signability: SignerCheckSignabilityClosure,
    pub sign: SignerSignClosure,
    pub force_sequential_signing: bool,
}

#[derive(Debug, Clone)]
pub struct SignerInstance {
    pub specification: SignerSpecification,
    pub name: String,
    pub block: Block,
    pub package_id: PackageId,
    pub namespace: Namespace,
}

impl SignerInstance {
    pub fn compute_fingerprint(&self, evaluated_inputs: &CommandInputsEvaluationResult) -> Did {
        let mut comps = vec![];
        for input in self.specification.inputs.iter() {
            let Some(value) = evaluated_inputs.inputs.get_value(&input.name) else { continue };
            if input.sensitive {
                comps.push(value.to_be_bytes());
            }
        }
        Did::from_components(comps)
    }

    /// Checks the `CommandInstance` HCL Block for an attribute named `input.name`
    pub fn get_expression_from_input(&self, input_name: &str) -> Option<Expression> {
        visit_optional_untyped_attribute(&input_name, &self.block)
    }

    pub fn get_group(&self) -> String {
        let Some(group) = self.block.body.get_attribute("group") else {
            return format!("{} Review", self.specification.name.to_string());
        };
        group.value.to_string()
    }

    pub fn get_expression_from_object_property(
        &self,
        input_name: &str,
        prop: &ObjectProperty,
    ) -> Option<Expression> {
        let object = self.block.body.get_blocks(&input_name).next();
        match object {
            Some(block) => {
                let expr_res = visit_optional_untyped_attribute(&prop.name, &block);
                match expr_res {
                    Some(expression) => Some(expression),
                    None => None,
                }
            }
            None => None,
        }
    }

    pub async fn check_activability(
        &self,
        construct_did: &ConstructDid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        action_item_requests: &Option<&Vec<&mut ActionItemRequest>>,
        action_item_responses: &Option<&Vec<ActionItemResponse>>,
        supervision_context: &RunbookSupervisionContext,
        authorization_context: &AuthorizationContext,
        is_balance_check_required: bool,
        is_public_key_required: bool,
    ) -> Result<(SignersState, Actions), (SignersState, Diagnostic)> {
        let mut values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs)
            .check(&self.name, &self.specification.inputs)
            .map_err(|e| (signers.clone(), e))?;

        match action_item_responses {
            Some(responses) => {
                for ActionItemResponse { payload, action_item_id } in responses.iter() {
                    match payload {
                        ActionItemResponseType::ProvidePublicKey(update) => {
                            values.insert(
                                PROVIDE_PUBLIC_KEY_ACTION_RESULT,
                                Value::string(update.public_key.clone()),
                            );
                        }
                        ActionItemResponseType::ReviewInput(response) => {
                            let request = action_item_requests
                                .map(|requests| requests.iter().find(|r| r.id.eq(&action_item_id)));

                            if let Some(Some(request)) = request {
                                if let Some(signer_did) = &request.construct_did {
                                    let mut signer_state =
                                        signers.pop_signer_state(signer_did).unwrap();
                                    if request.internal_key == ACTION_ITEM_CHECK_ADDRESS {
                                        if response.value_checked {
                                            let data = request
                                                .action_type
                                                .as_review_input()
                                                .expect("review input action item");
                                            signer_state.insert(
                                                CHECKED_ADDRESS,
                                                Value::string(data.value.to_string()),
                                            );
                                        }
                                    } else if request.internal_key == ACTION_ITEM_CHECK_BALANCE {
                                        signer_state.insert(
                                            IS_BALANCE_CHECKED,
                                            Value::bool(response.value_checked),
                                        );
                                    }
                                    signers.push_signer_state(signer_state);
                                }
                            }
                        }

                        _ => {}
                    }
                }
            }
            None => {}
        }

        let signer_state = signers.pop_signer_state(construct_did).unwrap();
        let spec = &self.specification;
        let res = (spec.check_activability)(
            &construct_did,
            &self.name,
            &self.specification,
            &values,
            signer_state,
            signers,
            signers_instances,
            &supervision_context,
            &authorization_context,
            is_balance_check_required,
            is_public_key_required,
        );

        consolidate_signer_future_result(res, self.block.span()).await
    }

    pub async fn perform_activation(
        &self,
        construct_did: &ConstructDid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        progress_tx: &channel::Sender<BlockEvent>,
    ) -> Result<(SignersState, CommandExecutionResult), (SignersState, Diagnostic)> {
        let values = ValueStore::new(&self.name, &construct_did.value())
            .with_defaults(&evaluated_inputs.inputs.defaults)
            .with_inputs(&evaluated_inputs.inputs.inputs);

        let signer_state = signers.pop_signer_state(construct_did).unwrap();
        let future = (&self.specification.activate)(
            &construct_did,
            &self.specification,
            &values,
            signer_state,
            signers,
            signers_instances,
            progress_tx,
        );
        consolidate_signer_activate_future_result(future, self.block.span()).await?
    }
}

impl ConstructInstance for SignerInstance {
    fn block(&self) -> &Block {
        &self.block
    }
    fn inputs(&self) -> Vec<Box<dyn EvaluatableInput>> {
        self.specification
            .inputs
            .iter()
            .chain(&self.specification.default_inputs)
            .map(|input| Box::new(input.clone()) as Box<dyn EvaluatableInput>)
            .collect()
    }
}

pub trait SignerImplementation {
    fn check_instantiability(
        _ctx: &SignerSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic>;

    fn check_activability(
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &SignerSpecification,
        _values: &ValueStore,
        _signer_state: ValueStore,
        _signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _supervision_context: &RunbookSupervisionContext,
        _authorization_context: &AuthorizationContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        unimplemented!()
    }

    fn activate(
        _construct_id: &ConstructDid,
        _spec: &SignerSpecification,
        _values: &ValueStore,
        _signer_state: ValueStore,
        _signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        unimplemented!()
    }

    fn check_signability(
        _caller_uuid: &ConstructDid,
        _title: &str,
        _description: &Option<String>,
        _meta_description: &Option<String>,
        _markdown: &Option<String>,
        _payload: &Value,
        _spec: &SignerSpecification,
        _values: &ValueStore,
        _signer_state: ValueStore,
        _signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _supervision_context: &RunbookSupervisionContext,
        _auth_ctx: &AuthorizationContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        unimplemented!()
    }

    fn sign(
        _caller_uuid: &ConstructDid,
        _title: &str,
        _payload: &Value,
        _spec: &SignerSpecification,
        _values: &ValueStore,
        _signer_state: ValueStore,
        _signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> SignerSignFutureResult {
        unimplemented!()
    }
}
