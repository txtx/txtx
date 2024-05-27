use clarity_repl::clarity::stacks_common::types::chainstate::StacksAddress;
use clarity_repl::{
    clarity::{
        address::{
            AddressHashMode, C32_ADDRESS_VERSION_MAINNET_SINGLESIG,
            C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
        },
        codec::StacksMessageCodec,
        util::secp256k1::{MessageSignature, Secp256k1PrivateKey, Secp256k1PublicKey},
        vm::types::PrincipalData,
        ClarityVersion, ContractName,
    },
    codec::{
        SinglesigHashMode, SinglesigSpendingCondition, StacksString, StacksTransaction,
        StacksTransactionSigner, TokenTransferMemo, TransactionAnchorMode, TransactionAuth,
        TransactionPayload, TransactionPostConditionMode, TransactionPublicKeyEncoding,
        TransactionSmartContract, TransactionSpendingCondition, TransactionVersion,
    },
};
use libsecp256k1::{PublicKey, SecretKey};
use std::{collections::HashMap, str::FromStr};
use tiny_hderive::bip32::ExtendedPrivKey;
use txtx_addon_kit::types::commands::{
    return_synchronous_ok, CommandExecutionFutureResult, CommandInstance, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::ActionItem;
use txtx_addon_kit::types::ConstructUuid;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::{PrimitiveValue, Type, Value},
};
use txtx_addon_kit::AddonDefaults;

use crate::typing::STACKS_SIGNED_TRANSACTION;

lazy_static! {
    pub static ref SIGN_STACKS_TRANSACTION: PreCommandSpecification = define_command! {
      SignStacksTransaction => {
          name: "Sign Stacks Transaction",
          matcher: "sign_transaction",
          documentation: "The `sign_transaction` action signs an encoded transaction payload with the supplied wallet data.",
          inputs: [
            nonce: {
                documentation: "The transaction nonce.",
                typing: Type::uint(),
                optional: false,
                interpolable: true
            },
            transaction_payload_bytes: {
                documentation: "The transaction payload bytes, encoded as a clarity buffer.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            fee: {
                documentation: "The transaction fee.",
                typing: Type::uint(),
                optional: false,
                interpolable: true
            },
            sender_mnemonic: {
                documentation: "The wallet mnemonic that will be used to generate a private key that will be used to sign the transaction.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            sender_derivation_path: {
              documentation: "The derivation path that will be used to generate a private key that will be used to sign the transaction.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            network_id: {
                documentation: indoc!{r#"The network id, which is used to set the transaction version. Can be `"testnet"` or `"mainnet"`."#},
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

impl CommandImplementation for SignStacksTransaction {
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
        todo!()
    }
    fn run(
        ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        defaults: &AddonDefaults,
    ) -> CommandExecutionFutureResult {
        let mut result = CommandExecutionResult::new();
        // Extract nonce
        let nonce = match args.get("nonce") {
            Some(Value::Primitive(PrimitiveValue::UnsignedInteger(value))) => value.clone(),
            _ => todo!("return diagnostic"),
        };
        // Extract tx_fee
        let tx_fee = match args.get("fee") {
            Some(Value::Primitive(PrimitiveValue::UnsignedInteger(value))) => value.clone(),
            _ => todo!("return diagnostic"),
        };
        // Extract mnemonic
        let mnemonic = match args.get("sender_mnemonic") {
            Some(Value::Primitive(PrimitiveValue::String(value))) => value.clone(),
            _ => todo!("return diagnostic"),
        };
        // Extract derivation path
        let derivation_path = match args.get("sender_derivation_path") {
            Some(Value::Primitive(PrimitiveValue::String(value))) => value.clone(),
            _ => todo!("return diagnostic"),
        };

        let wallet = Wallet {
            mnemonic,
            derivation_path,
        };

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

        // Sign
        let signed_transaction = match sign_transaction_payload(
            &wallet,
            transaction_payload,
            nonce,
            tx_fee,
            TransactionAnchorMode::OffChainOnly, // todo(lgalabru)
            &transaction_version,                // todo(lgalabru)
        ) {
            Ok(res) => res,
            Err(_e) => {
                todo!("return diagnostic")
            }
        };

        let mut bytes = vec![];
        signed_transaction.consensus_serialize(&mut bytes).unwrap();
        let value = Value::buffer(bytes, STACKS_SIGNED_TRANSACTION.clone());

        result
            .outputs
            .insert("signed_transaction_bytes".to_string(), value);
        result
            .outputs
            .insert("network_id".to_string(), Value::string(network_id));

        return_synchronous_ok(result)
    }
}

#[derive(Debug, Clone)]
struct Wallet {
    pub mnemonic: String,
    pub derivation_path: String,
}

fn sign_transaction_payload(
    wallet: &Wallet,
    payload: TransactionPayload,
    nonce: u64,
    tx_fee: u64,
    anchor_mode: TransactionAnchorMode,
    network: &TransactionVersion,
) -> Result<StacksTransaction, String> {
    let (_, secret_key, public_key) = get_keypair(wallet);
    let signer_addr = get_stacks_address(&public_key, network);

    let spending_condition = TransactionSpendingCondition::Singlesig(SinglesigSpendingCondition {
        signer: signer_addr.bytes,
        nonce,
        tx_fee,
        hash_mode: SinglesigHashMode::P2PKH,
        key_encoding: TransactionPublicKeyEncoding::Compressed,
        signature: MessageSignature::empty(),
    });

    let auth = TransactionAuth::Standard(spending_condition);
    let unsigned_tx = StacksTransaction {
        version: network.clone(),
        chain_id: match network {
            TransactionVersion::Mainnet => 0x00000001,
            _ => 0x80000000,
        },
        auth,
        anchor_mode,
        post_condition_mode: TransactionPostConditionMode::Allow,
        post_conditions: vec![],
        payload,
    };

    let mut unsigned_tx_bytes = vec![];
    unsigned_tx
        .consensus_serialize(&mut unsigned_tx_bytes)
        .expect("FATAL: invalid transaction");

    let mut tx_signer = StacksTransactionSigner::new(&unsigned_tx);
    tx_signer.sign_origin(&secret_key).unwrap();
    let signed_tx = tx_signer.get_tx().unwrap();
    Ok(signed_tx)
}

fn _encode_stx_transfer(
    recipient: PrincipalData,
    amount: u64,
    memo: [u8; 34],
    wallet: &Wallet,
    nonce: u64,
    tx_fee: u64,
    anchor_mode: TransactionAnchorMode,
    network: &TransactionVersion,
) -> Result<StacksTransaction, String> {
    let payload = TransactionPayload::TokenTransfer(recipient, amount, TokenTransferMemo(memo));
    sign_transaction_payload(wallet, payload, nonce, tx_fee, anchor_mode, network)
}

fn _encode_contract_publish(
    contract_name: &ContractName,
    source: &str,
    clarity_version: Option<ClarityVersion>,
    wallet: &Wallet,
    nonce: u64,
    tx_fee: u64,
    anchor_mode: TransactionAnchorMode,
    network: &TransactionVersion,
) -> Result<StacksTransaction, String> {
    let payload = TransactionSmartContract {
        name: contract_name.clone(),
        code_body: StacksString::from_str(source).unwrap(),
    };
    sign_transaction_payload(
        wallet,
        TransactionPayload::SmartContract(payload, clarity_version),
        nonce,
        tx_fee,
        anchor_mode,
        network,
    )
}

pub fn get_bip39_seed_from_mnemonic(mnemonic: &str, password: &str) -> Result<Vec<u8>, String> {
    use hmac::Hmac;
    use pbkdf2::pbkdf2;
    use sha2::Sha512;

    const PBKDF2_ROUNDS: u32 = 2048;
    const PBKDF2_BYTES: usize = 64;
    let salt = format!("mnemonic{}", password);
    let mut seed = vec![0u8; PBKDF2_BYTES];

    pbkdf2::<Hmac<Sha512>>(
        mnemonic.as_bytes(),
        salt.as_bytes(),
        PBKDF2_ROUNDS,
        &mut seed,
    )
    .map_err(|e| e.to_string())?;
    Ok(seed)
}

fn get_keypair(wallet: &Wallet) -> (ExtendedPrivKey, Secp256k1PrivateKey, PublicKey) {
    let bip39_seed = match get_bip39_seed_from_mnemonic(&wallet.mnemonic, "") {
        Ok(bip39_seed) => bip39_seed,
        Err(_) => panic!(),
    };
    let ext = ExtendedPrivKey::derive(&bip39_seed[..], wallet.derivation_path.as_str()).unwrap();
    let wrapped_secret_key: Secp256k1PrivateKey =
        Secp256k1PrivateKey::from_slice(&ext.secret()).unwrap();
    let secret_key = SecretKey::parse_slice(&ext.secret()).unwrap();
    let public_key = PublicKey::from_secret_key(&secret_key);
    (ext, wrapped_secret_key, public_key)
}

fn get_stacks_address(public_key: &PublicKey, network: &TransactionVersion) -> StacksAddress {
    let wrapped_public_key =
        Secp256k1PublicKey::from_slice(&public_key.serialize_compressed()).unwrap();

    StacksAddress::from_public_keys(
        match network {
            TransactionVersion::Mainnet => C32_ADDRESS_VERSION_MAINNET_SINGLESIG,
            _ => C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
        },
        &AddressHashMode::SerializeP2PKH,
        1,
        &vec![wrapped_public_key],
    )
    .unwrap()
}
