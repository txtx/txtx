use std::collections::HashMap;
use txtx_addon_kit::types::commands::{
    return_synchronous_result, CommandExecutionContext, CommandExecutionFutureResult,
    CommandExecutionResult,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus,
    ProvidePublicKeyRequest, ProvideSignedTransactionRequest,
};
use txtx_addon_kit::types::wallets::{WalletImplementation, WalletSpecification};
use txtx_addon_kit::types::ConstructUuid;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::{channel, AddonDefaults};

lazy_static! {
    pub static ref STACKS_CONNECT: WalletSpecification = define_wallet! {
        StacksConnect => {
          name: "Stacks Connect",
          matcher: "connect",
          documentation: "Coming soon",
          inputs: [
            expected_address: {
              documentation: "Coming soon",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            public_key: {
              documentation: "Coming soon",
                typing: Type::string(),
                optional: true,
                interpolable: true
            }
          ],
          outputs: [
              public_key: {
                documentation: "Coming soon",
                typing: Type::array(Type::buffer())
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
        // Coming soon
    "#},
      }
    };
}

pub struct StacksConnect;
impl WalletImplementation for StacksConnect {
    fn check_instantiability(
        _ctx: &WalletSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        uuid: &ConstructUuid,
        instance_name: &str,
        spec: &WalletSpecification,
        args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        execution_context: &CommandExecutionContext,
    ) -> Result<(), Vec<ActionItemRequest>> {
        // Early return - public key was provided and expected address checked
        let check_expected_address = args.get("expected_address").and_then(|a| a.as_string());
        if check_expected_address.is_some() {
            for input_spec in spec.inputs.iter() {
                if input_spec.name == "expected_address" && input_spec.check_performed {
                    return Ok(());
                }
            }
        }

        // Early return - public key was provided and expected addresss not specified
        if args.get("public_key").is_some() {
            for input_spec in spec.inputs.iter() {
                // todo: verify public_key/expected address match?
                if input_spec.name == "public_key" && input_spec.check_performed {
                    return Ok(());
                }
            }
        }

        let mut action_item_requests = vec![];
        action_item_requests.push(ActionItemRequest::new(
            &uuid.value(),
            &Some(uuid.value()),
            0,
            &format!("Connect wallet {instance_name}"),
            "".into(),
            ActionItemStatus::Todo,
            ActionItemRequestType::ProvidePublicKey(ProvidePublicKeyRequest {
                check_expectation_action_uuid: Some(uuid.value()),
            }),
        ));

        if let Some(expected_address) = check_expected_address {
            action_item_requests.push(ActionItemRequest::new(
                &Uuid::new_v4(),
                &Some(uuid.value()),
                0,
                "Check wallet signature provided",
                &expected_address.to_string(),
                ActionItemStatus::Todo,
                ActionItemRequestType::ReviewInput,
            ))
        }

        return Err(action_item_requests);
    }

    fn execute(
        _uuid: &ConstructUuid,
        _spec: &WalletSpecification,
        args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult {
        let mut result = CommandExecutionResult::new();
        if let Some(public_key) = args.get("public_key") {
            result
                .outputs
                .insert("public_key".to_string(), public_key.clone());
        } else {
            unreachable!("responsibility of 'check_executability'")
        }
        return_synchronous_result(Ok(result))
    }

    fn check_sign_executability(
        caller_uuid: &ConstructUuid,
        title: &str,
        payload: &Value,
        _spec: &WalletSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> ActionItemRequest {
        ActionItemRequest::new(
            &Uuid::new_v4(),
            &Some(caller_uuid.value()),
            0,
            title,
            "", //payload,
            ActionItemStatus::Todo,
            ActionItemRequestType::ProvideSignedTransaction(ProvideSignedTransactionRequest {
                check_expectation_action_uuid: Some(caller_uuid.value()),
                payload: payload.clone(),
            }),
        )
    }

    fn sign(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult {
        let result = CommandExecutionResult::new();
        return_synchronous_result(Ok(result))
    }
}
