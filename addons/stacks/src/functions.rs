use std::collections::BTreeMap;

use crate::{
    constants::{NAMESPACE, SIGNER},
    stacks_helpers::encode_any_value_to_clarity_value,
    typing::{DEPLOYMENT_ARTIFACTS_TYPE, STACKS_CV_ERR, STACKS_POST_CONDITIONS},
};
use clarity::vm::{
    errors::InterpreterError,
    types::{
        ASCIIData, BuffData, CharType, OptionalData, PrincipalData, QualifiedContractIdentifier,
        SequenceData, SequencedValue, UTF8Data,
    },
};
use clarity_repl::clarity::{codec::StacksMessageCodec, Value as ClarityValue};
use txtx_addon_kit::{
    indexmap::indexmap,
    types::{
        diagnostics::Diagnostic,
        functions::{
            arg_checker_with_ctx, fn_diag_with_ctx, FunctionImplementation, FunctionSpecification,
        },
        types::{Type, Value},
        AuthorizationContext,
    },
};

use crate::typing::StacksValue;
use crate::{
    codec::codec::{
        AssetInfo, FungibleConditionCode, NonfungibleConditionCode, PostConditionPrincipal,
        TransactionPostCondition,
    },
    stacks_helpers::{parse_clarity_value, value_to_tuple},
    typing::{
        STACKS_CV_BOOL, STACKS_CV_BUFFER, STACKS_CV_GENERIC, STACKS_CV_INT, STACKS_CV_NONE,
        STACKS_CV_OK, STACKS_CV_PRINCIPAL, STACKS_CV_SOME, STACKS_CV_STRING_ASCII,
        STACKS_CV_STRING_UTF8, STACKS_CV_TUPLE, STACKS_CV_UINT,
    },
};

pub fn arg_checker(fn_spec: &FunctionSpecification, args: &Vec<Value>) -> Result<(), Diagnostic> {
    let checker = arg_checker_with_ctx(NAMESPACE.to_string());
    checker(fn_spec, args)
}
pub fn to_diag(fn_spec: &FunctionSpecification, e: String) -> Diagnostic {
    let error_fn = fn_diag_with_ctx(NAMESPACE.to_string());
    error_fn(fn_spec, e)
}

pub fn serialize_cv_err(fn_spec: &FunctionSpecification, e: InterpreterError) -> Diagnostic {
    to_diag(fn_spec, format!("unable to serialize clarity value: {:?}", e))
}
pub fn cv_to_bytes(
    fn_spec: &FunctionSpecification,
    clarity_value: ClarityValue,
) -> Result<Vec<u8>, Diagnostic> {
    clarity_value.serialize_to_vec().map_err(|e| serialize_cv_err(fn_spec, e))
}

lazy_static! {
    pub static ref FUNCTIONS: Vec<FunctionSpecification> = vec![
        define_function! {
            EncodeClarityValueSome => {
                name: "cv_some",
                documentation: "`stacks::cv_some` wraps the given Clarity value in a Clarity `Optional`.",
                example: indoc! {r#"
                output "some" { 
                  value = stacks::cv_some(stacks::cv_bool(true))
                }
                // > some: 0x0a03
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "A Clarity Value.",
                        typing: vec![Type::addon(STACKS_CV_GENERIC)],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The input Clarity value wrapped in a Clarity `Optional`.",
                    typing: Type::addon(STACKS_CV_SOME)
                },
            }
        },
        define_function! {
            EncodeClarityValueNone => {
                name: "cv_none",
                documentation: "`stacks::cv_none` returns the Clarity value `None`.",
                example: indoc! {r#"
                output "none" { 
                  value = stacks::cv_none()
                }
                // > none: 0x09
                "#},
                inputs: [],
                output: {
                    documentation: "The Clarity value `None`.",
                    typing: Type::addon(STACKS_CV_NONE)
                },
            }
        },
        define_function! {
            EncodeClarityValueBool => {
                name: "cv_bool",
                documentation: "`stacks::cv_bool` returns the given boolean as a Clarity `bool`.",
                example: indoc! {r#"
                output "my_bool" { 
                  value = stacks::cv_bool(true)
                }
                // > my_bool: 0x03
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "The boolean values `true` or `false`.",
                        typing: vec![Type::bool()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The input boolean as a Clarity `bool`.",
                    typing: Type::addon(STACKS_CV_BOOL)
                },
            }
        },
        define_function! {
            EncodeClarityValueUint => {
                name: "cv_uint",
                documentation: "`stacks::cv_uint` returns the given number as a Clarity `uint`.",
                example: indoc! {r#"
                output "my_uint" { 
                  value = stacks::cv_uint(1)
                }
                // > my_uint: 0x0100000000000000000000000000000001
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "A positive integer between 0 and 2<sup>128</sup>-1.",
                        typing: vec![Type::integer()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The input integer as a Clarity `uint`.",
                    typing: Type::addon(STACKS_CV_UINT)
                },
            }
        },
        define_function! {
            EncodeClarityValueInt => {
                name: "cv_int",
                documentation: "`stacks::cv_int` returns the given number as a Clarity `int`.",
                example: indoc! {r#"
                output "my_int" { 
                  value = stacks::cv_int(-1)
                }
                // > my_int: 0x00FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "An integer between -2<sup>127</sup> and 2<sup>127</sup>-1.",
                        typing: vec![Type::integer()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The input integer as a Clarity `int`.",
                    typing: Type::addon(STACKS_CV_INT)
                },
            }
        },
        define_function! {
            EncodeClarityValuePrincipal => {
                name: "cv_principal",
                documentation: txtx_addon_kit::indoc! {r#"
                `stacks::cv_principal` returns the given string as a Clarity principal. 
                A Clarity principal represents a Stacks address on the blockchain.

                Clarity admits two different kinds of principals: _standard principals_ and _contract principals_. 
                Standard principals (e.g. `SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE`) are backed by a corresponding private key while contract principals (e.g. `ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.pyth-oracle-v1`) point to a smart contract.
                "#},
                example: indoc! {r#"
                output "my_principal" { 
                  value = stacks::cv_principal("SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE")
                }
                // > my_principal: 0x0516DEBC095099629BADB11B9D5335E874D12F1F1D45
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity principal string.",
                        typing: vec![Type::string()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The input string as a Clarity principal.",
                    typing: Type::addon(STACKS_CV_PRINCIPAL)
                },
            }
        },
        define_function! {
            EncodeClarityValueAscii => {
                name: "cv_string_ascii",
                documentation: "`stacks::cv_string_ascii` returns the given string as a Clarity ASCII string.",
                example: indoc! {r#"
                output "my_ascii" { 
                  value = stacks::cv_string_ascii("my ascii string")
                }
                // > my_ascii: 0x0D0000000F6D7920617363696920737472696E67
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "Any valid ASCII string.",
                        typing: vec![Type::string()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The input string as a Clarity ASCII string.",
                    typing: Type::addon(STACKS_CV_STRING_ASCII)
                },
            }
        },
        define_function! {
            EncodeClarityValueUTF8 => {
                name: "cv_string_utf8",
                documentation: "`stacks::cv_string_utf8` returns the given string as a Clarity UTF-8 string.",
                example: indoc! {r#"
                output "my_utf8" { 
                  value = stacks::cv_string_utf8("🍊")
                }
                // > my_utf8: 0x0E00000004F09F8D8A
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "Any valid UTF-8 string.",
                        typing: vec![Type::string()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The input string as a Clarity UTF-8 string.",
                    typing: Type::addon(STACKS_CV_STRING_UTF8)
                },
            }
        },
        define_function! {
            EncodeClarityValueTuple => {
                name: "cv_tuple",
                documentation: "`stacks::cv_tuple` returns the given object as a Clarity tuple.",
                example: indoc! {r#"
                output "my_tuple" { 
                  value = stacks::cv_tuple({ "key": stacks::cv_uint(1) })
                }
                // > my_tuple: 0x0C00000001036B65790100000000000000000000000000000001
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "An object where each key is a string and each value is a valid Clarity value.",
                        typing: vec![Type::object(vec![])],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The input object as a Clarity tuple.",
                    typing: Type::addon(STACKS_CV_TUPLE)
                },
            }
        },
        define_function! {
            EncodeClarityValueBuffer => {
                name: "cv_buff",
                documentation: "`stacks::cv_buff` returns the given hex string as a Clarity buffer.",
                example: indoc! {r#"
                output "my_buffer" { 
                  value = stacks::cv_buff("0x010203") 
                }
                // > my_buffer: 0x0200000003010203
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "A hex string.",
                        typing: vec![Type::string(), Type::buffer()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The input string as a Clarity buffer.",
                    typing: Type::addon(STACKS_CV_BUFFER)
                },
            }
        },
        define_function! {
            EncodeClarityValueOk => {
                name: "cv_ok",
                documentation: "`stacks::cv_ok` returns the given Clarity value wrapped in an `Ok` Clarity type.",
                example: indoc! {r#"
                output "ok" { 
                  value = stacks::cv_ok(1) 
                }
                // > ok: 0x070100000000000000000000000000000001
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value.",
                        typing: vec![Type::addon(STACKS_CV_GENERIC)],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The input wrapped in an `Ok` Clarity type.",
                    typing: Type::addon(STACKS_CV_OK)
                },
            }
        },
        define_function! {
            EncodeClarityValueErr => {
                name: "cv_err",
                documentation: "`stacks::cv_err` returns the given Clarity value wrapped in an `Err` Clarity type.",
                example: indoc! {r#"
                output "err" { 
                  value = stacks::cv_err(1) 
                }
                // > err: 0x080100000000000000000000000000000001
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "Any valid Clarity value.",
                        typing: vec![Type::addon(STACKS_CV_GENERIC)],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The input wrapped in an `Err` Clarity type.",
                    typing: Type::addon(STACKS_CV_ERR)
                },
            }
        },
        define_function! {
            RevertIfAccountSendingMoreThan => {
                name: "revert_if_account_sends_more_than",
                documentation: "`stacks::revert_if_account_sends_more_than` returns a post condition that will cancel a successfully executed transaction if the transaction results in the specified account sending more than the specified number of tokens. The default token is µSTX.",
                example: indoc! {r#"
                action "my_tx" "stacks::call_contract" {
                    contract_id = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.some-contract"
                    function_name = "some-function"
                    function_args = []
                    post_condition = [stacks::revert_if_account_sends_more_than("signer", 100)]
                }
                "#},
                inputs: [
                    account_address: {
                        documentation: indoc! {r#"The address of the account that the post condition will check. Use `"signer"` to apply this post condition to the transaction sender."#},
                        typing: vec![Type::string()],
                        optional: false
                    },
                    tokens_amount: {
                        documentation: "Threshold of tokens that triggers the revert action to prevent overspending.",
                        typing: vec![Type::integer()],
                        optional: false
                    },
                    token_id: {
                        documentation: "The asset identifier of the token to be checked. The default is µSTX if not provided.",
                        typing: vec![Type::string()],
                        optional: true
                    }
                ],
                output: {
                    documentation: "The post-condition, encoded as a buffer.",
                    typing: Type::addon(STACKS_POST_CONDITIONS)
                },
            }
        },
        define_function! {
            RevertIfAccountNotSending => {
                name: "revert_if_account_not_sending_exactly",
                documentation: "`stacks::revert_if_account_not_sending_exactly` returns a post condition that will cancel a successfully executed transaction if the transaction does not result in the specified account sending exactly the specified number of tokens. The default token is µSTX.",
                example: indoc! {r#"
                action "my_tx" "stacks::call_contract" {
                    contract_id = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.some-contract"
                    function_name = "some-function"
                    function_args = []
                    post_condition = [stacks::revert_if_account_not_sending_exactly("signer", 100)]
                }
                "#},
                inputs: [
                    account_address: {
                        documentation: indoc! {r#"The address of the account that the post condition will check. Use `"signer"` to apply this post condition to the transaction sender."#},
                        typing: vec![Type::string()],
                        optional: false
                    },
                    tokens_amount: {
                        documentation: "The number of tokens required to be sent.",
                        typing: vec![Type::integer()],
                        optional: false
                    },
                    token_id: {
                        documentation: "The asset identifier of the token to be checked. The default is µSTX if not provided.",
                        typing: vec![Type::string()],
                        optional: true
                    }
                ],
                output: {
                    documentation: "The post-condition, encoded as a buffer.",
                    typing: Type::addon(STACKS_POST_CONDITIONS)
                },
            }
        },
        define_function! {
            RevertIfAccountNotSendingAtLeast => {
                name: "revert_if_account_not_sending_at_least",
                documentation: "`stacks::revert_if_account_not_sending_at_least` returns a post condition that will cancel a successfully executed transaction if the transaction does not result in the specified account sending the minimum specified number of tokens. The default token is µSTX.",
                example: indoc! {r#"
                action "my_tx" "stacks::call_contract" {
                    contract_id = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.some-contract"
                    function_name = "some-function"
                    function_args = []
                    post_condition = [stacks::revert_if_account_not_sending_at_least("signer", 100)]
                }
                "#},
                inputs: [
                    account_address: {
                        documentation: indoc! {r#"The address of the account that the post condition will check. Use `"signer"` to apply this post condition to the transaction sender."#},
                        typing: vec![Type::string()],
                        optional: false
                    },
                    tokens_amount: {
                        documentation: "The minimum number of tokens required to be sent.",
                        typing: vec![Type::integer()],
                        optional: false
                    },
                    token_id: {
                        documentation: "The asset identifier of the token to be checked. The default is µSTX if not provided.",
                        typing: vec![Type::string()],
                        optional: true
                    }
                ],
                output: {
                    documentation: "The post-condition, encoded as a buffer.",
                    typing: Type::addon(STACKS_POST_CONDITIONS)
                },
            }
        },
        define_function! {
            RevertIfNFTNotOwnedByAccount => {
                name: "revert_if_nft_not_owned_by_account",
                documentation: "`stacks::revert_if_nft_not_owned_by_account` returns a post condition that will cancel a successfully executed transaction if the transaction does not result in the specified account owning a specific NFT.",
                example: indoc! {r#"
                action "my_tx" "stacks::call_contract" {
                    contract_id = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.some-contract"
                    function_name = "some-function"
                    function_args = []
                    post_condition = [
                        stacks::revert_if_nft_not_owned_by_account(
                            "ST2JHG361ZXG51QTKY2NQCVBPPRRE2KZB1HR05NNC", 
                            "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.some-contract", 
                            "nft_asset_id"
                        )
                    ]
                }
                "#},
                inputs: [
                    account_address: {
                        documentation: "The address of the account that the post condition will check.",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    contract_asset_id: {
                        documentation: "The NFT Contract Asset Id to check (<principal>.<contract_nam>::<non_fungible_storage>).",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    asset_id: {
                        documentation: "The NFT Asset Id to check.",
                        typing: vec![Type::string()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The post-condition, encoded as a buffer.",
                    typing: Type::addon(STACKS_POST_CONDITIONS)
                },
            }
        },
        define_function! {
            RevertIfNFTOwnedByAccount => {
                name: "revert_if_nft_owned_by_account",
                documentation: "`stacks::revert_if_nft_owned_by_account` returns a post condition that will cancel a successfully executed transaction if the transaction results in the specified account owning a specific NFT.",
                example: indoc! {r#"
                action "my_tx" "stacks::call_contract" {
                    contract_id = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.some-contract"
                    function_name = "some-function"
                    function_args = []
                    post_condition = [
                        stacks::revert_if_nft_owned_by_account(
                            "ST2JHG361ZXG51QTKY2NQCVBPPRRE2KZB1HR05NNC", 
                            "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.some-contract", 
                            "nft_asset_id"
                        )
                    ]
                }"#},
                inputs: [
                    account_address: {
                        documentation: "The address of the account that the post condition will check.",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    contract_asset_id: {
                        documentation: "The NFT Contract Asset Id to check (<principal>.<contract_name>::<non_fungible_storage>).",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    asset_id: {
                        documentation: "The NFT Asset Id to check.",
                        typing: vec![Type::string()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The post-condition, encoded as a buffer.",
                    typing: Type::addon(STACKS_POST_CONDITIONS)
                },
            }
        },
        define_function! {
            DecodeClarityValueResponse => {
                name: "decode_response",
                documentation: "`stacks::decode_response` returns the inner value of a Clarity Response as a Clarity buffer.",
                example: indoc! {r#"
                input "ok_uint" {
                    value = stacks::cv_ok(1)
                }
                output "ok" {
                    value = input.ok_uint
                }
                output "decoded_ok" {
                    value = stacks::decode_response(input.ok_uint)
                } 
                // > ok: 0x070100000000000000000000000000000001
                // > decoded_ok: 0x0100000000000000000000000000000001
                "#},
                inputs: [
                    clarity_value: {
                        documentation: "Any Clarity Response (`(ok <inner>)` or `(err <inner>)`) type.",
                        typing: vec![Type::addon(STACKS_CV_GENERIC)],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The inner value that was wrapped in a Clarity Response type.",
                    typing: Type::addon(STACKS_CV_GENERIC)
                },
            }
        },
        define_function! {
            RetrieveClarinetContract => {
                name: "get_contract_from_clarinet_project",
                documentation: "`stacks::get_contract_from_clarinet_project` retrieves the source of a contract present in a Clarinet manifest.",
                example: indoc! {r#"// Coming soon "#},
                inputs: [
                    clarinet_manifest_path: {
                        documentation: "The path of the Clarinet toml file.",
                        typing: vec![Type::string()],
                        optional: false
                    },
                    contract_name: {
                        documentation: "Contract name of the contract source to fetch.",
                        typing: vec![Type::string()],
                        optional: false
                    }
                ],
                output: {
                    documentation: "The source code of the contract",
                    typing: DEPLOYMENT_ARTIFACTS_TYPE.clone()
                },
            }
        },
    ];
}

pub struct EncodeClarityValueOk;
impl FunctionImplementation for EncodeClarityValueOk {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let arg = args.get(0).unwrap();
        let value =
            encode_any_value_to_clarity_value(arg).map_err(|e| to_diag(fn_spec, e.to_string()))?;
        let clarity_value =
            ClarityValue::okay(value).map_err(|e| to_diag(fn_spec, e.to_string()))?;
        let bytes = cv_to_bytes(fn_spec, clarity_value)?;
        Ok(StacksValue::ok(bytes))
    }
}

pub struct EncodeClarityValueErr;
impl FunctionImplementation for EncodeClarityValueErr {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let arg = args.get(0).unwrap();
        let value =
            encode_any_value_to_clarity_value(arg).map_err(|e| to_diag(fn_spec, e.to_string()))?;
        let clarity_value =
            ClarityValue::error(value).map_err(|e| to_diag(fn_spec, e.to_string()))?;
        let bytes = cv_to_bytes(fn_spec, clarity_value)?;
        Ok(StacksValue::err(bytes))
    }
}

#[derive(Clone)]
pub struct EncodeClarityValueSome;
impl FunctionImplementation for EncodeClarityValueSome {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let arg = args.get(0).unwrap();
        let value = encode_any_value_to_clarity_value(arg).map_err(|e| to_diag(fn_spec, e))?;
        let clarity_value =
            ClarityValue::some(value).map_err(|e| to_diag(fn_spec, e.to_string()))?;
        let bytes = cv_to_bytes(fn_spec, clarity_value)?;
        Ok(StacksValue::some(bytes))
    }
}

pub struct EncodeClarityValueNone;
impl FunctionImplementation for EncodeClarityValueNone {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        if !args.is_empty() {
            return Err(to_diag(fn_spec, "expected no arguments".into()));
        }
        let clarity_value = ClarityValue::Optional(OptionalData { data: None });
        let bytes = cv_to_bytes(fn_spec, clarity_value)?;
        Ok(StacksValue::none(bytes))
    }
}

pub struct EncodeClarityValueBool;
impl FunctionImplementation for EncodeClarityValueBool {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let entry = args.get(0).unwrap().as_bool().unwrap();
        let clarity_value = ClarityValue::Bool(entry);
        let bytes = cv_to_bytes(fn_spec, clarity_value)?;
        Ok(StacksValue::bool(bytes))
    }
}

pub struct EncodeClarityValueUint;
impl FunctionImplementation for EncodeClarityValueUint {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let entry = args
            .get(0)
            .unwrap()
            .as_integer()
            .and_then(|i| Some(u128::try_from(i)))
            .transpose()
            .map_err(|e| to_diag(fn_spec, format!("could not parse uint: {e}")))?
            .unwrap();

        let clarity_value = ClarityValue::UInt(u128::from(entry));
        let bytes = cv_to_bytes(fn_spec, clarity_value)?;
        Ok(StacksValue::uint(bytes))
    }
}

pub struct EncodeClarityValueInt;
impl FunctionImplementation for EncodeClarityValueInt {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let entry = args.get(0).unwrap().as_integer().unwrap();
        let clarity_value = ClarityValue::Int(i128::from(entry));
        let bytes = cv_to_bytes(fn_spec, clarity_value)?;
        Ok(StacksValue::int(bytes))
    }
}

pub struct EncodeClarityValuePrincipal;
impl FunctionImplementation for EncodeClarityValuePrincipal {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let arg = args.get(0).unwrap().as_string().unwrap();
        let principal = PrincipalData::parse(&arg).map_err(|e| to_diag(fn_spec, e.to_string()))?;
        let clarity_value = ClarityValue::Principal(principal);
        let bytes = cv_to_bytes(fn_spec, clarity_value)?;
        Ok(StacksValue::principal(bytes))
    }
}

pub struct EncodeClarityValueAscii;
impl FunctionImplementation for EncodeClarityValueAscii {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let arg = args.get(0).unwrap().as_string().unwrap();

        let clarity_value =
            ClarityValue::Sequence(SequenceData::String(CharType::ASCII(ASCIIData {
                data: arg.as_bytes().to_vec(),
            })));
        let bytes = cv_to_bytes(fn_spec, clarity_value)?;
        Ok(StacksValue::string_ascii(bytes))
    }
}

pub struct EncodeClarityValueUTF8;
impl FunctionImplementation for EncodeClarityValueUTF8 {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let arg = args.get(0).unwrap().as_string().unwrap();
        let clarity_value = UTF8Data::to_value(&arg.as_bytes().to_vec())
            .map_err(|e| to_diag(fn_spec, e.to_string()))?;
        let bytes = cv_to_bytes(fn_spec, clarity_value)?;
        Ok(StacksValue::string_utf8(bytes))
    }
}

pub struct EncodeClarityValueBuffer;
impl FunctionImplementation for EncodeClarityValueBuffer {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let data = args
            .get(0)
            .unwrap()
            .try_get_buffer_bytes_result()
            .map_err(|e| to_diag(fn_spec, e))?
            .unwrap();
        let clarity_value = ClarityValue::Sequence(SequenceData::Buffer(BuffData { data }));
        let bytes = cv_to_bytes(fn_spec, clarity_value)?;
        Ok(StacksValue::buffer(bytes))
    }
}

pub struct EncodeClarityValueTuple;
impl FunctionImplementation for EncodeClarityValueTuple {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let arg = args.get(0).unwrap();
        let clarity_value = ClarityValue::Tuple(value_to_tuple(arg));
        let bytes = cv_to_bytes(fn_spec, clarity_value)?;
        Ok(StacksValue::tuple(bytes))
    }
}

fn encode_ft_post_condition(
    address: &str,
    token_amount: i128,
    token_id: &str,
    condition: FungibleConditionCode,
) -> Result<TransactionPostCondition, String> {
    let principal_monitored =
        if address.eq(SIGNER) { PostConditionPrincipal::Origin } else { unimplemented!() };

    let Some((contract_id_specified, asset_name)) = token_id.split_once("::") else {
        unimplemented!()
    };

    let contract_id = QualifiedContractIdentifier::parse(contract_id_specified).unwrap();
    let asset_info = AssetInfo {
        contract_address: contract_id.issuer.into(),
        contract_name: contract_id.name,
        asset_name: asset_name.try_into().unwrap(),
    };

    let post_condition = TransactionPostCondition::Fungible(
        principal_monitored,
        asset_info,
        condition,
        token_amount as u64,
    );

    Ok(post_condition)
}

fn encode_nft_post_condition(
    address: &str,
    contract_asset_id: &str,
    asset_id_str: &str,
    condition: NonfungibleConditionCode,
) -> Result<TransactionPostCondition, String> {
    let principal_monitored = if address.eq("signer") {
        PostConditionPrincipal::Origin
    } else {
        match PrincipalData::parse(address)
            .map_err(|e| format!("unable to parse address: {}", e.to_string()))?
        {
            PrincipalData::Contract(contract) => {
                PostConditionPrincipal::Contract(contract.issuer.into(), contract.name.clone())
            }
            PrincipalData::Standard(contract) => PostConditionPrincipal::Standard(contract.into()),
        }
    };

    let Some((contract_id_specified, asset_name)) = contract_asset_id.split_once("::") else {
        unimplemented!()
    };

    let contract_id = QualifiedContractIdentifier::parse(contract_id_specified).unwrap();
    let asset_info = AssetInfo {
        contract_address: contract_id.issuer.into(),
        contract_name: contract_id.name,
        asset_name: asset_name.try_into().unwrap(),
    };

    let asset_id_value = Value::parse_and_default_to_string(asset_id_str);
    let asset_id = encode_any_value_to_clarity_value(&asset_id_value)?;

    let post_condition =
        TransactionPostCondition::Nonfungible(principal_monitored, asset_info, asset_id, condition);

    Ok(post_condition)
}

fn encode_stx_post_condition(
    address: &str,
    token_amount: i128,
    condition: FungibleConditionCode,
) -> Result<TransactionPostCondition, String> {
    let principal_monitored = if address.eq("signer") {
        PostConditionPrincipal::Origin
    } else {
        match PrincipalData::parse(address)
            .map_err(|e| format!("unable to parse address: {}", e.to_string()))?
        {
            PrincipalData::Contract(contract) => {
                PostConditionPrincipal::Contract(contract.issuer.into(), contract.name.clone())
            }
            PrincipalData::Standard(contract) => PostConditionPrincipal::Standard(contract.into()),
        }
    };

    let post_condition =
        TransactionPostCondition::STX(principal_monitored, condition, token_amount as u64);

    Ok(post_condition)
}

pub struct RevertIfAccountSendingMoreThan;
impl FunctionImplementation for RevertIfAccountSendingMoreThan {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let address = args.get(0).unwrap().as_string().unwrap();

        let token_amount = args.get(1).unwrap().as_integer().unwrap();

        let token_id_opt = match args.get(2) {
            Some(Value::String(val)) => Some(val),
            _ => None,
        };

        let post_condition_bytes = match token_id_opt {
            Some(token_id) => encode_ft_post_condition(
                address,
                token_amount,
                token_id,
                FungibleConditionCode::SentLe,
            )
            .map_err(|e| to_diag(fn_spec, e))?
            .serialize_to_vec(),
            None => encode_stx_post_condition(address, token_amount, FungibleConditionCode::SentLe)
                .map_err(|e| to_diag(fn_spec, e))?
                .serialize_to_vec(),
        };
        Ok(StacksValue::post_conditions(post_condition_bytes))
    }
}

pub struct RevertIfAccountNotSending;
impl FunctionImplementation for RevertIfAccountNotSending {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let address = args.get(0).unwrap().as_string().unwrap();

        let token_amount = args.get(1).unwrap().as_integer().unwrap();

        let token_id_opt = match args.get(2) {
            Some(Value::String(val)) => Some(val),
            _ => None,
        };

        let post_condition_bytes = match token_id_opt {
            Some(token_id) => encode_ft_post_condition(
                address,
                token_amount,
                token_id,
                FungibleConditionCode::SentEq,
            )
            .map_err(|e| to_diag(fn_spec, e))?
            .serialize_to_vec(),
            None => encode_stx_post_condition(address, token_amount, FungibleConditionCode::SentEq)
                .map_err(|e| to_diag(fn_spec, e))?
                .serialize_to_vec(),
        };
        Ok(StacksValue::post_conditions(post_condition_bytes))
    }
}

pub struct RevertIfAccountNotSendingAtLeast;
impl FunctionImplementation for RevertIfAccountNotSendingAtLeast {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let address = args.get(0).unwrap().as_string().unwrap();

        let token_amount = args.get(1).unwrap().as_integer().unwrap();

        let token_id_opt = match args.get(2) {
            Some(Value::String(val)) => Some(val),
            _ => None,
        };

        let post_condition_bytes = match token_id_opt {
            Some(token_id) => encode_ft_post_condition(
                address,
                token_amount,
                token_id,
                FungibleConditionCode::SentGe,
            )
            .map_err(|e| to_diag(fn_spec, e))?
            .serialize_to_vec(),
            None => encode_stx_post_condition(address, token_amount, FungibleConditionCode::SentGe)
                .map_err(|e| to_diag(fn_spec, e))?
                .serialize_to_vec(),
        };
        Ok(StacksValue::post_conditions(post_condition_bytes))
    }
}

pub struct RevertIfNFTNotOwnedByAccount;
impl FunctionImplementation for RevertIfNFTNotOwnedByAccount {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let address = args.get(0).unwrap().as_string().unwrap();

        let contract_asset_id = args.get(1).unwrap().as_string().unwrap();

        let asset_id = args.get(2).unwrap().as_string().unwrap();

        let post_condition_bytes = encode_nft_post_condition(
            address,
            contract_asset_id,
            asset_id,
            NonfungibleConditionCode::NotSent,
        )
        .map_err(|e| to_diag(fn_spec, e))?
        .serialize_to_vec();

        Ok(StacksValue::post_conditions(post_condition_bytes))
    }
}

pub struct RevertIfNFTOwnedByAccount;
impl FunctionImplementation for RevertIfNFTOwnedByAccount {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let address = args.get(0).unwrap().as_string().unwrap();

        let contract_asset_id = args.get(1).unwrap().as_string().unwrap();

        let asset_id = args.get(2).unwrap().as_string().unwrap();

        let post_condition_bytes = encode_nft_post_condition(
            address,
            contract_asset_id,
            asset_id,
            NonfungibleConditionCode::Sent,
        )
        .map_err(|e| to_diag(fn_spec, e))?
        .serialize_to_vec();

        Ok(StacksValue::post_conditions(post_condition_bytes))
    }
}

pub struct DecodeClarityValueResponse;
impl FunctionImplementation for DecodeClarityValueResponse {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let bytes = args
            .get(0)
            .unwrap()
            .try_get_buffer_bytes_result()
            .map_err(|e| to_diag(fn_spec, e))?
            .unwrap();
        let clarity_value =
            parse_clarity_value(&bytes, STACKS_CV_GENERIC).map_err(|e| to_diag(fn_spec, e))?;

        let ClarityValue::Response(response) = clarity_value else {
            return Err(to_diag(fn_spec, "expected Clarity Response type".into()));
        };
        let inner_bytes: Vec<u8> =
            response.data.serialize_to_vec().map_err(|e| serialize_cv_err(fn_spec, e))?;

        Ok(StacksValue::generic_clarity_value(inner_bytes))
    }
}

pub struct RetrieveClarinetContract;
impl FunctionImplementation for RetrieveClarinetContract {
    fn check_instantiability(
        _fn_spec: &FunctionSpecification,
        _auth_ctx: &AuthorizationContext,
        _args: &Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        fn_spec: &FunctionSpecification,
        auth_ctx: &AuthorizationContext,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        arg_checker(fn_spec, args)?;
        let clarinet_toml_path = args.get(0).unwrap().as_string().unwrap();
        let contract_key = args.get(1).unwrap().as_string().unwrap();

        let mut clarinet_manifest =
            auth_ctx.workspace_location.get_parent_location().map_err(|e| {
                to_diag(fn_spec, format!("invalid Clarinet.toml location ({})", e.to_string()))
            })?;
        let _ = clarinet_manifest.append_path(clarinet_toml_path);

        let manifest_bytes = clarinet_manifest.read_content().map_err(|e| {
            to_diag(fn_spec, format!("unable to read Clarinet.toml ({})", e.to_string()))
        })?;
        let manifest: ClarinetManifest = toml::from_slice(&manifest_bytes).map_err(|e| {
            to_diag(fn_spec, format!("unable to deserialize Clarinet.toml ({})", e.to_string()))
        })?;

        let mut contract_entry = None;
        for (contract_name, contract) in manifest.contracts.into_iter() {
            if contract_name.eq(&contract_key) {
                contract_entry = Some(contract.clone());
                break;
            }
        }

        let Some(contract) = contract_entry else {
            return Err(to_diag(
                fn_spec,
                format!("unable to locate contract with name {} in Clarinet.toml", contract_key),
            ));
        };

        let mut contract_location = clarinet_manifest.get_parent_location().map_err(|e| {
            to_diag(fn_spec, format!("invalid contract location ({})", e.to_string()))
        })?;
        let _ = contract_location.append_path(&contract.path);
        let contract_source = contract_location.read_content_as_utf8().map_err(|e| {
            to_diag(
                fn_spec,
                format!(
                    "unable to read contract at path {} ({})",
                    contract_location.to_string(),
                    e
                ),
            )
        })?;

        let res = Value::object(indexmap! {
            "contract_source".to_string() => Value::string(contract_source),
            "contract_name".to_string() => Value::string(contract_key.to_string()),
            "clarity_version".to_string() => Value::integer(contract.clarity_version as i128)
        });

        Ok(res)
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Contract {
    pub path: String,
    pub clarity_version: u64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub description: String,
    pub requirements: Option<Vec<RequirementConfig>>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct RequirementConfig {
    pub contract_id: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
struct ClarinetManifest {
    pub project: ProjectConfig,
    pub contracts: BTreeMap<String, Contract>,
}
