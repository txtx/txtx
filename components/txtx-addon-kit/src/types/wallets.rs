use crate::{
    helpers::hcl::{
        collect_constructs_references_from_expression, visit_optional_untyped_attribute,
    },
    AddonDefaults,
};
use futures::future;
use hcl_edit::{expr::Expression, structure::Block};
use std::{collections::HashMap, future::Future, pin::Pin};

use super::{
    commands::{
        CommandExecutionContext, CommandExecutionResult, CommandInput,
        CommandInputsEvaluationResult, CommandOutput,
    },
    diagnostics::Diagnostic,
    frontend::{
        ActionItemRequest, ActionItemResponse, ActionItemResponseType, Actions, BlockEvent,
    },
    types::{ObjectProperty, Type, Value},
    ConstructDid, PackageDid, PackageId, ValueStore,
};

#[derive(Debug, Clone)]
pub struct SigningCommandsState {
    pub store: HashMap<ConstructDid, ValueStore>,
}

impl SigningCommandsState {
    pub fn new() -> SigningCommandsState {
        SigningCommandsState {
            store: HashMap::new(),
        }
    }

    pub fn get_signing_command_state_mut(
        &mut self,
        signing_construct_did: &ConstructDid,
    ) -> Option<&mut ValueStore> {
        self.store.get_mut(signing_construct_did)
    }

    pub fn get_signing_command_state(
        &self,
        signing_construct_did: &ConstructDid,
    ) -> Option<&ValueStore> {
        self.store.get(signing_construct_did)
    }

    pub fn pop_signing_command_state(
        &mut self,
        signing_construct_did: &ConstructDid,
    ) -> Option<ValueStore> {
        self.store.remove(signing_construct_did)
    }

    pub fn push_signing_command_state(&mut self, signing_command_state: ValueStore) {
        self.store.insert(
            ConstructDid(signing_command_state.uuid.clone()),
            signing_command_state,
        );
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

    pub fn create_new_wallet(&mut self, signing_construct_did: &ConstructDid, wallet_name: &str) {
        if !self.store.contains_key(&signing_construct_did) {
            self.store.insert(
                signing_construct_did.clone(),
                ValueStore::new(wallet_name, &signing_construct_did.value()),
            );
        }
    }
}
pub type WalletActionOk = (SigningCommandsState, ValueStore, CommandExecutionResult);
pub type WalletActionErr = (SigningCommandsState, ValueStore, Diagnostic);
pub type WalletActivateFutureResult = Result<
    Pin<Box<dyn Future<Output = Result<WalletActionOk, WalletActionErr>> + Send>>,
    WalletActionErr,
>;

pub fn consolidate_wallet_activate_result(
    res: Result<WalletActionOk, WalletActionErr>,
) -> Result<(SigningCommandsState, CommandExecutionResult), (SigningCommandsState, Diagnostic)> {
    match res {
        Ok((mut wallets, signing_command_state, result)) => {
            wallets.push_signing_command_state(signing_command_state);
            Ok((wallets, result))
        }
        Err((mut wallets, signing_command_state, diag)) => {
            wallets.push_signing_command_state(signing_command_state);
            Err((wallets, diag))
        }
    }
}
pub async fn consolidate_wallet_activate_future_result(
    future: WalletActivateFutureResult,
) -> Result<
    Result<(SigningCommandsState, CommandExecutionResult), (SigningCommandsState, Diagnostic)>,
    (SigningCommandsState, Diagnostic),
> {
    match future {
        Ok(res) => Ok(consolidate_wallet_activate_result(res.await)),
        Err((mut wallets, signing_command_state, diag)) => {
            wallets.push_signing_command_state(signing_command_state);
            Err((wallets, diag))
        }
    }
}

pub type WalletActivateClosure = Box<
    fn(
        &ConstructDid,
        &WalletSpecification,
        &ValueStore,
        ValueStore,
        SigningCommandsState,
        &HashMap<ConstructDid, WalletInstance>,
        &AddonDefaults,
        &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult,
>;

pub type WalletSignFutureResult = Result<
    Pin<Box<dyn Future<Output = Result<WalletActionOk, WalletActionErr>> + Send>>,
    WalletActionErr,
>;

pub type WalletSignClosure = Box<
    fn(
        &ConstructDid,
        &str,
        &Value,
        &WalletSpecification,
        &ValueStore,
        ValueStore,
        SigningCommandsState,
        &HashMap<ConstructDid, WalletInstance>,
        &AddonDefaults,
    ) -> WalletSignFutureResult,
>;

pub type WalletCheckActivabilityClosure = fn(
    &ConstructDid,
    &str,
    &WalletSpecification,
    &ValueStore,
    ValueStore,
    SigningCommandsState,
    &HashMap<ConstructDid, WalletInstance>,
    &AddonDefaults,
    &CommandExecutionContext,
    bool,
    bool,
) -> WalletActionsFutureResult;

pub type WalletActionsFutureResult = Result<
    Pin<Box<dyn Future<Output = Result<CheckSignabilityOk, WalletActionErr>> + Send>>,
    WalletActionErr,
>;

pub type WalletCheckInstantiabilityClosure =
    fn(&WalletSpecification, Vec<Type>) -> Result<Type, Diagnostic>;

pub type CheckSignabilityOk = (SigningCommandsState, ValueStore, Actions);

pub type WalletCheckSignabilityClosure = fn(
    &ConstructDid,
    &str,
    &Option<String>,
    &Value,
    &WalletSpecification,
    &ValueStore,
    ValueStore,
    SigningCommandsState,
    &HashMap<ConstructDid, WalletInstance>,
    &AddonDefaults,
    &CommandExecutionContext,
) -> Result<CheckSignabilityOk, WalletActionErr>;

pub type WalletOperationFutureResult = Result<
    Pin<Box<dyn Future<Output = Result<WalletActionOk, WalletActionErr>> + Send>>,
    WalletActionErr,
>;

pub fn return_synchronous_actions(
    res: Result<CheckSignabilityOk, WalletActionErr>,
) -> WalletActionsFutureResult {
    Ok(Box::pin(future::ready(res)))
}

pub fn return_synchronous_result(
    res: Result<WalletActionOk, WalletActionErr>,
) -> WalletOperationFutureResult {
    Ok(Box::pin(future::ready(res)))
}

pub fn return_synchronous_ok(
    wallets: SigningCommandsState,
    signing_command_state: ValueStore,
    res: CommandExecutionResult,
) -> WalletOperationFutureResult {
    return_synchronous_result(Ok((wallets, signing_command_state, res)))
}

pub fn return_synchronous_err(
    wallets: SigningCommandsState,
    signing_command_state: ValueStore,
    diag: Diagnostic,
) -> WalletOperationFutureResult {
    return_synchronous_result(Err((wallets, signing_command_state, diag)))
}

pub fn consolidate_wallet_result(
    res: Result<CheckSignabilityOk, WalletActionErr>,
) -> Result<(SigningCommandsState, Actions), (SigningCommandsState, Diagnostic)> {
    match res {
        Ok((mut wallets, signing_command_state, actions)) => {
            wallets.push_signing_command_state(signing_command_state);
            Ok((wallets, actions))
        }
        Err((mut wallets, signing_command_state, diag)) => {
            wallets.push_signing_command_state(signing_command_state);
            Err((wallets, diag))
        }
    }
}
pub async fn consolidate_wallet_future_result(
    future: WalletActionsFutureResult,
) -> Result<
    Result<(SigningCommandsState, Actions), (SigningCommandsState, Diagnostic)>,
    (SigningCommandsState, Diagnostic),
> {
    match future {
        Ok(res) => Ok(consolidate_wallet_result(res.await)),
        Err((mut wallets, signing_command_state, diag)) => {
            wallets.push_signing_command_state(signing_command_state);
            Err((wallets, diag))
        }
    }
}

#[derive(Debug, Clone)]
pub struct WalletSpecification {
    pub name: String,
    pub matcher: String,
    pub documentation: String,
    pub requires_interaction: bool,
    pub example: String,
    pub default_inputs: Vec<CommandInput>,
    pub inputs: Vec<CommandInput>,
    pub outputs: Vec<CommandOutput>,
    pub check_instantiability: WalletCheckInstantiabilityClosure,
    pub check_activability: WalletCheckActivabilityClosure,
    pub activate: WalletActivateClosure,
    pub check_signability: WalletCheckSignabilityClosure,
    pub sign: WalletSignClosure,
}

#[derive(Debug, Clone)]
pub struct WalletInstance {
    pub specification: WalletSpecification,
    pub name: String,
    pub block: Block,
    pub package_id: PackageId,
    pub namespace: String,
}

impl WalletInstance {
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
            Type::Primitive(_) | Type::Array(_) | Type::Addon(_) => {
                visit_optional_untyped_attribute(&input.name, &self.block)?
            }
            Type::Object(_) => unreachable!(),
        };
        match (res, input.optional) {
            (Some(res), _) => Ok(Some(res)),
            (None, true) => Ok(None),
            (None, false) => todo!(
                "command '{}' (type '{}') is missing value for field '{}'",
                self.name,
                self.specification.matcher,
                input.name
            ),
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
                    (None, false) => todo!(
                        "command '{}' (type '{}') is missing property '{}' for object '{}'",
                        self.name,
                        self.specification.matcher,
                        prop.name,
                        input.name
                    ),
                }
            }
            (None, true) => Ok(None),
            (None, false) => todo!(
                "command '{}' (type '{}') is missing object '{}'",
                self.name,
                self.specification.matcher,
                input.name
            ),
        }
    }

    pub async fn check_activability(
        &self,
        construct_did: &ConstructDid,
        input_evaluation_results: &mut CommandInputsEvaluationResult,
        mut wallets: SigningCommandsState,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        addon_defaults: &AddonDefaults,
        _action_item_requests: &Option<&Vec<&mut ActionItemRequest>>,
        action_item_responses: &Option<&Vec<ActionItemResponse>>,
        execution_context: &CommandExecutionContext,
        is_balance_check_required: bool,
        is_public_key_required: bool,
    ) -> Result<(SigningCommandsState, Actions), (SigningCommandsState, Diagnostic)> {
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

                            let signing_command_state =
                                wallets.pop_signing_command_state(construct_did).unwrap();
                            let res = ((&self.specification).check_activability)(
                                &construct_did,
                                &self.name,
                                &self.specification,
                                &values,
                                signing_command_state,
                                wallets,
                                wallets_instances,
                                &addon_defaults,
                                &execution_context,
                                is_balance_check_required,
                                is_public_key_required,
                            );
                            return consolidate_wallet_future_result(res).await?;
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

        let signing_command_state = wallets.pop_signing_command_state(construct_did).unwrap();

        let spec = &self.specification;
        let res = (spec.check_activability)(
            &construct_did,
            &self.name,
            &self.specification,
            &values,
            signing_command_state,
            wallets,
            wallets_instances,
            &addon_defaults,
            &execution_context,
            is_balance_check_required,
            is_public_key_required,
        );

        consolidate_wallet_future_result(res).await?
    }

    pub async fn perform_activation(
        &self,
        construct_did: &ConstructDid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        mut wallets: SigningCommandsState,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        addon_defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
    ) -> Result<(SigningCommandsState, CommandExecutionResult), (SigningCommandsState, Diagnostic)>
    {
        // todo: I don't think this one needs to be a result
        let mut values = ValueStore::new(&self.name, &construct_did.value());
        for (key, value) in evaluated_inputs.inputs.iter() {
            values.insert(&key, value.clone());
        }

        let signing_command_state = wallets.pop_signing_command_state(construct_did).unwrap();
        let future = (&self.specification.activate)(
            &construct_did,
            &self.specification,
            &values,
            signing_command_state,
            wallets,
            wallets_instances,
            &addon_defaults,
            progress_tx,
        );
        let res = consolidate_wallet_activate_future_result(future).await?;

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

pub trait WalletImplementation {
    fn check_instantiability(
        _ctx: &WalletSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic>;

    fn check_activability(
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _signing_command_state: ValueStore,
        _wallets: SigningCommandsState,
        _wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> WalletActionsFutureResult {
        unimplemented!()
    }

    fn activate(
        _construct_id: &ConstructDid,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _signing_command_state: ValueStore,
        _wallets: SigningCommandsState,
        _wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult {
        unimplemented!()
    }

    fn check_signability(
        _caller_uuid: &ConstructDid,
        _title: &str,
        _description: &Option<String>,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _signing_command_state: ValueStore,
        _wallets: SigningCommandsState,
        _wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<CheckSignabilityOk, WalletActionErr> {
        unimplemented!()
    }

    fn sign(
        _caller_uuid: &ConstructDid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _signing_command_state: ValueStore,
        _wallets: SigningCommandsState,
        _wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        _defaults: &AddonDefaults,
    ) -> WalletSignFutureResult {
        unimplemented!()
    }
}
