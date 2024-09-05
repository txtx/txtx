use crate::{
    helpers::hcl::{
        collect_constructs_references_from_expression, visit_optional_untyped_attribute,
    },
    AddonDefaults,
};
use futures::future;
use hcl_edit::{expr::Expression, structure::Block, Span};
use std::{collections::HashMap, future::Future, pin::Pin};

use super::{
    commands::{
        CommandExecutionResult, CommandInput, CommandInputsEvaluationResult, CommandOutput,
    },
    diagnostics::Diagnostic,
    frontend::{
        ActionItemRequest, ActionItemResponse, ActionItemResponseType, Actions, BlockEvent,
    },
    types::{ObjectProperty, RunbookSupervisionContext, Type, Value},
    ConstructDid, Did, PackageId, ValueStore,
};

#[derive(Debug, Clone)]
pub struct SignersState {
    pub store: HashMap<ConstructDid, ValueStore>,
}

impl SignersState {
    pub fn new() -> SignersState {
        SignersState { store: HashMap::new() }
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
) -> Result<(SignersState, CommandExecutionResult), (SignersState, Diagnostic)> {
    match res {
        Ok((mut signers, signer_state, result)) => {
            signers.push_signer_state(signer_state);
            Ok((signers, result))
        }
        Err((mut signers, signer_state, diag)) => {
            signers.push_signer_state(signer_state);
            Err((signers, diag))
        }
    }
}
pub async fn consolidate_signer_activate_future_result(
    future: SignerActivateFutureResult,
) -> Result<
    Result<(SignersState, CommandExecutionResult), (SignersState, Diagnostic)>,
    (SignersState, Diagnostic),
> {
    match future {
        Ok(res) => Ok(consolidate_signer_activate_result(res.await)),
        Err((mut signers, signer_state, diag)) => {
            signers.push_signer_state(signer_state);
            Err((signers, diag))
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
        &AddonDefaults,
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
        &AddonDefaults,
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
    &AddonDefaults,
    &RunbookSupervisionContext,
    bool,
    bool,
) -> SignerActionsFutureResult;

pub type SignerActionsFutureResult = Result<
    Pin<Box<dyn Future<Output = Result<CheckSignabilityOk, SignerActionErr>> + Send>>,
    SignerActionErr,
>;

pub type SignerCheckInstantiabilityClosure =
    fn(&SignerSpecification, Vec<Type>) -> Result<Type, Diagnostic>;

pub type CheckSignabilityOk = (SignersState, ValueStore, Actions);

pub type SignerCheckSignabilityClosure = fn(
    &ConstructDid,
    &str,
    &Option<String>,
    &Value,
    &SignerSpecification,
    &ValueStore,
    ValueStore,
    SignersState,
    &HashMap<ConstructDid, SignerInstance>,
    &AddonDefaults,
    &RunbookSupervisionContext,
) -> Result<CheckSignabilityOk, SignerActionErr>;

pub type SignerOperationFutureResult = Result<
    Pin<Box<dyn Future<Output = Result<SignerActionOk, SignerActionErr>> + Send>>,
    SignerActionErr,
>;

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
) -> Result<(SignersState, Actions), (SignersState, Diagnostic)> {
    match res {
        Ok((mut signers, signer_state, actions)) => {
            signers.push_signer_state(signer_state);
            Ok((signers, actions))
        }
        Err((mut signers, signer_state, diag)) => {
            signers.push_signer_state(signer_state);
            Err((signers, diag))
        }
    }
}
pub async fn consolidate_signer_future_result(
    future: SignerActionsFutureResult,
) -> Result<Result<(SignersState, Actions), (SignersState, Diagnostic)>, (SignersState, Diagnostic)>
{
    match future {
        Ok(res) => Ok(consolidate_signer_result(res.await)),
        Err((mut signers, signer_state, diag)) => {
            signers.push_signer_state(signer_state);
            Err((signers, diag))
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
}

#[derive(Debug, Clone)]
pub struct SignerInstance {
    pub specification: SignerSpecification,
    pub name: String,
    pub block: Block,
    pub package_id: PackageId,
    pub namespace: String,
}

impl SignerInstance {
    pub fn check_inputs(&self) -> Result<Vec<Diagnostic>, Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        let mut has_errors = false;

        for input in self.specification.inputs.iter() {
            match (input.optional, self.block.body.get_attribute(&input.name)) {
                (false, None) => {
                    has_errors = true;
                    diagnostics.push(Diagnostic::error_from_expression(
                        &self.block,
                        None,
                        format!("missing attribute '{}'", input.name),
                    ));
                }
                (_, Some(_attr)) => {
                    // todo(lgalabru): check typing
                }
                (_, _) => {}
            }
        }

        // todo(lgalabru): check arbitrary attributes

        if has_errors {
            Err(diagnostics)
        } else {
            Ok(diagnostics)
        }
    }

    pub fn compute_fingerprint(&self, evaluated_inputs: &CommandInputsEvaluationResult) -> Did {
        let mut comps = vec![];
        for input in self.specification.inputs.iter() {
            let Some(value) = evaluated_inputs.inputs.get_value(&input.name) else { continue };
            if input.sensitive {
                comps.push(value.to_bytes());
            }
        }
        Did::from_components(comps)
    }

    pub fn get_expressions_referencing_commands_from_inputs(
        &self,
    ) -> Result<Vec<(Option<&CommandInput>, Expression)>, String> {
        let mut expressions = vec![];
        for input in self.specification.inputs.iter() {
            match input.typing {
                Type::Object(ref props) => {
                    for prop in props.iter() {
                        let mut blocks_iter = self.block.body.get_blocks(&input.name);
                        while let Some(block) = blocks_iter.next() {
                            let res = visit_optional_untyped_attribute(&prop.name, &block)
                                .map_err(|e| format!("{:?}", e))?;
                            if let Some(expr) = res {
                                let mut references = vec![];
                                collect_constructs_references_from_expression(
                                    &expr,
                                    Some(input),
                                    &mut references,
                                );
                                expressions.append(&mut references);
                            }
                        }
                    }
                }
                _ => {
                    let res = visit_optional_untyped_attribute(&input.name, &self.block)
                        .map_err(|e| format!("{:?}", e))?;
                    if let Some(expr) = res {
                        let mut references = vec![];
                        collect_constructs_references_from_expression(
                            &expr,
                            Some(input),
                            &mut references,
                        );
                        expressions.append(&mut references);
                    }
                }
            }
        }
        Ok(expressions)
    }

    /// Checks the `CommandInstance` HCL Block for an attribute named `input.name`
    pub fn get_expression_from_input(
        &self,
        input: &CommandInput,
    ) -> Result<Option<Expression>, Vec<Diagnostic>> {
        let res = match &input.typing {
            Type::Object(_) => unreachable!(),
            _ => visit_optional_untyped_attribute(&input.name, &self.block)?,
        };
        match (res, input.optional) {
            (Some(res), _) => Ok(Some(res)),
            (None, true) => Ok(None),
            (None, false) => Err(vec![Diagnostic::error_from_string(format!(
                "command '{}' (type '{}') is missing value for field '{}'",
                self.name, self.specification.matcher, input.name
            ))]),
        }
    }

    pub fn get_group(&self) -> String {
        let Some(group) = self.block.body.get_attribute("group") else {
            return format!("{} Review", self.specification.name.to_string());
        };
        group.value.to_string()
    }

    pub fn get_expression_from_object_property(
        &self,
        input: &CommandInput,
        prop: &ObjectProperty,
    ) -> Result<Option<Expression>, Vec<Diagnostic>> {
        let object = self.block.body.get_blocks(&input.name).next();
        match (object, input.optional) {
            (Some(block), _) => {
                let expr_res = visit_optional_untyped_attribute(&prop.name, &block)?;
                match (expr_res, prop.optional) {
                    (Some(expression), _) => Ok(Some(expression)),
                    (None, true) => Ok(None),
                    (None, false) => Err(vec![Diagnostic::error_from_string(format!(
                        "command '{}' (type '{}') is missing property '{}' for object '{}'",
                        self.name, self.specification.matcher, prop.name, input.name
                    ))]),
                }
            }
            (None, true) => Ok(None),
            (None, false) => Err(vec![Diagnostic::error_from_string(format!(
                "command '{}' (type '{}') is missing object '{}'",
                self.name, self.specification.matcher, input.name
            ))]),
        }
    }

    pub async fn check_activability(
        &self,
        construct_did: &ConstructDid,
        input_evaluation_results: &mut CommandInputsEvaluationResult,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        addon_defaults: &AddonDefaults,
        _action_item_requests: &Option<&Vec<&mut ActionItemRequest>>,
        action_item_responses: &Option<&Vec<ActionItemResponse>>,
        supervision_context: &RunbookSupervisionContext,
        is_balance_check_required: bool,
        is_public_key_required: bool,
    ) -> Result<(SignersState, Actions), (SignersState, Diagnostic)> {
        let mut values = ValueStore::new(&self.name, &construct_did.value());
        for input in self.specification.inputs.iter() {
            let value = match input_evaluation_results.inputs.get_value(&input.name) {
                Some(value) => value.clone(),
                None => match input.optional {
                    true => continue,
                    false => unreachable!(), // todo: return diagnostic
                },
            };
            values.insert(&input.name, value);
        }

        match action_item_responses {
            Some(responses) => {
                for ActionItemResponse { payload, .. } in responses.iter() {
                    match payload {
                        ActionItemResponseType::ProvidePublicKey(update) => {
                            values.insert("public_key", Value::string(update.public_key.clone()));

                            let signer_state = signers.pop_signer_state(construct_did).unwrap();
                            let res = ((&self.specification).check_activability)(
                                &construct_did,
                                &self.name,
                                &self.specification,
                                &values,
                                signer_state,
                                signers,
                                signers_instances,
                                &addon_defaults,
                                &supervision_context,
                                is_balance_check_required,
                                is_public_key_required,
                            );
                            return consolidate_signer_future_result(res).await?.map_err(
                                |(state, diag)| (state, diag.set_span_range(self.block.span())),
                            );
                            // WIP
                            // let (status, success) = match &res {
                            //     Ok((_, actions)) => {

                            //         (ActionItemStatus::Success(message.clone()), true)
                            //     }
                            //     Err(diag) => (ActionItemStatus::Error(diag.clone()), false),
                            // };

                            // match request.action_type {
                            //     ActionItemRequestType::ReviewInput => {
                            //         request.action_status = status.clone();
                            //     }
                            //     ActionItemRequestType::ProvidePublicKey(_) => {
                            //         if success {
                            //             request.action_status = status.clone();
                            //         }
                            //     }
                            //     _ => unreachable!(),
                            // }

                            // for request in action_item_requests.iter() {}
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
            &addon_defaults,
            &supervision_context,
            is_balance_check_required,
            is_public_key_required,
        );

        consolidate_signer_future_result(res)
            .await?
            .map_err(|(state, diag)| (state, diag.set_span_range(self.block.span())))
    }

    pub async fn perform_activation(
        &self,
        construct_did: &ConstructDid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        addon_defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
    ) -> Result<(SignersState, CommandExecutionResult), (SignersState, Diagnostic)> {
        // todo: I don't think this one needs to be a result
        let mut values = ValueStore::new(&self.name, &construct_did.value());
        for (key, value) in evaluated_inputs.inputs.iter() {
            values.insert(&key, value.clone());
        }

        let signer_state = signers.pop_signer_state(construct_did).unwrap();
        let future = (&self.specification.activate)(
            &construct_did,
            &self.specification,
            &values,
            signer_state,
            signers,
            signers_instances,
            &addon_defaults,
            progress_tx,
        );
        let res = consolidate_signer_activate_future_result(future)
            .await?
            .map_err(|(state, diag)| (state, diag.set_span_range(self.block.span())));

        res
    }

    pub fn collect_dependencies(&self) -> Vec<(Option<&CommandInput>, Expression)> {
        let mut dependencies = vec![];
        for input in self.specification.inputs.iter() {
            match input.typing {
                Type::Object(ref props) => {
                    for prop in props.iter() {
                        let mut blocks_iter = self.block.body.get_blocks(&input.name);
                        while let Some(block) = blocks_iter.next() {
                            let Some(attr) = block.body.get_attribute(&prop.name) else {
                                continue;
                            };
                            collect_constructs_references_from_expression(
                                &attr.value,
                                Some(input),
                                &mut dependencies,
                            );
                        }
                    }
                }
                _ => {
                    let Some(attr) = self.block.body.get_attribute(&input.name) else {
                        continue;
                    };
                    collect_constructs_references_from_expression(
                        &attr.value,
                        Some(input),
                        &mut dependencies,
                    );
                }
            }
        }
        dependencies
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
        _args: &ValueStore,
        _signer_state: ValueStore,
        _signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        unimplemented!()
    }

    fn activate(
        _construct_id: &ConstructDid,
        _spec: &SignerSpecification,
        _args: &ValueStore,
        _signer_state: ValueStore,
        _signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        unimplemented!()
    }

    fn check_signability(
        _caller_uuid: &ConstructDid,
        _title: &str,
        _description: &Option<String>,
        _payload: &Value,
        _spec: &SignerSpecification,
        _args: &ValueStore,
        _signer_state: ValueStore,
        _signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        unimplemented!()
    }

    fn sign(
        _caller_uuid: &ConstructDid,
        _title: &str,
        _payload: &Value,
        _spec: &SignerSpecification,
        _args: &ValueStore,
        _signer_state: ValueStore,
        _signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _defaults: &AddonDefaults,
    ) -> SignerSignFutureResult {
        unimplemented!()
    }
}
