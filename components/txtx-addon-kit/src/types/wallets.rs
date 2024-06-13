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
    ConstructUuid, PackageUuid, ValueStore,
};

#[derive(Debug, Clone)]
pub struct WalletsState {
    pub store: HashMap<ConstructUuid, ValueStore>,
}

impl WalletsState {
    pub fn new() -> WalletsState {
        WalletsState {
            store: HashMap::new(),
        }
    }

    pub fn get_wallet_state_mut(&mut self, wallet_uuid: &ConstructUuid) -> Option<&mut ValueStore> {
        self.store.get_mut(wallet_uuid)
    }

    pub fn get_wallet_state(&self, wallet_uuid: &ConstructUuid) -> Option<&ValueStore> {
        self.store.get(wallet_uuid)
    }

    pub fn pop_wallet_state(&mut self, wallet_uuid: &ConstructUuid) -> Option<ValueStore> {
        self.store.remove(wallet_uuid)
    }

    pub fn push_wallet_state(&mut self, wallet_state: ValueStore) {
        self.store.insert(
            ConstructUuid::from_uuid(&wallet_state.uuid.clone()),
            wallet_state,
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

    pub fn create_new_wallet(&mut self, wallet_uuid: &ConstructUuid, wallet_name: &str) {
        if !self.store.contains_key(&wallet_uuid) {
            self.store.insert(
                wallet_uuid.clone(),
                ValueStore::new(wallet_name, &wallet_uuid.value()),
            );
        }
    }
}
pub type WalletActionOk = (WalletsState, ValueStore, CommandExecutionResult);
pub type WalletActionErr = (WalletsState, ValueStore, Diagnostic);
pub type WalletActivateFutureResult = Result<
    Pin<Box<dyn Future<Output = Result<WalletActionOk, WalletActionErr>> + Send>>,
    WalletActionErr,
>;

pub fn consolidate_wallet_activate_result(
    res: Result<WalletActionOk, WalletActionErr>,
) -> Result<(WalletsState, CommandExecutionResult), (WalletsState, Diagnostic)> {
    match res {
        Ok((mut wallets, wallet_state, result)) => {
            wallets.push_wallet_state(wallet_state);
            Ok((wallets, result))
        }
        Err((mut wallets, wallet_state, diag)) => {
            wallets.push_wallet_state(wallet_state);
            Err((wallets, diag))
        }
    }
}
pub async fn consolidate_wallet_activate_future_result(
    future: WalletActivateFutureResult,
) -> Result<
    Result<(WalletsState, CommandExecutionResult), (WalletsState, Diagnostic)>,
    (WalletsState, Diagnostic),
> {
    match future {
        Ok(res) => Ok(consolidate_wallet_activate_result(res.await)),
        Err((mut wallets, wallet_state, diag)) => {
            wallets.push_wallet_state(wallet_state);
            Err((wallets, diag))
        }
    }
}

pub type WalletActivateClosure = Box<
    fn(
        &ConstructUuid,
        &WalletSpecification,
        &ValueStore,
        ValueStore,
        WalletsState,
        &HashMap<ConstructUuid, WalletInstance>,
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
        &ConstructUuid,
        &str,
        &Value,
        &WalletSpecification,
        &ValueStore,
        ValueStore,
        WalletsState,
        &HashMap<ConstructUuid, WalletInstance>,
        &AddonDefaults,
    ) -> WalletSignFutureResult,
>;

pub type WalletCheckActivabilityClosure = fn(
    &ConstructUuid,
    &str,
    &WalletSpecification,
    &ValueStore,
    ValueStore,
    WalletsState,
    &HashMap<ConstructUuid, WalletInstance>,
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

pub type CheckSignabilityOk = (WalletsState, ValueStore, Actions);

pub type WalletCheckSignabilityClosure = fn(
    &ConstructUuid,
    &str,
    &Option<String>,
    &Value,
    &WalletSpecification,
    &ValueStore,
    ValueStore,
    WalletsState,
    &HashMap<ConstructUuid, WalletInstance>,
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
    wallets: WalletsState,
    wallet_state: ValueStore,
    res: CommandExecutionResult,
) -> WalletOperationFutureResult {
    return_synchronous_result(Ok((wallets, wallet_state, res)))
}

pub fn return_synchronous_err(
    wallets: WalletsState,
    wallet_state: ValueStore,
    diag: Diagnostic,
) -> WalletOperationFutureResult {
    return_synchronous_result(Err((wallets, wallet_state, diag)))
}

pub fn consolidate_wallet_result(
    res: Result<CheckSignabilityOk, WalletActionErr>,
) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)> {
    match res {
        Ok((mut wallets, wallet_state, actions)) => {
            wallets.push_wallet_state(wallet_state);
            Ok((wallets, actions))
        }
        Err((mut wallets, wallet_state, diag)) => {
            wallets.push_wallet_state(wallet_state);
            Err((wallets, diag))
        }
    }
}
pub async fn consolidate_wallet_future_result(
    future: WalletActionsFutureResult,
) -> Result<Result<(WalletsState, Actions), (WalletsState, Diagnostic)>, (WalletsState, Diagnostic)>
{
    match future {
        Ok(res) => Ok(consolidate_wallet_result(res.await)),
        Err((mut wallets, wallet_state, diag)) => {
            wallets.push_wallet_state(wallet_state);
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
    pub package_uuid: PackageUuid,
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
    ) -> Result<Vec<Expression>, String> {
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
                        collect_constructs_references_from_expression(&expr, &mut references);
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
        construct_uuid: &ConstructUuid,
        input_evaluation_results: &mut CommandInputsEvaluationResult,
        mut wallets: WalletsState,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        addon_defaults: &AddonDefaults,
        _action_item_requests: &Option<&Vec<&mut ActionItemRequest>>,
        action_item_responses: &Option<&Vec<ActionItemResponse>>,
        execution_context: &CommandExecutionContext,
        is_balance_check_required: bool,
        is_public_key_required: bool,
    ) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)> {
        let mut values = ValueStore::new(&self.name, &construct_uuid.value());
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

                            let wallet_state = wallets.pop_wallet_state(construct_uuid).unwrap();
                            let res = ((&self.specification).check_activability)(
                                &construct_uuid,
                                &self.name,
                                &self.specification,
                                &values,
                                wallet_state,
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

        let wallet_state = wallets.pop_wallet_state(construct_uuid).unwrap();

        let spec = &self.specification;
        let res = (spec.check_activability)(
            &construct_uuid,
            &self.name,
            &self.specification,
            &values,
            wallet_state,
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
        construct_uuid: &ConstructUuid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        mut wallets: WalletsState,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        addon_defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
    ) -> Result<(WalletsState, CommandExecutionResult), (WalletsState, Diagnostic)> {
        // todo: I don't think this one needs to be a result
        let mut values = ValueStore::new(&self.name, &construct_uuid.value());
        for (key, value) in evaluated_inputs.inputs.iter() {
            values.insert(&key, value.clone());
        }

        let wallet_state = wallets.pop_wallet_state(construct_uuid).unwrap();
        let future = (&self.specification.activate)(
            &construct_uuid,
            &self.specification,
            &values,
            wallet_state,
            wallets,
            wallets_instances,
            &addon_defaults,
            progress_tx,
        );
        let res = consolidate_wallet_activate_future_result(future).await?;

        res
    }

    pub fn collect_dependencies(&self) -> Vec<Expression> {
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
                                &mut dependencies,
                            );
                        }
                    }
                }
                _ => {
                    let Some(attr) = self.block.body.get_attribute(&input.name) else {
                        continue;
                    };
                    collect_constructs_references_from_expression(&attr.value, &mut dependencies);
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
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _wallet_state: ValueStore,
        _wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> WalletActionsFutureResult {
        unimplemented!()
    }

    fn activate(
        _uuid: &ConstructUuid,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _wallet_state: ValueStore,
        _wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult {
        unimplemented!()
    }

    fn check_signability(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _description: &Option<String>,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _wallet_state: ValueStore,
        _wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<CheckSignabilityOk, WalletActionErr> {
        unimplemented!()
    }

    fn sign(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _wallet_state: ValueStore,
        _wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
    ) -> WalletSignFutureResult {
        unimplemented!()
    }
}
