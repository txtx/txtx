use crate::typing::CLARITY_BUFFER;
use clarity::address::{public_keys_to_address_hash, AddressHashMode};
use clarity::types::chainstate::StacksPublicKey;
use clarity_repl::codec::{
    MultisigHashMode, MultisigSpendingCondition, StacksTransaction, TransactionAuth,
    TransactionSpendingCondition, TransactionVersion,
};
use clarity_repl::{clarity::codec::StacksMessageCodec, codec::TransactionPayload};
use std::collections::HashMap;
use txtx_addon_kit::types::commands::{
    return_synchronous_ok, CommandExecutionContext, CommandExecutionFutureResult,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::ActionItemRequest;
use txtx_addon_kit::types::wallets::WalletInstance;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
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
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Vec<ActionItemRequest>, Diagnostic> {
        unimplemented!()
    }

    fn execute(
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        _wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        _progress_tx: &txtx_addon_kit::channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult {
        let mut result = CommandExecutionResult::new();

        // Extract network_id
        let network_id = args.retrieve_value_using_defaults("network_id", defaults)?;

        let buffer = args.get_expected_buffer("bytes", &CLARITY_BUFFER)?;

        let public_keys = args.get_expected_array("public_keys")?;

        let stacks_public_keys: Vec<StacksPublicKey> = public_keys
            .iter()
            .map(|v| {
                StacksPublicKey::from_hex(v.expect_string())
                    // .map_err(|e| Diagnostic::error_from_string(e.to_string()))
                    .unwrap()
            })
            // .collect::<Result<Vec<StacksPublicKey>, Diagnostic>>()?
            .collect::<Vec<StacksPublicKey>>();

        let payload = TransactionPayload::consensus_deserialize(&mut &buffer.bytes[..])
            .map_err(|e| Diagnostic::error_from_string(e.to_string()))
            .unwrap();
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

        return_synchronous_ok(result)
    }
}
