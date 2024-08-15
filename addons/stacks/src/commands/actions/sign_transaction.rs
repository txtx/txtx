use clarity::types::chainstate::{StacksAddress, StacksPublicKey};
use clarity::types::Address;
use clarity::util::secp256k1::MessageSignature;
use clarity::vm::{ClarityName, ContractName};
use clarity_repl::codec::{
    MultisigHashMode, MultisigSpendingCondition, SinglesigHashMode, SinglesigSpendingCondition,
    TransactionContractCall, TransactionPostCondition, TransactionPublicKeyEncoding,
};
use clarity_repl::{
    clarity::{address::AddressHashMode, codec::StacksMessageCodec},
    codec::{
        StacksTransaction, TransactionAuth, TransactionPayload, TransactionSpendingCondition,
        TransactionVersion,
    },
};
use std::collections::HashMap;
use txtx_addon_kit::types::commands::{
    CommandExecutionResult, CommandImplementation, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, Actions, BlockEvent,
    ReviewInputRequest,
};
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::wallets::{
    return_synchronous_ok, SigningCommandsState, WalletActionsFutureResult, WalletInstance,
    WalletSignFutureResult,
};
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructDid, ValueStore};
use txtx_addon_kit::AddonDefaults;

use crate::constants::{
    NETWORK_ID, PUBLIC_KEYS, RPC_API_URL, SIGNED_TRANSACTION_BYTES, TRANSACTION_PAYLOAD_BYTES,
    UNSIGNED_TRANSACTION_BYTES,
};

use crate::rpc::StacksRpc;
use crate::typing::StacksValue;

use super::get_signing_construct_did;

lazy_static! {
    pub static ref SIGN_STACKS_TRANSACTION: PreCommandSpecification = define_command! {
      SignStacksTransaction => {
          name: "Sign Stacks Transaction",
          matcher: "sign_transaction",
          documentation: "The `stacks::sign_transaction` action signs an encoded transaction payload with the supplied wallet data.",
          implements_signing_capability: true,
          implements_background_task_capability: false,
          inputs: [
            description: {
                documentation: "Description of the transaction",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            transaction_payload_bytes: {
                documentation: "The transaction payload bytes, encoded as a clarity buffer.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            network_id: {
                documentation: indoc!{r#"The network id, which is used to set the transaction version. Can be `"mainnet"`, `"testnet"` and `"devnet"`."#},
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            signer: {
                documentation: "A reference to a wallet construct, which will be used to sign the transaction payload.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            nonce: {
                documentation: "The account nonce of the signer. This value will be retrieved from the network if omitted.",
                typing: Type::integer(),
                optional: true,
                interpolable: true
            },
            fee: {
                documentation: "The transaction fee. This value will automatically be estimated if omitted.",
                typing: Type::integer(),
                optional: false,
                interpolable: true
            },
            fee_strategy: {
                documentation: "The strategy to use for automatically estimating fee ('low', 'medium', 'high'). Default to 'medium'.",
                typing: Type::string(),
                optional: true,
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
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        mut wallets: SigningCommandsState,
    ) -> WalletActionsFutureResult {
        use crate::constants::{ACTION_ITEM_CHECK_FEE, ACTION_ITEM_CHECK_NONCE};

        let signing_construct_did = get_signing_construct_did(args).unwrap();
        let wallet = wallets_instances
            .get(&signing_construct_did)
            .unwrap()
            .clone();
        let construct_did = construct_did.clone();
        let instance_name = instance_name.to_string();
        let spec = spec.clone();
        let args = args.clone();
        let defaults = defaults.clone();
        let supervision_context = supervision_context.clone();
        let wallets_instances = wallets_instances.clone();

        let future = async move {
            let mut actions = Actions::none();
            let mut signing_command_state = wallets
                .pop_signing_command_state(&signing_construct_did)
                .unwrap();
            if let Some(_) = signing_command_state
                .get_scoped_value(&construct_did.to_string(), SIGNED_TRANSACTION_BYTES)
            {
                return Ok((wallets, signing_command_state, Actions::none()));
            }

            let nonce = args.get_value("nonce").map(|v| v.expect_uint().unwrap());
            let fee = args.get_value("fee").map(|v| v.expect_uint().unwrap());
            let fee_strategy = args.get_string("fee_strategy");
            let post_conditions = match args.get_value("post_conditions") {
                Some(Value::Addon(v)) => vec![Value::Addon(v.clone())],
                Some(Value::Array(data)) => *data.clone(),
                _ => vec![],
            };
            let post_condition_mode = match args.get_value("post_condition_mode") {
                Some(Value::String(v)) => Value::string(v.into()),
                _ => Value::string("deny".into()),
            };

            let transaction = match build_unsigned_transaction(
                &construct_did,
                &mut signing_command_state,
                &spec,
                fee,
                fee_strategy,
                nonce,
                post_conditions,
                post_condition_mode,
                &args,
                &defaults,
            )
            .await
            {
                Ok(transaction) => transaction,
                Err(diag) => {
                    return Err((wallets, signing_command_state, diag));
                }
            };

            let mut bytes = vec![];
            transaction.consensus_serialize(&mut bytes).unwrap(); // todo
            let payload = StacksValue::transaction(bytes);

            signing_command_state.insert_scoped_value(
                &construct_did.to_string(),
                UNSIGNED_TRANSACTION_BYTES,
                payload.clone(),
            );
            wallets.push_signing_command_state(signing_command_state);
            let description = args
                .get_expected_string("description")
                .ok()
                .and_then(|d| Some(d.to_string()));

            if supervision_context.review_input_values {
                actions.push_group(
                    &description.clone().unwrap_or("".into()),
                    vec![
                        ActionItemRequest::new(
                            &Some(construct_did.clone()),
                            "".into(),
                            Some(format!("Check account nonce")),
                            ActionItemStatus::Todo,
                            ActionItemRequestType::ReviewInput(ReviewInputRequest {
                                input_name: "".into(),
                                value: Value::integer(transaction.get_origin_nonce() as i128),
                            }),
                            ACTION_ITEM_CHECK_NONCE,
                        ),
                        ActionItemRequest::new(
                            &Some(construct_did.clone()),
                            "µSTX".into(),
                            Some(format!("Check transaction fee")),
                            ActionItemStatus::Todo,
                            ActionItemRequestType::ReviewInput(ReviewInputRequest {
                                input_name: "".into(),
                                value: Value::integer(transaction.get_tx_fee() as i128),
                            }),
                            ACTION_ITEM_CHECK_FEE,
                        ),
                    ],
                )
            }

            let signing_command_state = wallets
                .pop_signing_command_state(&signing_construct_did)
                .unwrap();

            let (wallets, signing_command_state, mut wallet_actions) =
                (wallet.specification.check_signability)(
                    &construct_did,
                    &instance_name,
                    &description,
                    &payload,
                    &wallet.specification,
                    &args,
                    signing_command_state,
                    wallets,
                    &wallets_instances,
                    &defaults,
                    &supervision_context,
                )?;
            actions.append(&mut wallet_actions);
            Ok((wallets, signing_command_state, actions))
        };
        Ok(Box::pin(future))
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        mut wallets: SigningCommandsState,
    ) -> WalletSignFutureResult {
        let signing_construct_did = get_signing_construct_did(args).unwrap();
        let signing_command_state = wallets
            .pop_signing_command_state(&signing_construct_did)
            .unwrap();

        if let Ok(signed_transaction_bytes) = args.get_expected_value(SIGNED_TRANSACTION_BYTES) {
            let mut result = CommandExecutionResult::new();
            result.outputs.insert(
                SIGNED_TRANSACTION_BYTES.into(),
                signed_transaction_bytes.clone(),
            );
            return return_synchronous_ok(wallets, signing_command_state, result);
        }

        let wallet = wallets_instances.get(&signing_construct_did).unwrap();

        let payload = signing_command_state
            .get_scoped_value(&construct_did.to_string(), UNSIGNED_TRANSACTION_BYTES)
            .unwrap()
            .clone();

        let title = args
            .get_expected_string("description")
            .unwrap_or("New Transaction".into());

        let res = (wallet.specification.sign)(
            construct_did,
            title,
            &payload,
            &wallet.specification,
            &args,
            signing_command_state,
            wallets,
            wallets_instances,
            &defaults,
        );
        res
    }
}

#[cfg(not(feature = "wasm"))]
async fn build_unsigned_transaction(
    construct_did: &ConstructDid,
    signing_command_state: &mut ValueStore,
    _spec: &CommandSpecification,
    fee: Option<u64>,
    fee_strategy: Option<&str>,
    nonce: Option<u64>,
    post_conditions: Vec<Value>,
    post_condition_mode: Value,
    args: &ValueStore,
    defaults: &AddonDefaults,
) -> Result<StacksTransaction, Diagnostic> {
    // Extract and decode transaction_payload_bytes

    use clarity_repl::codec::TransactionPostConditionMode;

    use crate::constants::RPC_API_AUTH_TOKEN;
    let transaction_payload_bytes = args.get_expected_buffer_bytes(TRANSACTION_PAYLOAD_BYTES)?;
    let transaction_payload =
        match TransactionPayload::consensus_deserialize(&mut &transaction_payload_bytes[..]) {
            Ok(res) => res,
            Err(e) => {
                todo!(
                    "transaction payload invalid, return diagnostic ({})",
                    e.to_string()
                )
            }
        };
    let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;
    let default_payload = {
        let boot_address = match network_id.as_str() {
            "mainnet" => "SP000000000000000000002Q6VF78",
            "testnet" => "ST000000000000000000002AMW42H",
            "devnet" => "ST000000000000000000002AMW42H",
            _ => {
                return Err(diagnosed_error!(
                    "Network {} unknown ('mainnet', 'testnet' or 'devnet')",
                    network_id.as_str()
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

    let rpc_api_url = args.get_defaulting_string(RPC_API_URL, &defaults)?;
    let rpc_api_auth_token = args
        .get_defaulting_string(RPC_API_AUTH_TOKEN, defaults)
        .ok();

    let fee = match fee {
        Some(fee) => fee,
        None => {
            let fee_strategy = match fee_strategy {
                Some("low") => 0,
                Some("medium") => 1,
                Some("high") => 2,
                _ => 1,
            };
            let rpc = StacksRpc::new(&rpc_api_url, rpc_api_auth_token.clone());
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
    let transaction_version = match network_id.as_str() {
        "mainnet" => TransactionVersion::Mainnet,
        "testnet" => TransactionVersion::Testnet,
        "devnet" => TransactionVersion::Testnet,
        _ => {
            return Err(diagnosed_error!(
                "Network {} unknown ('mainnet', 'testnet' or 'devnet')",
                network_id.as_str()
            ))
        }
    };

    let public_keys = signing_command_state.get_expected_array(PUBLIC_KEYS)?;

    let stacks_public_keys: Vec<StacksPublicKey> = public_keys
        .iter()
        .map(|v| {
            let bytes = v.expect_buffer_bytes();
            StacksPublicKey::from_slice(&bytes[..])
                .map_err(|e| Diagnostic::error_from_string(e.to_string()))
        })
        .collect::<Result<Vec<StacksPublicKey>, Diagnostic>>()?;

    let version: u8 = signing_command_state
        .get_expected_integer("hash_flag")?
        .try_into()
        .unwrap();
    let hash_mode = AddressHashMode::from_version(version);

    let address = StacksAddress::from_public_keys(
        version,
        &hash_mode,
        stacks_public_keys.len(),
        &stacks_public_keys,
    )
    .unwrap();

    let nonce = match nonce {
        Some(nonce) => nonce,
        None => match signing_command_state.get_autoincremented_nonce(&construct_did.to_string()) {
            Some(value) => value.try_into().unwrap(),
            None => {
                let rpc = StacksRpc::new(&rpc_api_url, rpc_api_auth_token);
                let nonce = rpc
                    .get_nonce(&address.to_string())
                    .await
                    .map_err(|e| diagnosed_error!("{}", e.to_string()))?;
                signing_command_state
                    .set_autoincrementable_nonce(&construct_did.to_string(), nonce.into());
                nonce
            }
        },
    };

    let is_multisig = signing_command_state.get_expected_bool("multi_sig")?;

    let spending_condition = match is_multisig {
        true => TransactionSpendingCondition::Multisig(MultisigSpendingCondition {
            hash_mode: MultisigHashMode::P2SH,
            signer: address.bytes,
            nonce,
            tx_fee: fee,
            fields: vec![],
            signatures_required: stacks_public_keys.len() as u16,
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
            Err(e) => {
                return Err(diagnosed_error!(
                    "transaction payload invalid, return diagnostic ({})",
                    e.to_string()
                ))
            }
        };
        unsigned_tx.post_conditions.push(post_condition);
    }
    Ok(unsigned_tx)
}
