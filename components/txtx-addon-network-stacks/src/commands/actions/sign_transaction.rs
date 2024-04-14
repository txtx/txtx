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
use serde_json::Value as JsonValue;
use std::{collections::HashMap, str::FromStr};
use tiny_hderive::bip32::ExtendedPrivKey;
use txtx_addon_kit::types::{
    commands::{
        CommandExecutionResult, CommandImplementation, CommandInputsEvaluationResult,
        CommandSpecification,
    },
    diagnostics::Diagnostic,
    types::{PrimitiveValue, Type, Value},
};

use crate::typing::STACKS_SIGNED_TRANSACTION;

lazy_static! {
  pub static ref SIGN_STACKS_TRANSACTION: CommandSpecification = define_command! {
    SignStacksTransaction => {
        name: "Sign Stacks Transaction",
        matcher: "sign_transaction",
        documentation: "Sign an encoded transaction payload",
        inputs: [
            no_interact: {
                documentation: "Any valid Clarity value",
                typing: define_object_type! [
                    nonce: {
                        documentation: "Transaction nonce",
                        typing: Type::uint(),
                        optional: false,
                        interpolable: true
                    },
                    transaction_payload_bytes: {
                        documentation: "Transaction payload",
                        typing: Type::string(),
                        optional: false,
                        interpolable: true
                    },
                    fee: {
                        documentation: "Transaction fee",
                        typing: Type::uint(),
                        optional: false,
                        interpolable: true
                    },
                    sender_mnemonic: {
                        documentation: "Mnemonic",
                        typing: Type::string(),
                        optional: false,
                        interpolable: true
                    },
                    sender_derivation_path: {
                        documentation: "Derivation path",
                        typing: Type::string(),
                        optional: false,
                        interpolable: true
                    }

                ],
                optional: true,
                interpolable: true
            },
            cli_interact: {
                documentation: "Any valid Clarity value",
                typing: define_object_type! [], // todo
                optional: true,
                interpolable: true
            },
            web_interact: {
                documentation: "Any valid Clarity value",
                typing: define_object_type! [
                  transaction_payload_bytes: {
                      documentation: "The encoded transaction bytes to be signed.",
                      typing: Type::buffer(),
                      optional: false,
                      interpolable: true
                  },
                  signed_transaction_bytes: {
                      documentation: "The signed transaction bytes.",
                      typing: Type::buffer(),
                      optional: true,
                      interpolable: true
                  },
                  nonce: {
                      documentation: "The nonce of the address signing the transaction.",
                      typing: Type::uint(),
                      optional: true,
                      interpolable: true
                  }
                ],
                optional: true,
                interpolable: true
            }
        ],
        outputs: [
            signed_transaction_bytes: {
                documentation: "The signed transaction bytes.",
                typing: Type::string()
            }
        ],
    }
  };
}

pub struct SignStacksTransaction;
impl CommandImplementation for SignStacksTransaction {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut result = CommandExecutionResult::new();
        if let Some(Value::Object(obj)) = args.get("no_interact") {
            // Extract nonce
            let nonce = match obj.get("nonce") {
                Some(Ok(Value::Primitive(PrimitiveValue::UnsignedInteger(value)))) => value.clone(),
                _ => todo!("return diagnostic"),
            };
            // Extract tx_fee
            let tx_fee = match obj.get("fee") {
                Some(Ok(Value::Primitive(PrimitiveValue::UnsignedInteger(value)))) => value.clone(),
                _ => todo!("return diagnostic"),
            };
            // Extract mnemonic
            let mnemonic = match obj.get("sender_mnemonic") {
                Some(Ok(Value::Primitive(PrimitiveValue::String(value)))) => value.clone(),
                _ => todo!("return diagnostic"),
            };
            // Extract derivation path
            let derivation_path = match obj.get("sender_derivation_path") {
                Some(Ok(Value::Primitive(PrimitiveValue::String(value)))) => value.clone(),
                _ => todo!("return diagnostic"),
            };

            let wallet = Wallet {
                mnemonic,
                derivation_path,
            };

            // Extract and decode transaction_payload_bytes
            let transaction_payload_bytes = match obj.get("transaction_payload_bytes") {
                Some(Ok(Value::Primitive(PrimitiveValue::Buffer(bytes)))) => bytes.clone(),
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
            // Sign
            let signed_transaction = match sign_transaction_payload(
                &wallet,
                transaction_payload,
                nonce,
                tx_fee,
                TransactionAnchorMode::OffChainOnly, // todo(lgalabru)
                &TransactionVersion::Mainnet,        // todo(lgalabru)
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
        } else if let Some(web_interact_inputs) = args.get("web_interact") {
            let bytes = match web_interact_inputs {
                Value::Object(obj) => obj.get("signed_transaction_bytes"),
                _ => unimplemented!(),
            };
            match bytes {
                Some(bytes) => match bytes {
                    Ok(bytes) => {
                        result
                            .outputs
                            .insert("signed_transaction_bytes".to_string(), bytes.clone());
                    }
                    Err(e) => return Err(e.clone()),
                },
                None => {}
            }
        }

        Ok(result)
    }

    fn update_input_evaluation_results_from_user_input(
        ctx: &CommandSpecification,
        current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        input_name: String,
        value: String,
    ) {
        let (input_key, value) = match input_name.as_str() {
            "description" => {
                let description_input =
                    ctx.inputs.iter().find(|i| i.name == "description").expect(
                        "Sign Stacks Transaction specification must have description input",
                    );

                let expected_type = description_input.typing.clone();
                let value = if value.is_empty() {
                    None
                } else {
                    Some(Value::from_string(value, expected_type, None))
                };
                (description_input, value)
            }
            "web_interact" => {
                let mut object_values = HashMap::new();
                let web_interact_input =
                    ctx.inputs.iter().find(|i| i.name == "web_interact").expect(
                        "Sign Stacks Transaction specification must have a web_interact input",
                    );
                let web_interact_input_object = web_interact_input
                    .as_object()
                    .expect("Sign Stacks Transaction web interact input must be and object.");

                let value_json: JsonValue = match serde_json::from_str(&value) {
                    Ok(value) => value,
                    Err(_e) => unimplemented!(), // todo: return diagnostic
                };
                let value_json = value_json.as_object().unwrap(); // todo

                let transaction_signature_property = web_interact_input_object.iter().find(|p| p.name == "signed_transaction_bytes").expect("Sign Stacks Transaction specification's web_interact input should have a signed_transaction_bytes property.");
                let expected_type = transaction_signature_property.typing.clone();
                let value = match value_json.get("signed_transaction_bytes") {
                    Some(value) => Some(Value::from_string(
                        value.as_str().unwrap().to_string(),
                        expected_type,
                        Some(STACKS_SIGNED_TRANSACTION.clone()),
                    )),
                    None => None,
                };
                match value {
                    Some(value) => {
                        object_values.insert(transaction_signature_property.name.clone(), value);
                    }
                    None => {}
                };

                let nonce_property = web_interact_input_object.iter().find(|p| p.name == "nonce").expect("Send Stacks Transaction specification's web_interact input should have a nonce property.");
                let nonce_expected_type = nonce_property.typing.clone();
                let nonce_val = match value_json.get("nonce") {
                    Some(value) => Some(Value::from_string(
                        value.to_string(),
                        nonce_expected_type,
                        None,
                    )),
                    None => None,
                };
                match nonce_val {
                    Some(value) => {
                        object_values.insert(nonce_property.name.clone(), value);
                    }
                    None => {}
                };

                let result = Some(Ok(Value::Object(object_values)));
                (web_interact_input, result)
            }
            _ => unimplemented!("cannot parse serialized output for input {input_name}"),
        };
        match value {
            Some(value) => current_input_evaluation_result
                .inputs
                .insert(input_key.clone(), value),
            None => current_input_evaluation_result.inputs.remove(&input_key),
        };
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
