use clarity::address::public_keys_to_address_hash;
use clarity::types::chainstate::StacksPublicKey;
use clarity_repl::codec::{MultisigHashMode, MultisigSpendingCondition};
use clarity_repl::{
    clarity::{address::AddressHashMode, codec::StacksMessageCodec},
    codec::{
        StacksTransaction, TransactionAuth, TransactionPayload, TransactionSpendingCondition,
        TransactionVersion,
    },
};
use std::collections::HashMap;
use std::pin::Pin;
use txtx_addon_kit::types::commands::{CommandImplementationAsync, PreCommandSpecification};
use txtx_addon_kit::types::wallets::{WalletRunner, WalletSpecification};
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandInputsEvaluationResult, CommandSpecification},
    diagnostics::Diagnostic,
    types::{PrimitiveValue, Type, Value},
};
use txtx_addon_kit::AddonDefaults;

lazy_static! {
    pub static ref SIGN_STACKS_TRANSACTION: PreCommandSpecification = define_async_command! {
      SignStacksTransaction => {
          name: "Sign Stacks Transaction",
          matcher: "sign_transaction",
          documentation: "The `sign_transaction` action signs an encoded transaction payload with the supplied wallet data.",
          inputs: [
            transaction_payload_bytes: {
                documentation: "The transaction payload bytes, encoded as a clarity buffer.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            network_id: {
                documentation: indoc!{r#"The network id, which is used to set the transaction version. Can be `"testnet"` or `"mainnet"`."#},
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            signer: {
              documentation: "Coming soon",
              typing: Type::string(),
              optional: false,
              interpolable: true
            }
          ],
          outputs: [
              signed_transaction_bytes: {
                  documentation: "The signed transaction bytes.",
                  typing: Type::string()
              },
              network_id: {
                  documentation: "Network id of the signed transaction.",
                  typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
          action "my_ref" "stacks::sign_transaction" {
              transaction_payload_bytes = encode_buffer("0x021A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0E707974682D6F7261636C652D76311D7665726966792D616E642D7570646174652D70726963652D66656564730000000202000000030102030C0000000315707974682D6465636F6465722D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE14707974682D706E61752D6465636F6465722D763115707974682D73746F726167652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0D707974682D73746F72652D763116776F726D686F6C652D636F72652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE10776F726D686F6C652D636F72652D7631")
              nonce = 1
              fee = 1200
              sender_mnemonic = "fetch outside black test wash cover just actual execute nice door want airport betray quantum stamp fish act pen trust portion fatigue scissors vague"
              sender_derivation_path = "m/44'/5757'/0'/0/0"
              network_id = "testnet"
          }
          output "signed_bytes" {
            value = action.my_ref.signed_transaction_bytes
          }
          // > signed_bytes: 0x8080000000040063A5EDA39412C016478AE5A8C300843879F78245000000000000000100000000000004B0000182C1712C31B7F683F6C56EEE8920892F735FC0118C98FD10C1FDAA85ABEC2808063773E5F61229D76B29784B8BBBBAAEA72EEA701C92A4FE15EF3B9E32A373D8020100000000021A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0E707974682D6F7261636C652D76311D7665726966792D616E642D7570646174652D70726963652D66656564730000000202000000030102030C0000000315707974682D6465636F6465722D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE14707974682D706E61752D6465636F6465722D763115707974682D73746F726167652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0D707974682D73746F72652D763116776F726D686F6C652D636F72652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE10776F726D686F6C652D636F72652D7631
      "#},
      }
    };
}

pub struct SignStacksTransaction;
impl CommandImplementationAsync for SignStacksTransaction {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        defaults: &AddonDefaults,
        wallets: &HashMap<String, WalletSpecification>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<CommandExecutionResult, Diagnostic>>>>
    {
        let args = args.clone();
        // Extract and decode transaction_payload_bytes
        let transaction_payload_bytes = match args.get("transaction_payload_bytes") {
            Some(Value::Primitive(PrimitiveValue::Buffer(bytes))) => bytes.clone(),
            _ => todo!("transaction_payload_bytes invalid, return diagnostic"),
        };
        let transaction_payload = match TransactionPayload::consensus_deserialize(
            &mut &transaction_payload_bytes.bytes[..],
        ) {
            Ok(res) => res,
            Err(e) => {
                todo!(
                    "transaction payload invalid, return diagnostic ({})",
                    e.to_string()
                )
            }
        };
        let signer = args
            .get("signer")
            .and_then(|a| Some(a.expect_string()))
            .ok_or(Diagnostic::error_from_string(format!(
                "command '{}': attribute 'signer' is missing",
                ctx.matcher
            )))
            .unwrap()
            .to_string();

        // Extract network_id
        let network_id = args
            .get("network_id")
            .and_then(|a| Some(a.expect_string()))
            .or(defaults.keys.get("network_id").map(|x| x.as_str()))
            .ok_or(Diagnostic::error_from_string(format!(
                "command '{}': attribute 'network_id' is missing",
                ctx.matcher
            )))
            .unwrap()
            .to_string();

        let transaction_version = match network_id.as_str() {
            "mainnet" => TransactionVersion::Mainnet,
            "testnet" => TransactionVersion::Testnet,
            _ => unimplemented!("invalid network_id, return diagnostic"),
        };

        let wallet = wallets
            .get(&signer)
            .ok_or(Diagnostic::error_from_string(format!(
                "command '{}': wallet named '{}' not found",
                ctx.matcher, &signer
            )))
            .unwrap();

        let public_keys = args
            .get("public_keys")
            .and_then(|a| Some(a.expect_array()))
            .ok_or(Diagnostic::error_from_string(format!(
                "command '{}': attribute 'public_keys' is missing",
                ctx.matcher
            )))
            .unwrap()
            .to_vec();

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

        let mut unsigned_tx =
            StacksTransaction::new(transaction_version, auth, transaction_payload);
        if let TransactionVersion::Testnet = transaction_version {
            unsigned_tx.chain_id = 0x80000000;
        }

        let WalletRunner::Async(async_sign_fn) = wallet.signer.clone() else {
            todo!();
        };
        let moved_args = args.clone();
        let moved_defaults = defaults.clone();
        let moved_wallet = wallet.clone();
        let future =
            async move { async_sign_fn(&moved_wallet, &moved_args, &moved_defaults).await };

        Box::pin(future)
    }

    fn update_input_evaluation_results_from_user_input(
        _ctx: &CommandSpecification,
        _current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String,
        _value: String,
    ) {
        unimplemented!()
    }
}
