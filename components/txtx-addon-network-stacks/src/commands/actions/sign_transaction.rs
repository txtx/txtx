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
use txtx_addon_kit::types::frontend::{ActionItemRequest, BlockEvent};
use txtx_addon_kit::types::wallets::WalletInstance;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::AddonDefaults;

use crate::constants::{NETWORK_ID, PUBLIC_KEYS, SIGNED_TRANSACTION_BYTES};
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
        wallet_instances: &mut HashMap<ConstructUuid, WalletInstance>,
        execution_context: &CommandExecutionContext,
    ) -> Result<Vec<ActionItemRequest>, Diagnostic> {
        let wallet = get_wallet_mut(spec, args, wallet_instances)?;
        let transaction = build_unsigned_transaction(wallet, spec, args, defaults)?;

        let mut bytes = vec![];
        transaction.consensus_serialize(&mut bytes).unwrap(); // todo
        let hex_str = txtx_addon_kit::hex::encode(bytes); // todo
        let payload = Value::string(hex_str);

        (wallet.specification.check_signability)(
            uuid,
            "Sign Transaction",
            &payload,
            &wallet.specification,
            &args,
            &mut wallet.store,
            &defaults,
            execution_context,
        )
    }

    fn execute(
        uuid: &ConstructUuid,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        if let Ok(signed_transaction_bytes) = args.get_expected_value(SIGNED_TRANSACTION_BYTES) {
            let mut result = CommandExecutionResult::new();
            result.outputs.insert(
                SIGNED_TRANSACTION_BYTES.into(),
                signed_transaction_bytes.clone(),
            );
            return return_synchronous_result(Ok(result));
        }

        let wallet = get_wallet(spec, args, wallet_instances)?;
        let transaction = build_unsigned_transaction(wallet, spec, args, defaults)?;

        let mut bytes = vec![];
        transaction.consensus_serialize(&mut bytes).unwrap(); // todo
        let payload = Value::buffer(bytes, STACKS_SIGNED_TRANSACTION.clone());

        (wallet.specification.sign)(
            uuid,
            "Sign Transaction",
            &payload,
            &wallet.specification,
            &args,
            &wallet.store,
            &defaults,
        )
    }
}

fn get_wallet_mut<'a>(
    spec: &CommandSpecification,
    args: &ValueStore,
    wallet_instances: &'a mut HashMap<ConstructUuid, WalletInstance>,
) -> Result<&'a mut WalletInstance, Diagnostic> {
    let signer = args.get_expected_string("signer")?;
    let wallet_uuid = ConstructUuid::Local(Uuid::from_str(&signer).unwrap());
    let wallet = wallet_instances
        .get_mut(&wallet_uuid)
        .ok_or(Diagnostic::error_from_string(format!(
            "command '{}': wallet named '{}' not found",
            spec.matcher, &signer
        )))
        .unwrap();
    Ok(wallet)
}

fn get_wallet<'a>(
    spec: &CommandSpecification,
    args: &ValueStore,
    wallet_instances: &'a HashMap<ConstructUuid, WalletInstance>,
) -> Result<&'a WalletInstance, Diagnostic> {
    let signer = args.get_expected_string("signer")?;
    let wallet_uuid = ConstructUuid::Local(Uuid::from_str(&signer).unwrap());
    let wallet = wallet_instances
        .get(&wallet_uuid)
        .ok_or(Diagnostic::error_from_string(format!(
            "command '{}': wallet named '{}' not found",
            spec.matcher, &signer
        )))
        .unwrap();
    Ok(wallet)
}

fn build_unsigned_transaction(
    wallet: &WalletInstance,
    _spec: &CommandSpecification,
    args: &ValueStore,
    defaults: &AddonDefaults,
) -> Result<StacksTransaction, Diagnostic> {
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

    // Extract network_id
    let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;

    let transaction_version = match network_id.as_str() {
        "mainnet" => TransactionVersion::Mainnet,
        "testnet" => TransactionVersion::Testnet,
        _ => unimplemented!("invalid network_id, return diagnostic"),
    };

    let public_keys = wallet.store.get_expected_array(PUBLIC_KEYS)?;

    let stacks_public_keys: Vec<StacksPublicKey> = public_keys
        .iter()
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

    let mut unsigned_tx = StacksTransaction::new(transaction_version, auth, transaction_payload);
    if let TransactionVersion::Testnet = transaction_version {
        unsigned_tx.chain_id = 0x80000000;
    }

    Ok(unsigned_tx)
}
