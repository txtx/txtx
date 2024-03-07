use std::{collections::HashMap, str::FromStr};

use clarity_repl::clarity::stacks_common::types::chainstate::StacksAddress;
use clarity_repl::clarity::Value as ClarityValue;
use clarity_repl::{
    clarity::{
        address::{
            AddressHashMode, C32_ADDRESS_VERSION_MAINNET_SINGLESIG,
            C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
        },
        codec::StacksMessageCodec,
        util::secp256k1::{MessageSignature, Secp256k1PrivateKey, Secp256k1PublicKey},
        vm::types::{PrincipalData, QualifiedContractIdentifier},
        ClarityName, ClarityVersion, ContractName,
    },
    codec::{
        SinglesigHashMode, SinglesigSpendingCondition, StacksString, StacksTransaction,
        StacksTransactionSigner, TokenTransferMemo, TransactionAnchorMode, TransactionAuth,
        TransactionContractCall, TransactionPayload, TransactionPostConditionMode,
        TransactionPublicKeyEncoding, TransactionSmartContract, TransactionSpendingCondition,
        TransactionVersion,
    },
};
use libsecp256k1::{PublicKey, SecretKey};
use tiny_hderive::bip32::ExtendedPrivKey;
use txtx_addon_kit::types::{
    commands::{
        CommandExecutionResult, CommandImplementation, CommandInputsEvaluationResult,
        CommandSpecification,
    },
    diagnostics::Diagnostic,
    types::{PrimitiveType, PrimitiveValue, Type, Value},
};

use crate::typing::{STACKS_CONTRACT_CALL, STACKS_SIGNED_TRANSACTION};

lazy_static! {
    pub static ref STACKS_COMMANDS: Vec<CommandSpecification> = vec![
        define_command! {
            EncodeStacksContractCall => {
                name: "Stacks Contract Call",
                matcher: "call_contract",
                documentation: "Encode contract call payload",
                inputs: [
                    description: {
                        documentation: "Description of the variable",
                        typing: Type::string(),
                        optional: true,
                        interpolable: true
                    },
                    contract_id: {
                        documentation: "Contract identifier to invoke",
                        typing: Type::string(),
                        optional: false,
                        interpolable: true
                    },
                    function_name: {
                        documentation: "Method to invoke",
                        typing: Type::string(),
                        optional: false,
                        interpolable: true
                    },
                    args: {
                        documentation: "Args to provide",
                        typing: Type::string(),
                        optional: true,
                        interpolable: true
                    }
                ],
                outputs: [
                    bytes: {
                        documentation: "Encoded contract call",
                        typing: Type::buffer()
                    }
                ],
            }
        },
        define_command! {
            StacksDeployContract => {
                name: "Stacks Contract Deployment",
                matcher: "deploy_contract",
                documentation: "Encode contract deployment payload",
                inputs: [
                    description: {
                        documentation: "Description of the variable",
                        typing: Type::string(),
                        optional: true,
                        interpolable: true
                    },
                    clarity_value: {
                        documentation: "Any valid Clarity value",
                        typing: Type::bool(),
                        optional: true,
                        interpolable: true
                    }
                ],
                outputs: [
                    bytes: {
                        documentation: "Encoded contract call",
                        typing: Type::buffer()
                    }
                ],
            }
        },
        define_command! {
            EncodeStacksTransaction => {
                name: "Stacks Transaction",
                matcher: "transaction",
                documentation: "Encode contract deployment payload",
                inputs: [
                    description: {
                        documentation: "Description of the variable",
                        typing: Type::string(),
                        optional: true,
                        interpolable: true
                    },
                    no_interact: {
                        documentation: "Any valid Clarity value",
                        typing: define_object_type! [
                            nonce: {
                                documentation: "Transaction nonce",
                                typing: PrimitiveType::UnsignedInteger,
                                optional: false,
                                interpolable: true
                            },
                            transaction_payload_bytes: {
                                documentation: "Transaction payload",
                                typing: PrimitiveType::String,
                                optional: false,
                                interpolable: true
                            },
                            fee: {
                                documentation: "Transaction fee",
                                typing: PrimitiveType::UnsignedInteger,
                                optional: false,
                                interpolable: true
                            },
                            sender_mnemonic: {
                                documentation: "Mnemonic",
                                typing: PrimitiveType::String,
                                optional: false,
                                interpolable: true
                            },
                            sender_derivation_path: {
                                documentation: "Derivation path",
                                typing: PrimitiveType::String,
                                optional: false,
                                interpolable: true
                            }

                        ],
                        optional: true,
                        interpolable: true
                    },
                    cli_interact: {
                        documentation: "Any valid Clarity value",
                        typing: define_object_type! [
                            nonce: {
                                documentation: "Nonce of the transaction",
                                typing: PrimitiveType::UnsignedInteger,
                                optional: false,
                                interpolable: true
                            },
                            payload_bytes: {
                                documentation: "Transaction payload",
                                typing: PrimitiveType::UnsignedInteger,
                                optional: false,
                                interpolable: true
                            }
                        ],
                        optional: true,
                        interpolable: true
                    },
                    web_interact: {
                        documentation: "Any valid Clarity value",
                        typing: define_object_type! [
                            nonce: {
                                documentation: "Nonce of the transaction",
                                typing: PrimitiveType::UnsignedInteger,
                                optional: false,
                                interpolable: true
                            },
                            payload_bytes: {
                                documentation: "Transaction payload",
                                typing: PrimitiveType::UnsignedInteger,
                                optional: false,
                                interpolable: true
                            }
                        ],
                        optional: true,
                        interpolable: true
                    }
                ],
                outputs: [
                    bytes: {
                        documentation: "Encoded transaction",
                        typing: Type::buffer()
                    }
                ],
            }
        },
        define_command! {
            SignStacksTransaction => {
                name: "Sign Stacks Transaction",
                matcher: "sign_transaction",
                documentation: "Sign an encoded transaction payload",
                inputs: [
                    description: {
                        documentation: "A description of the transaction being signed.",
                        typing: Type::string(),
                        optional: true,
                        interpolable: true
                    },
                    no_interact: {
                        documentation: "Any valid Clarity value",
                        typing: define_object_type! [], // todo
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
                          encoded_bytes: {
                              documentation: "The encoded transaction bytes to be signed.",
                              typing: PrimitiveType::UnsignedInteger,
                              optional: false,
                              interpolable: true
                          },
                          signed_transaction_bytes: {
                              documentation: "The signed transaction bytes.",
                              typing: PrimitiveType::UnsignedInteger,
                              optional: true,
                              interpolable: true
                          }
                        ],
                        optional: true,
                        interpolable: true
                    }
                ],
                outputs: [
                    bytes: {
                        documentation: "The signed transaction",
                        typing: Type::string()
                    }
                ],
            }
        },
    ];
}

pub struct StacksCallContract;
impl CommandImplementation for StacksCallContract {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let value = args.get("contract_id").unwrap().clone(); // todo(lgalabru): get default, etc.
        let mut result = CommandExecutionResult::new();
        result.outputs.insert("bytes".to_string(), value);
        Ok(result)
    }

    fn update_input_evaluation_results_from_user_input(
        _ctx: &CommandSpecification,
        _current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String,
        _value: String,
    ) {
        todo!()
    }
}

pub struct StacksDeployContract;
impl CommandImplementation for StacksDeployContract {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let value = args.get("value").unwrap().clone(); // todo(lgalabru): get default, etc.
        let mut result = CommandExecutionResult::new();
        result.outputs.insert("bytes".to_string(), value);
        Ok(result)
    }

    fn update_input_evaluation_results_from_user_input(
        _ctx: &CommandSpecification,
        _current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String,
        _value: String,
    ) {
        todo!()
    }
}

pub struct EncodeStacksContractCall;
impl CommandImplementation for EncodeStacksContractCall {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut result = CommandExecutionResult::new();

        // Extract contract_id
        let contract_id = match args.get("contract_id") {
            Some(Value::Primitive(PrimitiveValue::String(value))) => {
                match QualifiedContractIdentifier::parse(value) {
                    Ok(value) => value,
                    _ => todo!("contract_id invalid, return diagnostic"),
                }
            }
            _ => todo!("contract_id missing, return diagnostic"),
        };
        // Extract derivation path
        let function_name = match args.get("function_name") {
            Some(Value::Primitive(PrimitiveValue::String(value))) => value.clone(),
            _ => todo!("function_name missing, return diagnostic"),
        };

        let payload = TransactionPayload::ContractCall(TransactionContractCall {
            contract_name: contract_id.name.clone(),
            address: StacksAddress::from(contract_id.issuer.clone()),
            function_name: ClarityName::try_from(function_name).unwrap(),
            function_args: vec![],
        });

        let mut bytes = vec![];
        payload.consensus_serialize(&mut bytes).unwrap();
        let value = Value::buffer(bytes, STACKS_CONTRACT_CALL.clone());

        result.outputs.insert("bytes".to_string(), value);

        println!("==> {:?}", result);
        Ok(result)
    }

    fn update_input_evaluation_results_from_user_input(
        _ctx: &CommandSpecification,
        _current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String,
        _value: String,
    ) {
        todo!()
    }
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
        let bytes = match args.get("web_interact") {
            Some(web_interact_inputs) => match web_interact_inputs {
                Value::Object(obj) => obj.get("signed_transaction_bytes"),
                _ => unimplemented!(),
            },
            None => None,
        };
        match bytes {
            Some(bytes) => match bytes {
                Ok(bytes) => {
                    result
                        .outputs
                        .insert("bytes".to_string(), Value::Primitive(bytes.clone()));
                }
                Err(e) => return Err(e.clone()),
            },
            None => {}
        }

        println!("==> {:?}", result);

        Ok(result)
    }

    fn update_input_evaluation_results_from_user_input(
        _ctx: &CommandSpecification,
        _current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String,
        _value: String,
    ) {
        todo!()
    }
}

pub struct EncodeStacksTransaction;
impl CommandImplementation for EncodeStacksTransaction {
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
                Some(Ok(PrimitiveValue::UnsignedInteger(value))) => value.clone(),
                _ => todo!("return diagnostic"),
            };
            // Extract tx_fee
            let tx_fee = match obj.get("fee") {
                Some(Ok(PrimitiveValue::UnsignedInteger(value))) => value.clone(),
                _ => todo!("return diagnostic"),
            };
            // Extract mnemonic
            let mnemonic = match obj.get("sender_mnemonic") {
                Some(Ok(PrimitiveValue::String(value))) => value.clone(),
                _ => todo!("return diagnostic"),
            };
            // Extract derivation path
            let derivation_path = match obj.get("sender_derivation_path") {
                Some(Ok(PrimitiveValue::String(value))) => value.clone(),
                _ => todo!("return diagnostic"),
            };

            let wallet = Wallet {
                mnemonic,
                derivation_path,
            };

            // Extract and decode transaction_payload_bytes
            let transaction_payload_bytes = match obj.get("transaction_payload_bytes") {
                Some(Ok(PrimitiveValue::Buffer(bytes))) => bytes.clone(),
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
                Err(e) => {
                    todo!("return diagnostic")
                }
            };

            let mut bytes = vec![];
            signed_transaction.consensus_serialize(&mut bytes).unwrap();
            let value = Value::buffer(bytes, STACKS_SIGNED_TRANSACTION.clone());

            result.outputs.insert("bytes".to_string(), value);
        };

        println!("==> {:?}", result);
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
                let description_input = ctx
                    .inputs
                    .iter()
                    .find(|i| i.name == "description")
                    .expect("Variable specification must have description input");

                let expected_type = description_input.typing.clone();
                let value = if value.is_empty() {
                    None
                } else {
                    Some(Value::from_string(value, expected_type))
                };
                (description_input, value)
            }
            "web_interact:signed_transaction_bytes" => {
                let mut object_values = HashMap::new();
                let web_interact_input =
                    ctx.inputs.iter().find(|i| i.name == "web_interact").expect(
                        "Sign Stacks Transaction specification must have a web_interact input",
                    );
                let web_interact_input_object = web_interact_input
                    .as_object()
                    .expect("Sign Stacks Transaction web interact input must be and object.");
                let transaction_signature_property = web_interact_input_object.iter().find(|p| p.name == "signed_transaction_bytes").expect("Sign Stacks Transaction specification's web_interact input should have a signed_transaction_bytes property.");
                let expected_type = transaction_signature_property.typing.clone();
                let value = if value.is_empty() {
                    None
                } else {
                    Some(PrimitiveValue::from_string(value, expected_type))
                };
                let object_values = match value {
                    Some(value) => {
                        object_values.insert(transaction_signature_property.name.clone(), value);
                        Some(Ok(Value::Object(object_values)))
                    }
                    None => None,
                };
                (web_interact_input, object_values)
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum StacksNetwork {
    Testnet,
    Mainnet,
}

#[derive(Debug, Clone)]
pub struct Wallet {
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

pub fn encode_stx_transfer(
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

pub fn encode_contract_publish(
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
