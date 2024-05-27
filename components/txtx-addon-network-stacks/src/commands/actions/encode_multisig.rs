use crate::typing::CLARITY_BUFFER;
use clarity::address::{public_keys_to_address_hash, AddressHashMode};
use clarity::types::chainstate::StacksPublicKey;
use clarity_repl::codec::{
    MultisigHashMode, MultisigSpendingCondition, StacksTransaction, TransactionAuth,
    TransactionSpendingCondition, TransactionVersion,
};
use clarity_repl::{clarity::codec::StacksMessageCodec, codec::TransactionPayload};
use std::collections::HashMap;
use txtx_addon_kit::types::commands::{CommandInstance, PreCommandSpecification};
use txtx_addon_kit::types::frontend::ActionItem;
use txtx_addon_kit::types::ConstructUuid;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::AddonDefaults;

lazy_static! {
    pub static ref ENCODE_MULTISIG_TRANSACTION: PreCommandSpecification = define_command! {
        EncodeMultisigTransaction => {
          name: "Encode Multisig Transaction",
          matcher: "encode_multisig",
          documentation: "Coming soon",
          inputs: [
              bytes: {
                  documentation: "Coming soon",
                  typing: Type::buffer(),
                  optional: false,
                  interpolable: true
              },
              public_keys: {
                  documentation: "Coming soon",
                  typing: Type::array(Type::string()),
                  optional: false,
                  interpolable: true
              },
              network_id: {
                  documentation: "The network id used to validate the transaction version.",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
              }
          ],
          outputs: [
              bytes: {
                  documentation: "The encoded contract call bytes.",
                  typing: Type::buffer()
              },
              network_id: {
                  documentation: "The network id of the encoded transaction.",
                  typing: Type::string()
              },
              public_keys: {
                documentation: "Coming soon",
                typing: Type::array(Type::string())
              }
          ],
          example: txtx_addon_kit::indoc! {r#"//Coming soon"#},
      }
    };
}

pub struct EncodeMultisigTransaction;
impl CommandImplementation for EncodeMultisigTransaction {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }
    fn get_action(
        _ctx: &CommandSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        _uuid: &ConstructUuid,
        _index: u16,
        _instance: &CommandInstance,
    ) -> Option<ActionItem> {
        None
    }
    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        defaults: &AddonDefaults,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut result = CommandExecutionResult::new();

        // Extract network_id
        let network_id = args
            .get("network_id")
            .and_then(|a| Some(a.expect_string()))
            .or(defaults.keys.get("network_id").map(|x| x.as_str()))
            .ok_or(Diagnostic::error_from_string(format!(
                "Key 'network_id' is missing"
            )))
            .unwrap()
            .to_string();

        let bytes = &args
            .get("bytes")
            .and_then(|a| Some(a.expect_buffer_data()))
            .ok_or(Diagnostic::error_from_string(format!(
                "Key 'bytes' is missing"
            )))?
            .bytes;

        let public_keys = args
            .get("public_keys")
            .and_then(|a| Some(a.expect_array()))
            .ok_or(Diagnostic::error_from_string(format!(
                "Key 'public_keys' is missing"
            )))?
            .clone();

        let stacks_public_keys: Vec<StacksPublicKey> = public_keys
            .clone()
            .into_iter()
            .map(|v| {
                StacksPublicKey::from_hex(v.expect_string())
                    // .map_err(|e| Diagnostic::error_from_string(e.to_string()))
                    .unwrap()
            })
            // .collect::<Result<Vec<StacksPublicKey>, Diagnostic>>()?
            .collect::<Vec<StacksPublicKey>>();

        let payload = TransactionPayload::consensus_deserialize(&mut &bytes[..])
            .map_err(|e| Diagnostic::error_from_string(e.to_string()))?;
        let address_hash = public_keys_to_address_hash(
            &AddressHashMode::SerializeP2SH,
            stacks_public_keys.len(),
            &stacks_public_keys,
        );

        let auth = TransactionAuth::Standard(TransactionSpendingCondition::Multisig(
            MultisigSpendingCondition {
                hash_mode: MultisigHashMode::P2SH,
                signer: address_hash,
                nonce: 0,
                tx_fee: 0,
                fields: vec![],
                signatures_required: stacks_public_keys.len() as u16,
            },
        ));

        let version = match network_id.as_str() {
            "testnet" => TransactionVersion::Testnet,
            "mainnet" => TransactionVersion::Mainnet,
            _ => unreachable!(),
        };

        let mut tx = StacksTransaction::new(version, auth, payload);
        if let TransactionVersion::Testnet = version {
            tx.chain_id = 0x80000000;
        }

        let mut bytes = vec![];
        tx.consensus_serialize(&mut bytes).unwrap();
        result.outputs.insert(
            "bytes".to_string(),
            Value::buffer(bytes, CLARITY_BUFFER.clone()),
        );
        result
            .outputs
            .insert("network_id".to_string(), Value::string(network_id));

        result.outputs.insert(
            "public_keys".to_string(),
            Value::array(public_keys.clone().into_iter().collect()),
        );

        Ok(result)
    }
}
