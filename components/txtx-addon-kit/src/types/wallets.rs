use std::{collections::HashMap, future::Future, pin::Pin};

use futures::future;
use hcl_edit::{expr::Expression, structure::Block};

use crate::{
    helpers::hcl::{
        collect_constructs_references_from_expression, visit_optional_untyped_attribute,
    },
    AddonDefaults,
};

use super::{
    commands::{
        CommandExecutionContext, CommandExecutionResult,
        CommandInput, CommandInputsEvaluationResult, CommandOutput,
    },
    diagnostics::{Diagnostic, DiagnosticLevel},
    frontend::{ActionItemRequest, ActionItemResponseType, Actions, BlockEvent},
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
        println!("Popping {:?}", wallet_uuid);
        self.store.remove(wallet_uuid)
    }

    pub fn push_wallet_state(&mut self, wallet_uuid: &ConstructUuid, wallet_state: ValueStore) {
        self.store.insert(wallet_uuid.clone(), wallet_state);
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
            self.store.insert(wallet_uuid.clone(), ValueStore::new(wallet_name));
        }
    }

}

pub type WalletActivateFutureResult = Result<
    Pin<
        Box<
            dyn Future<
                    Output = Result<
                        (WalletsState, CommandExecutionResult),
                        (WalletsState, Diagnostic),
                    >,
                > + Send,
        >,
    >,
    (WalletsState, Diagnostic),
>;

pub type WalletActivateClosure = Box<
    fn(
        &ConstructUuid,
        &WalletSpecification,
        &ValueStore,
        ValueStore,
        WalletsState,
        &AddonDefaults,
        &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult,
>;

pub type WalletSignFutureResult = Result<
    Pin<
        Box<
            dyn Future<
                    Output = Result<
                        (WalletsState, CommandExecutionResult),
                        (WalletsState, Diagnostic),
                    >,
                > + Send,
        >,
    >,
    (WalletsState, Diagnostic),
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
&AddonDefaults,
    &CommandExecutionContext,
) -> WalletActivabilityFutureResult;

pub type WalletActivabilityFutureResult = Result<
    Pin<
        Box<
            dyn Future<Output = Result<(WalletsState, Actions), (WalletsState, Diagnostic)>> + Send,
        >,
    >,
    (WalletsState, Diagnostic),
>;

pub type WalletCheckInstantiabilityClosure =
    fn(&WalletSpecification, Vec<Type>) -> Result<Type, Diagnostic>;

pub type WalletCheckSignabilityClosure =
    fn(
        &ConstructUuid,
        &str,
        &Value,
        &WalletSpecification,
        &ValueStore,
        ValueStore,
        WalletsState,
        &AddonDefaults,
        &CommandExecutionContext,
    ) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)>;

pub type WalletOperationFutureResult = Result<
    Pin<
        Box<
            dyn Future<
                    Output = Result<
                        (WalletsState, CommandExecutionResult),
                        (WalletsState, Diagnostic),
                    >,
                > + Send,
        >,
    >,
    (WalletsState, Diagnostic),
>;

pub fn return_synchronous_result(
    res: Result<(WalletsState, CommandExecutionResult), (WalletsState, Diagnostic)>,
) -> WalletOperationFutureResult {
    Ok(Box::pin(future::ready(res)))
}

pub fn return_synchronous_ok(
    wallets_state: WalletsState,
    res: CommandExecutionResult,
) -> WalletOperationFutureResult {
    return_synchronous_result(Ok((wallets_state, res)))
}

pub fn return_synchronous_err(
    wallets_state: WalletsState,
    diag: Diagnostic,
) -> WalletOperationFutureResult {
    return_synchronous_result(Err((wallets_state, diag)))
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
    ) -> Result<Option<Expression>, Diagnostic> {
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
    ) -> Result<Option<Expression>, Diagnostic> {
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
        input_evaluation_results: &CommandInputsEvaluationResult,
        mut wallets: WalletsState,
        addon_defaults: &AddonDefaults,
        action_item_requests: &Option<&Vec<ActionItemRequest>>,
        action_item_responses: &Option<&Vec<ActionItemResponseType>>,
        execution_context: &CommandExecutionContext,
    ) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)> {
        let mut values = ValueStore::new(&self.name);
        for input in self.specification.inputs.iter() {
            let value = match input_evaluation_results.inputs.get(&input.name) {
                Some(Ok(value)) => Ok(value.clone()),
                Some(Err(e)) => Err(Diagnostic {
                    span: None,
                    location: None,
                    message: format!("Cannot execute command due to erroring inputs"),
                    level: DiagnosticLevel::Error,
                    documentation: None,
                    example: None,
                    parent_diagnostic: Some(Box::new(e.clone())),
                }),
                None => match input.optional {
                    true => continue,
                    false => unreachable!(), // todo: return diagnostic
                },
            }
            .unwrap();
            values.insert(&input.name, value);
        }

        match action_item_responses {
            Some(responses) => {
                for response in responses.iter() {
                    match response {
                        ActionItemResponseType::ProvidePublicKey(update) => {
                            let public_key_bytes: Vec<u8> =
                                hex::decode(&update.public_key).expect("unable to decode bytes");

                            // input_evaluation_results.inputs.insert(
                            //     "public_key".into(),
                            //     Ok(Value::string(update.public_key.clone())),
                            // );
                            // for input in self.specification.inputs.iter_mut() {
                            //     if input.name.eq("public_key") {
                            //         input.check_performed = true;
                            //         break;
                            //     }
                            // }

                            // let res = ((&self.specification).check_public_key_expectations)(
                            //     &construct_uuid,
                            //     &self.name,
                            //     &public_key_bytes,
                            //     &self.specification,
                            //     &values,
                            //     wallets_state,
                            //     &addon_defaults,
                            //     &execution_context,
                            // );

                            values.insert("public_key", Value::string(update.public_key.clone()));

                            // for request in action_item_requests.iter_mut() {
                            //     let (status, success) = match &res {
                            //         Ok(message) => {
                            //             (ActionItemStatus::Success(message.clone()), true)
                            //         }
                            //         Err(diag) => (ActionItemStatus::Error(diag.clone()), false),
                            //     };

                            //     match request.action_type {
                            //         ActionItemRequestType::ReviewInput => {
                            //             request.action_status = status.clone();
                            //         }
                            //         ActionItemRequestType::ProvidePublicKey(_) => {
                            //             if success {
                            //                 request.action_status = status.clone();
                            //             }
                            //         }
                            //         _ => unreachable!(),
                            //     }
                            // }
                            return Ok((wallets, Actions::none()));
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
            &addon_defaults,
            &execution_context,
        )?
        .await;
        
        res
    }

    pub async fn perform_activation(
        &self,
        construct_uuid: &ConstructUuid,
        evaluated_inputs: &CommandInputsEvaluationResult,
        mut wallets: WalletsState,
        addon_defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
    ) -> Result<(WalletsState, CommandExecutionResult), (WalletsState, Diagnostic)> {
        // todo: I don't think this one needs to be a result
        let mut values = ValueStore::new(&self.name);
        for (key, value_res) in evaluated_inputs.inputs.iter() {
            match value_res {
                Ok(value) => {
                    values.insert(&key, value.clone());
                }
                Err(diag) => return Err((wallets, diag.clone())),
            };
        }

        println!("==> {:?}", values);

        let wallet_state = wallets.pop_wallet_state(construct_uuid).unwrap();

        let res = (&self.specification.activate)(
            &construct_uuid,
            &self.specification,
            &values,
            wallet_state,
            wallets,
            &addon_defaults,
            progress_tx,
        )?
        .await;

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
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> WalletActivabilityFutureResult;

    fn activate(
        _uuid: &ConstructUuid,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _wallet_state: ValueStore,
        _wallets: WalletsState,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult;

    fn check_signability(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _wallet_state: ValueStore,
        wallets: WalletsState,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)> {
        Ok((wallets, Actions::none()))
    }

    fn sign(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _wallet_state: ValueStore,
        _wallets: WalletsState,
        _defaults: &AddonDefaults,
    ) -> WalletSignFutureResult;
}
