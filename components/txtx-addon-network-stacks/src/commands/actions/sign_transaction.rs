use clarity::address::public_keys_to_address_hash;
use clarity::types::chainstate::StacksPublicKey;
use clarity::util::secp256k1::MessageSignature;
use clarity_repl::codec::{
    MultisigHashMode, MultisigSpendingCondition, SinglesigHashMode, SinglesigSpendingCondition,
    TransactionPublicKeyEncoding,
};
use clarity_repl::{
    clarity::{address::AddressHashMode, codec::StacksMessageCodec},
    codec::{
        StacksTransaction, TransactionAuth, TransactionPayload, TransactionSpendingCondition,
        TransactionVersion,
    },
};
use std::collections::HashMap;
use std::str::FromStr;
use txtx_addon_kit::types::commands::{
    return_synchronous_result, CommandExecutionContext, CommandExecutionFutureResult,
    CommandExecutionResult, CommandImplementation, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::ActionItemRequest;
use txtx_addon_kit::types::wallets::WalletInstance;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::AddonDefaults;

use crate::constants::NETWORK_ID;
use crate::typing::{CLARITY_BUFFER, STACKS_SIGNED_TRANSACTION};

lazy_static! {
    pub static ref SIGN_STACKS_TRANSACTION: PreCommandSpecification = define_command! {
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
impl CommandImplementation for SignStacksTransaction {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        uuid: &ConstructUuid,
        _instance_name: &str,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        execution_context: &CommandExecutionContext,
    ) -> Result<Vec<ActionItemRequest>, Diagnostic> {
        if let Ok(signed_transaction_bytes) =
            args.get_expected_buffer("signed_transaction_btyes", &CLARITY_BUFFER)
        {
            // check signature matching
            return Ok(vec![]);
        }

        let transaction_payload_bytes =
            args.get_expected_buffer("transaction_payload_bytes", &CLARITY_BUFFER)?;
        let transaction_payload =
            TransactionPayload::consensus_deserialize(&mut &transaction_payload_bytes.bytes[..])
                .unwrap();

        let network_id = args.retrieve_value_using_defaults("network_id", defaults)?;

        let transaction_version = match network_id.as_str() {
            "mainnet" => TransactionVersion::Mainnet,
            "testnet" => TransactionVersion::Testnet,
            "devnet" => TransactionVersion::Testnet,
            "simnet" => TransactionVersion::Testnet,
            _ => return Ok(vec![]),
        };

        let signer = args.get_expected_string("signer")?;

        let wallet_uuid = ConstructUuid::Local(Uuid::from_str(&signer).unwrap());

        let wallet = wallet_instances
            .get(&wallet_uuid)
            .ok_or(Diagnostic::error_from_string(format!(
                "command '{}': wallet named '{}' not found",
                spec.matcher, &signer
            )))
            .unwrap();

        let public_keys = wallet.store.get_expected_array("public_keys")?;
        let stacks_public_keys: Vec<StacksPublicKey> = public_keys
            .iter()
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

        let moved_args = args.clone();
        let moved_defaults = defaults.clone();
        let moved_wallet = wallet.clone();

        let mut bytes = vec![];
        unsigned_tx.consensus_serialize(&mut bytes).unwrap(); // todo
        let payload = Value::buffer(bytes, STACKS_SIGNED_TRANSACTION.clone());
        Ok(vec![(wallet.specification.check_sign_executability)(
            uuid,
            "Sign Transaction",
            &payload,
            &moved_wallet.specification,
            &moved_args,
            &moved_defaults,
            execution_context,
        )])
    }

    fn execute(
        uuid: &ConstructUuid,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        progress_tx: &txtx_addon_kit::channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult {
        if let Ok(signed_transaction_bytes) = args.get_expected_value("signed_transaction_btyes") {
            let mut result = CommandExecutionResult::new();
            result.outputs.insert(
                "signed_transaction_bytes".to_string(),
                signed_transaction_bytes.clone(),
            );
            return return_synchronous_result(Ok(result));
        }

        // Extract and decode transaction_payload_bytes
        let transaction_payload_bytes =
            args.get_expected_buffer("transaction_payload_bytes", &CLARITY_BUFFER)?;
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
        // let signer = args
        //     .get("signer")
        //     .and_then(|a| Some(a.expect_string()))
        //     .ok_or(Diagnostic::error_from_string(format!(
        //         "command '{}': attribute 'signer' is missing",
        //         spec.matcher
        //     )))
        //     .unwrap()
        //     .to_string();

        // let wallet_uuid = ConstructUuid::Local(Uuid::from_str(&signer).unwrap());

        // let wallet = wallet_instances
        //     .get(&wallet_uuid)
        //     .ok_or(Diagnostic::error_from_string(format!(
        //         "command '{}': wallet named '{}' not found",
        //         spec.matcher, &signer
        //     )))
        //     .unwrap();

        // // Extract network_id
        // let network_id = args
        //     .get("network_id")
        //     .and_then(|a| Some(a.expect_string()))
        //     .or(defaults.keys.get("network_id").map(|x| x.as_str()))
        //     .ok_or(Diagnostic::error_from_string(format!(
        //         "command '{}': attribute 'network_id' is missing",
        //         spec.matcher
        //     )))
        //     .unwrap()
        //     .to_string();

        // let transaction_version = match network_id.as_str() {
        //     "mainnet" => TransactionVersion::Mainnet,
        //     "testnet" => TransactionVersion::Testnet,
        //     _ => unimplemented!("invalid network_id, return diagnostic"),
        // };

        // let public_keys = wallet
        //     .runtime_state
        //     .get("public_keys")
        //     .and_then(|a| Some(a.expect_array()))
        //     .ok_or(Diagnostic::error_from_string(format!(
        //         "command '{}': attribute 'public_keys' is missing",
        //         spec.matcher
        //     )))
        //     .unwrap()
        //     .to_vec();

        // let stacks_public_keys: Vec<StacksPublicKey> = public_keys
        //     .clone()
        //     .into_iter()
        //     .map(|v| {
        //         StacksPublicKey::from_hex(v.expect_string())
        //             // .map_err(|e| Diagnostic::error_from_string(e.to_string()))
        //             .unwrap()
        //     })
        //     // .collect::<Result<Vec<StacksPublicKey>, Diagnostic>>()?
        //     .collect::<Vec<StacksPublicKey>>();

        // let version: u8 = wallet
        //     .runtime_state
        //     .get("hash_flag")
        //     .unwrap()
        //     .expect_uint()
        //     .try_into()
        //     .unwrap();
        // let hash_flag = AddressHashMode::from_version(version);

        // let signer =
        //     public_keys_to_address_hash(&hash_flag, stacks_public_keys.len(), &stacks_public_keys);

        // let is_multisig = wallet.runtime_state.get("multi_sig").unwrap().expect_bool();

        // let spending_condition = match is_multisig {
        //     true => TransactionSpendingCondition::Multisig(MultisigSpendingCondition {
        //         hash_mode: MultisigHashMode::P2SH,
        //         signer,
        //         nonce: 0,
        //         tx_fee: 0,
        //         fields: vec![],
        //         signatures_required: stacks_public_keys.len() as u16,
        //     }),
        //     false => TransactionSpendingCondition::Singlesig(SinglesigSpendingCondition {
        //         hash_mode: SinglesigHashMode::P2PKH,
        //         signer,
        //         nonce: 0,
        //         tx_fee: 0,
        //         key_encoding: TransactionPublicKeyEncoding::Compressed,
        //         signature: MessageSignature::empty(),
        //     }),
        // };

        // let auth = TransactionAuth::Standard(spending_condition);

        // let mut unsigned_tx =
        //     StacksTransaction::new(transaction_version, auth, transaction_payload);
        // if let TransactionVersion::Testnet = transaction_version {
        //     unsigned_tx.chain_id = 0x80000000;
        // }

        // let moved_args = args.clone();
        // let moved_defaults = defaults.clone();
        // let moved_wallet = wallet.clone();

        // let mut bytes = vec![];
        // unsigned_tx.consensus_serialize(&mut bytes).unwrap(); // todo
        // let payload = Value::buffer(bytes, STACKS_SIGNED_TRANSACTION.clone());
        // (wallet.specification.sign)(
        //     uuid,
        //     "Sign Transaction",
        //     &payload,
        //     &moved_wallet.specification,
        //     &moved_args,
        //     &moved_defaults,
        //     progress_tx,
        // )
        unimplemented!()
        // Ok(return_synchronous_result(res))
    }
}

impl SignStacksTransaction {
    fn build_unsigned_transasction(
        wallet_uuid: &ConstructUuid,
        signer: &str,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        progress_tx: &txtx_addon_kit::channel::Sender<(ConstructUuid, Diagnostic)>,
        transaction_payload: TransactionPayload,
    ) -> Result<Value, Diagnostic> {
        let wallet = wallet_instances
            .get(&wallet_uuid)
            .ok_or(Diagnostic::error_from_string(format!(
                "command '{}': wallet named '{}' not found",
                spec.matcher, &signer
            )))
            .unwrap();

        // Extract network_id
        let network_id = args.retrieve_value_using_defaults(NETWORK_ID, defaults)?;

        let transaction_version = match network_id.as_str() {
            "mainnet" => TransactionVersion::Mainnet,
            "testnet" => TransactionVersion::Testnet,
            _ => unimplemented!("invalid network_id, return diagnostic"),
        };

        let public_keys = wallet.store.get_expected_array("public_keys")?.to_vec();

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

        let version: u8 = wallet
            .store
            .get_expected_uint("hash_flag")?
            .try_into()
            .unwrap();

        let hash_flag = AddressHashMode::from_version(version);
        let signer =
            public_keys_to_address_hash(&hash_flag, stacks_public_keys.len(), &stacks_public_keys);

        let is_multisig = wallet.store.get_expected_bool("multi_sig")?;

        let spending_condition = match is_multisig {
            true => TransactionSpendingCondition::Multisig(MultisigSpendingCondition {
                hash_mode: MultisigHashMode::P2SH,
                signer,
                nonce: 0,
                tx_fee: 0,
                fields: vec![],
                signatures_required: stacks_public_keys.len() as u16,
            }),
            false => TransactionSpendingCondition::Singlesig(SinglesigSpendingCondition {
                hash_mode: SinglesigHashMode::P2PKH,
                signer,
                nonce: 0,
                tx_fee: 0,
                key_encoding: TransactionPublicKeyEncoding::Compressed,
                signature: MessageSignature::empty(),
            }),
        };

        let auth = TransactionAuth::Standard(spending_condition);

        let mut unsigned_tx =
            StacksTransaction::new(transaction_version, auth, transaction_payload);
        if let TransactionVersion::Testnet = transaction_version {
            unsigned_tx.chain_id = 0x80000000;
        }

        let mut bytes = vec![];
        unsigned_tx.consensus_serialize(&mut bytes).unwrap(); // todo
        let payload = Value::buffer(bytes, STACKS_SIGNED_TRANSACTION.clone());

        Ok(payload)
    }
}
