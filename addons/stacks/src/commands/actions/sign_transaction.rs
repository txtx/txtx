use crate::codec::codec::{
    MultisigHashMode, MultisigSpendingCondition, SinglesigHashMode, SinglesigSpendingCondition,
    StacksMessageCodec, StacksTransaction, TransactionAuth, TransactionContractCall,
    TransactionPayload, TransactionPostCondition, TransactionPostConditionMode,
    TransactionPublicKeyEncoding, TransactionSpendingCondition, TransactionVersion,
};
use clarity::types::chainstate::{StacksAddress, StacksPublicKey};
use clarity::types::Address;
use clarity::util::secp256k1::MessageSignature;
use clarity::vm::{ClarityName, ContractName};
use clarity_repl::clarity::address::AddressHashMode;
use std::collections::HashMap;
use txtx_addon_kit::constants::SignerKey;
use txtx_addon_kit::types::commands::{
    CommandExecutionResult, CommandImplementation, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemStatus, Actions, BlockEvent, ReviewInputRequest,
};
use txtx_addon_kit::types::signers::{
    return_synchronous_ok, SignerActionsFutureResult, SignerInstance, SignerSignFutureResult,
    SignersState,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};

use crate::constants::{
    NETWORK_ID, PUBLIC_KEYS, RPC_API_URL, TRANSACTION_PAYLOAD_BYTES, UNSIGNED_TRANSACTION_BYTES,
};

use crate::rpc::StacksRpc;
use crate::typing::StacksValue;

use super::get_signer_did;

lazy_static! {
    pub static ref SIGN_STACKS_TRANSACTION: PreCommandSpecification = define_command! {
      SignStacksTransaction => {
          name: "Sign Stacks Transaction",
          matcher: "sign_transaction",
          documentation: "The `stacks::sign_transaction` action signs an encoded transaction payload with the specified signer.",
          implements_signing_capability: true,
          implements_background_task_capability: false,
          inputs: [
            description: {
                documentation: "Description of the transaction",
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false
            },
            transaction_payload_bytes: {
                documentation: "The transaction payload bytes, encoded as a clarity buffer.",
                typing: Type::string(),
                optional: false,
                tainting: true,
                internal: false
            },
            network_id: {
                documentation: indoc!{r#"The network id, which is used to set the transaction version. Valid values are `"mainnet"`, `"testnet"` or `"devnet"`."#},
                typing: Type::string(),
                optional: false,
                tainting: true,
                internal: false
            },
            signer: {
                documentation: "A reference to a signer construct, which will be used to sign the transaction payload.",
                typing: Type::string(),
                optional: false,
                tainting: true,
                internal: false
            },
            nonce: {
                documentation: "The account nonce of the signer. This value will be retrieved from the network if omitted.",
                typing: Type::integer(),
                optional: true,
                tainting: false,
                internal: false
            },
            fee: {
                documentation: "The transaction fee. This value will automatically be estimated if omitted.",
                typing: Type::integer(),
                optional: false,
                tainting: false,
                internal: false
            },
            fee_strategy: {
                documentation: "The strategy to use for automatically estimating fee ('low', 'medium', 'high'). Default to 'medium'.",
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false
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
              transaction_payload_bytes = stacks::cv_buff("0x021A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0E707974682D6F7261636C652D76311D7665726966792D616E642D7570646174652D70726963652D66656564730000000202000000030102030C0000000315707974682D6465636F6465722D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE14707974682D706E61752D6465636F6465722D763115707974682D73746F726167652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE0D707974682D73746F72652D763116776F726D686F6C652D636F72652D636F6E7472616374061A6D78DE7B0625DFBFC16C3A8A5735F6DC3DC3F2CE10776F726D686F6C652D636F72652D7631")
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

    #[cfg(not(feature = "wasm"))]
    fn check_signed_executability(
        construct_did: &ConstructDid,
        instance_name: &str,
        spec: &CommandSpecification,
        args: &ValueStore,
        supervision_context: &RunbookSupervisionContext,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerActionsFutureResult {
        use txtx_addon_kit::constants::SignerKey;

        use crate::constants::{
            ACTION_ITEM_CHECK_FEE, ACTION_ITEM_CHECK_NONCE, FORMATTED_TRANSACTION,
        };

        let signer_did = get_signer_did(args).unwrap();
        let signer = signers_instances.get(&signer_did).unwrap().clone();
        let construct_did = construct_did.clone();
        let instance_name = instance_name.to_string();
        let spec = spec.clone();
        let values = args.clone();
        let supervision_context = supervision_context.clone();
        let signers_instances = signers_instances.clone();

        let future = async move {
            let mut actions = Actions::none();
            let mut signer_state = signers.pop_signer_state(&signer_did).unwrap();
            if signer_state
                .get_scoped_value(&construct_did.to_string(), SignerKey::SignedTransactionBytes)
                .is_some()
                || signer_state
                    .get_scoped_value(&construct_did.to_string(), SignerKey::SignatureApproved)
                    .is_some()
            {
                return Ok((signers, signer_state, Actions::none()));
            }

            let nonce = values.get_value("nonce").map(|v| v.expect_uint().unwrap());
            let fee = values.get_value("fee").map(|v| v.expect_uint().unwrap());
            let fee_strategy = values.get_string("fee_strategy");
            let post_conditions = match values.get_value("post_conditions") {
                Some(Value::Addon(v)) => vec![Value::Addon(v.clone())],
                Some(Value::Array(data)) => *data.clone(),
                _ => vec![],
            };
            let post_condition_mode = match values.get_value("post_condition_mode") {
                Some(Value::String(v)) => Value::string(v.into()),
                _ => Value::string("deny".into()),
            };

            let transaction = match build_unsigned_transaction(
                &construct_did,
                &mut signer_state,
                &spec,
                fee,
                fee_strategy,
                nonce,
                post_conditions,
                post_condition_mode,
                &values,
            )
            .await
            {
                Ok(transaction) => transaction,
                Err(diag) => {
                    return Err((signers, signer_state, diag));
                }
            };

            let mut bytes = vec![];
            transaction.consensus_serialize(&mut bytes).unwrap(); // todo
            let payload = StacksValue::transaction(bytes);

            let display_payload = transaction.format_for_display();
            signer_state.insert_scoped_value(
                &construct_did.to_string(),
                FORMATTED_TRANSACTION,
                Value::string(display_payload),
            );
            signer_state.insert_scoped_value(
                &construct_did.to_string(),
                UNSIGNED_TRANSACTION_BYTES,
                payload.clone(),
            );
            signers.push_signer_state(signer_state);
            let description =
                values.get_expected_string("description").ok().and_then(|d| Some(d.to_string()));

            if supervision_context.review_input_values {
                actions.push_group(
                    &description
                        .clone()
                        .unwrap_or("Review and sign the transactions from the list below".into()),
                    vec![
                        ReviewInputRequest::new(
                            "",
                            &Value::integer(transaction.get_origin_nonce() as i128),
                        )
                        .to_action_type()
                        .to_request(&instance_name, ACTION_ITEM_CHECK_NONCE)
                        .with_construct_did(&construct_did)
                        .with_meta_description("Check account nonce"),
                        ReviewInputRequest::new(
                            "".into(),
                            &Value::integer(transaction.get_tx_fee() as i128),
                        )
                        .to_action_type()
                        .to_request("µSTX", ACTION_ITEM_CHECK_FEE)
                        .with_construct_did(&construct_did)
                        .with_meta_description("Check transaction fee"),
                    ],
                )
            }

            let signer_state = signers.pop_signer_state(&signer_did).unwrap();

            let (signers, signer_state, mut signer_actions) =
                (signer.specification.check_signability)(
                    &construct_did,
                    &instance_name,
                    &description,
                    &payload,
                    &signer.specification,
                    &values,
                    signer_state,
                    signers,
                    &signers_instances,
                    &supervision_context,
                )?;
            actions.append(&mut signer_actions);
            Ok((signers, signer_state, actions))
        };
        Ok(Box::pin(future))
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerSignFutureResult {
        let signer_did = get_signer_did(args).unwrap();
        let signer_state = signers.pop_signer_state(&signer_did).unwrap();

        if let Ok(signed_transaction_bytes) = args.get_expected_value(SignerKey::SignedTransactionBytes) {
            let mut result = CommandExecutionResult::new();
            result
                .outputs
                .insert(SignerKey::SignedTransactionBytes, signed_transaction_bytes.clone());
            return return_synchronous_ok(signers, signer_state, result);
        }

        let signer = signers_instances.get(&signer_did).unwrap();

        let payload = signer_state
            .get_scoped_value(&construct_did.to_string(), UNSIGNED_TRANSACTION_BYTES)
            .unwrap()
            .clone();

        let title = args.get_expected_string("description").unwrap_or("New Transaction".into());

        let res = (signer.specification.sign)(
            construct_did,
            title,
            &payload,
            &signer.specification,
            &args,
            signer_state,
            signers,
            signers_instances,
        );
        res
    }
}

#[cfg(not(feature = "wasm"))]
async fn build_unsigned_transaction(
    construct_did: &ConstructDid,
    signer_state: &mut ValueStore,
    _spec: &CommandSpecification,
    fee: Option<u64>,
    fee_strategy: Option<&str>,
    nonce: Option<u64>,
    post_conditions: Vec<Value>,
    post_condition_mode: Value,
    values: &ValueStore,
) -> Result<StacksTransaction, Diagnostic> {
    use crate::constants::REQUIRED_SIGNATURE_COUNT;

    use crate::constants::RPC_API_AUTH_TOKEN;
    let transaction_payload_bytes = values.get_expected_buffer_bytes(TRANSACTION_PAYLOAD_BYTES)?;
    let transaction_payload =
        match TransactionPayload::consensus_deserialize(&mut &transaction_payload_bytes[..]) {
            Ok(res) => res,
            Err(e) => {
                todo!("transaction payload invalid, return diagnostic ({})", e.to_string())
            }
        };

    let network_id = values.get_expected_string(NETWORK_ID)?;
    let default_payload = {
        let boot_address = match network_id {
            "mainnet" => "SP000000000000000000002Q6VF78",
            "testnet" => "ST000000000000000000002AMW42H",
            "devnet" => "ST000000000000000000002AMW42H",
            _ => {
                return Err(diagnosed_error!(
                    "Network {} unknown ('mainnet', 'testnet' or 'devnet')",
                    network_id
                ))
            }
        };
        TransactionPayload::ContractCall(TransactionContractCall {
            address: StacksAddress::from_string(boot_address).unwrap(),
            contract_name: ContractName::from("bns"),
            function_name: ClarityName::from("name-preorder"),
            function_args: vec![],
        })
    };

    let rpc_api_url = values.get_expected_string(RPC_API_URL)?;
    let rpc_api_auth_token = values.get_string(RPC_API_AUTH_TOKEN).and_then(|t| Some(t.to_owned()));

    let fee = match fee {
        Some(fee) => fee,
        None => {
            let fee_strategy = match fee_strategy {
                Some("low") => 0,
                Some("medium") => 1,
                Some("high") => 2,
                _ => 1,
            };
            let rpc = StacksRpc::new(&rpc_api_url, &rpc_api_auth_token);
            let fee = rpc
                .estimate_transaction_fee(&transaction_payload, fee_strategy, &default_payload)
                .await
                .map_err(|e| {
                    diagnosed_error!("failure fetching fee estimation: {}", e.to_string())
                })?;
            fee
        }
    };

    // Extract network_id
    let transaction_version = match network_id {
        "mainnet" => TransactionVersion::Mainnet,
        "testnet" => TransactionVersion::Testnet,
        "devnet" => TransactionVersion::Testnet,
        _ => {
            return Err(diagnosed_error!(
                "Network {} unknown ('mainnet', 'testnet' or 'devnet')",
                network_id
            ))
        }
    };

    let public_keys = signer_state.get_expected_array(PUBLIC_KEYS)?;

    let stacks_public_keys: Vec<StacksPublicKey> = public_keys
        .iter()
        .map(|v| {
            let bytes = v.expect_buffer_bytes();
            StacksPublicKey::from_slice(&bytes[..])
                .map_err(|e| Diagnostic::error_from_string(e.to_string()))
        })
        .collect::<Result<Vec<StacksPublicKey>, Diagnostic>>()?;

    let signer_count = stacks_public_keys.len() as u16;
    let required_signature_count: u16 = signer_state
        .get_uint(REQUIRED_SIGNATURE_COUNT)
        .unwrap()
        .and_then(|count| Some(count.try_into().unwrap_or(signer_count).max(1)))
        .unwrap_or(signer_count);

    let version: u8 = signer_state.get_expected_integer("hash_flag")?.try_into().unwrap();
    let hash_mode = AddressHashMode::from_version(version);

    let address = StacksAddress::from_public_keys(
        version,
        &hash_mode,
        required_signature_count.into(),
        &stacks_public_keys,
    )
    .unwrap();

    let nonce = match nonce {
        Some(nonce) => nonce,
        None => match signer_state.get_autoincremented_nonce(&construct_did.to_string()) {
            Some(value) => value.try_into().unwrap(),
            None => {
                let rpc = StacksRpc::new(&rpc_api_url, &rpc_api_auth_token);
                let nonce = rpc
                    .get_nonce(&address.to_string())
                    .await
                    .map_err(|e| diagnosed_error!("{}", e.to_string()))?;
                signer_state.set_autoincrementable_nonce(&construct_did.to_string(), nonce.into());
                nonce
            }
        },
    };

    let is_multisig = signer_state.get_expected_bool("multi_sig")?;

    let spending_condition = match is_multisig {
        true => TransactionSpendingCondition::Multisig(MultisigSpendingCondition {
            hash_mode: MultisigHashMode::P2SH,
            signer: address.bytes,
            nonce,
            tx_fee: fee,
            fields: vec![],
            signatures_required: required_signature_count,
        }),
        false => TransactionSpendingCondition::Singlesig(SinglesigSpendingCondition {
            hash_mode: SinglesigHashMode::P2PKH,
            signer: address.bytes,
            nonce,
            tx_fee: fee,
            key_encoding: TransactionPublicKeyEncoding::Compressed,
            signature: MessageSignature::empty(),
        }),
    };

    let auth = TransactionAuth::Standard(spending_condition);

    let mut unsigned_tx = StacksTransaction::new(transaction_version, auth, transaction_payload);
    unsigned_tx.chain_id = match transaction_version {
        TransactionVersion::Testnet => 0x80000000,
        TransactionVersion::Mainnet => 0x00000001,
    };

    let post_condition_mode = match post_condition_mode.expect_string() {
        "allow" => TransactionPostConditionMode::Allow,
        "deny" => TransactionPostConditionMode::Deny,
        _ => {
            return Err(diagnosed_error!(
                "Post condition mode {} unknown ('allow' or 'deny')",
                post_condition_mode.expect_string()
            ))
        }
    };
    unsigned_tx.post_condition_mode = post_condition_mode;
    for post_condition_bytes in post_conditions.iter() {
        let post_condition = match TransactionPostCondition::consensus_deserialize(
            &mut &post_condition_bytes.expect_buffer_bytes()[..],
        ) {
            Ok(res) => res,
            Err(e) => return Err(diagnosed_error!("invalid post-condition: ({})", e.to_string())),
        };
        unsigned_tx.post_conditions.push(post_condition);
    }
    Ok(unsigned_tx)
}
