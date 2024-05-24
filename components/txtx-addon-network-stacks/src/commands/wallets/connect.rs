use std::collections::HashMap;
use std::future::Future;

use serde_json::Value as JsonValue;
use txtx_addon_kit::types::commands::CommandInputsEvaluationResult;
use txtx_addon_kit::types::wallets::{WalletImplementationAsync, WalletSpecification};
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::AddonDefaults;

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
impl WalletImplementationAsync for StacksConnect {
    fn check(_ctx: &WalletSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn sign(
        ctx: &WalletSpecification,
        args: &HashMap<String, Value>,
        defaults: &AddonDefaults,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<CommandExecutionResult, Diagnostic>>>> {
        todo!()
    }

    fn set_public_keys(
        ctx: &WalletSpecification,
        current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String, // todo: this may be needed to see which input is being edited (if only one at a time)
        value: String,
    ) {
        let value_json: JsonValue = match serde_json::from_str(&value) {
            Ok(value) => value,
            Err(_e) => unimplemented!(),
        };
        let value_json = value_json.as_object().unwrap();

        let expected_address_prop = ctx
            .inputs
            .iter()
            .find(|p| p.name == "expected_address")
            .expect("Missing expected_address property.");
        let expected_address = current_input_evaluation_result
            .inputs
            .get(&expected_address_prop)
            .ok_or(Diagnostic::error_from_string(format!(
                "command '{}': attribute 'expected_address' is missing",
                ctx.matcher
            )))
            .unwrap();

        let public_key_prop = ctx
            .inputs
            .iter()
            .find(|p| p.name == "public_key")
            .expect("Missing public_key property.");
        let expected_type = public_key_prop.typing.clone();
        match value_json.get("public_key") {
            Some(json_value) => {
                match Value::from_string(
                    json_value.as_str().unwrap().to_string(),
                    expected_type,
                    None,
                ) {
                    Ok(value) => {
                        let str = value.as_string();
                    }
                    Err(e) => {
                        current_input_evaluation_result
                            .inputs
                            .insert(public_key_prop.clone(), Err(e));
                    }
                };
            }
            None => {
                current_input_evaluation_result
                    .inputs
                    .remove(&public_key_prop);
            }
        }
    }
}
