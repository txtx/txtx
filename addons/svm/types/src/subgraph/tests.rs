use crate::{subgraph::idl::parse_bytes_to_value_with_expected_idl_type_def_ty, SvmValue, SVM_U64};

use super::*;
use anchor_lang_idl::types::{
    IdlAccount, IdlEnumVariant, IdlEvent, IdlField, IdlGenericArg, IdlInstruction, IdlInstructionAccount, IdlInstructionAccountItem, IdlType, IdlTypeDefGeneric
};
use borsh::{BorshDeserialize, BorshSerialize};
use test_case::test_case;
use txtx_addon_kit::types::types::{ObjectProperty, ObjectType};

lazy_static! {
    pub static ref IDL: Idl =
        serde_json::from_slice(&include_bytes!("./fixtures/idl.json").to_vec()).unwrap();
    pub static ref ACCOUNT: IdlAccount = IDL.accounts.get(0).unwrap().clone();
    pub static ref ACCOUNT_TYPE: IdlTypeDef =
        IDL.types.iter().find(|t| t.name == "CustomAccount").unwrap().clone();
    pub static ref INSTRUCTION_1: IdlInstruction = IDL.instructions.get(0).unwrap().clone();
    pub static ref INSTRUCTION_2: IdlInstruction = IDL.instructions.get(1).unwrap().clone();
    pub static ref INSTRUCTION_1_ACCOUNT: IdlInstructionAccount =
        INSTRUCTION_1.accounts.iter().find_map(|a| match a {
            IdlInstructionAccountItem::Single(a) => if a.name == "custom" {
                Some(a.clone())
            } else {
                None
            },
            IdlInstructionAccountItem::Composite(_) => None,
        }).unwrap().clone();
    // pub static ref INSTRUCTION_2_ACCOUNT: IdlInstructionAccount =
    //     INSTRUCTION_2.accounts.iter().find(|a| a.name == "account_2").unwrap().clone();
    pub static ref EVENT: IdlEvent = IDL.events.get(0).unwrap().clone();
    pub static ref EVENT_TYPE: IdlTypeDef =
        IDL.types.iter().find(|t| t.name == "SplitTransferEvent").unwrap().clone();

    pub static ref PDA_SOURCE_TYPE: IndexedSubgraphSourceType = IndexedSubgraphSourceType::Pda(
        PdaSubgraphSource {
            account: ACCOUNT.clone(),
            account_type: ACCOUNT_TYPE.clone(),
            instruction_accounts: vec![(INSTRUCTION_1.clone(), INSTRUCTION_1_ACCOUNT.clone())]
        }
    );
    pub static ref EVENT_SOURCE_TYPE: IndexedSubgraphSourceType = IndexedSubgraphSourceType::Event(
        EventSubgraphSource {
            event: EVENT.clone(),
            ty: EVENT_TYPE.clone(),
        }
    );

    pub static ref PUBKEY: solana_pubkey::Pubkey = solana_pubkey::Pubkey::new_unique();
}

fn defined_field(
    name: Value,
    idl_key: Option<Value>,
    description: Option<Value>,
    is_indexed: Option<bool>,
) -> Value {
    let mut obj = ObjectType::from([("name", name)]);
    if let Some(idl_key) = idl_key {
        obj.insert("idl_key", idl_key);
    }
    if let Some(description) = description {
        obj.insert("description", description);
    }
    if let Some(is_indexed) = is_indexed {
        obj.insert("is_indexed", Value::bool(is_indexed));
    }
    obj.to_value()
}

#[test_case(
    PDA_SOURCE_TYPE.clone(),
    None;
    "pda source type with no defined fields"
)]
#[test_case(
    PDA_SOURCE_TYPE.clone(),
    Some(vec![
        ("bool", None, None, None),
        ("data", None, None, Some(true)),
        ("my_number", Some("u8".into()), None, None),
        ("my_u16", Some("u16".into()), Some("my u16 description".into()), None),
    ]);
    "pda source type with fields"
)]
#[test_case(
    EVENT_SOURCE_TYPE.clone(),
    None;
    "event source type with no defined fields"
)]
#[test_case(
    EVENT_SOURCE_TYPE.clone(),
    Some(vec![
        ("bool", None, None, None),
        ("data", None, None, Some(true)),
        ("my_number", Some("u8".into()), None, None),
        ("my_u16", Some("u16".into()), Some("my u16 description".into()), None),
    ]);
    "event source type with fields"
)]
fn test_parse_defined_fields(
    data_source: IndexedSubgraphSourceType,
    defined_fields_in: Option<Vec<(&str, Option<String>, Option<String>, Option<bool>)>>,
) {
    let defined_fields = defined_fields_in.as_ref().map(|fields| {
        fields
            .iter()
            .map(|(name, idl_key, description, is_indexed)| {
                defined_field(
                    Value::String(name.to_string()),
                    idl_key.clone().map(Value::String),
                    description.clone().map(Value::String),
                    is_indexed.clone(),
                )
            })
            .collect::<Vec<_>>()
    });

    let res = IndexedSubgraphField::parse_defined_field_values(
        data_source,
        &defined_fields,
        &IDL.types,
        &IDL.constants,
    )
    .unwrap();

    if let Some(defined_fields) = defined_fields_in.as_ref() {
        for (i, field) in res.iter().enumerate() {
            assert_eq!(res.len(), defined_fields.len());
            let (name, idl_key, description, is_indexed) = &defined_fields[i];

            assert_eq!(field.display_name, *name);
            if let Some(idl_key) = idl_key {
                assert_eq!(field.source_key, *idl_key);
            } else {
                assert_eq!(field.source_key, *name);
            }

            if let Some(description) = description {
                assert_eq!(field.description, Some(description.to_string()));
            } else {
                assert!(field.description.is_none());
            }

            assert_eq!(field.is_indexed, is_indexed.unwrap_or(false));
        }
    }

    for field in res.iter() {
        assert_field_expected_types(&field.source_key, field.expected_type.clone());
    }
}

// Hard coded test cases from the IDL
fn assert_field_expected_types(field_name: &str, expected_type: Type) {
    match field_name {
        "bool" => {
            assert_eq!(expected_type, Type::bool());
        }
        "u8" => {
            assert_eq!(expected_type, Type::addon(crate::SVM_U8));
        }
        "u16" => {
            assert_eq!(expected_type, Type::addon(crate::SVM_U16));
        }
        "u32" => {
            assert_eq!(expected_type, Type::addon(crate::SVM_U32));
        }
        "u64" => {
            assert_eq!(expected_type, Type::addon(crate::SVM_U64));
        }
        "u128" => {
            assert_eq!(expected_type, Type::addon(crate::SVM_U128));
        }
        "i8" => {
            assert_eq!(expected_type, Type::addon(crate::SVM_I8));
        }
        "i16" => {
            assert_eq!(expected_type, Type::addon(crate::SVM_I16));
        }
        "i32" => {
            assert_eq!(expected_type, Type::addon(crate::SVM_I32));
        }
        "i64" => {
            assert_eq!(expected_type, Type::addon(crate::SVM_I64));
        }
        "f32" => {
            assert_eq!(expected_type, Type::addon(crate::SVM_F32));
        }
        "f64" => {
            assert_eq!(expected_type, Type::addon(crate::SVM_F64));
        }
        "i128" => {
            assert_eq!(expected_type, Type::addon(crate::SVM_I128));
        }
        "string" => {
            assert_eq!(expected_type, Type::string());
        }
        "bytes" => {
            assert_eq!(expected_type, Type::buffer());
        }
        "pubkey" => {
            assert_eq!(expected_type, Type::addon(SVM_PUBKEY));
        }
        "data" => {
            assert_eq!(expected_type, Type::addon(SVM_U64));
        }
        "option" => {
            assert_eq!(expected_type, Type::addon(SVM_U64));
        }
        "additional_data" => {
            assert_eq!(
                expected_type,
                Type::object(ObjectDefinition::Strict(vec![
                    ObjectProperty {
                        name: "my_generic_field".into(),
                        documentation: "Generic field of type T".into(),
                        typing: Type::addon(crate::SVM_U64),
                        optional: false,
                        tainting: false,
                        internal: false
                    },
                    ObjectProperty {
                        name: "my_other_generic_field".into(),
                        documentation: "Generic field of type U".into(),
                        typing: Type::addon(crate::SVM_U32),
                        optional: false,
                        tainting: false,
                        internal: false
                    }
                ]))
            );
        }
        "array" => {
            assert_eq!(expected_type, Type::array(Type::addon(crate::SVM_U8)));
        }
        "wrapper_with_const" => {
            assert_eq!(
                expected_type,
                Type::object(ObjectDefinition::Strict(vec![ObjectProperty {
                    name: "data".into(),
                    documentation: "".into(),
                    typing: Type::array(Type::addon(crate::SVM_U8)),
                    optional: false,
                    tainting: false,
                    internal: false
                }]))
            );
        }
        "my_tuple_enum" => {
            assert_eq!(
                expected_type,
                Type::object(ObjectDefinition::Enum(vec![
                    ObjectProperty {
                        name: "UnitVariant".into(),
                        documentation: "".into(),
                        typing: Type::null(),
                        optional: false,
                        tainting: false,
                        internal: false,
                    },
                    ObjectProperty {
                        name: "NamedVariant".into(),
                        documentation: "".into(),
                        typing: Type::object(ObjectDefinition::Strict(vec![ObjectProperty {
                            name: "foo".into(),
                            documentation: "".into(),
                            typing: Type::addon(crate::SVM_U64),
                            optional: false,
                            tainting: false,
                            internal: false,
                        }])),
                        optional: false,
                        tainting: false,
                        internal: false,
                    },
                    ObjectProperty {
                        name: "TupleVariant".into(),
                        documentation: "".into(),
                        typing: Type::object(ObjectDefinition::Tuple(vec![
                            ObjectProperty {
                                name: "field_0".into(),
                                documentation: "".into(),
                                typing: Type::addon(crate::SVM_U8),
                                optional: false,
                                tainting: false,
                                internal: false,
                            },
                            ObjectProperty {
                                name: "field_1".into(),
                                documentation: "".into(),
                                typing: Type::string(),
                                optional: false,
                                tainting: false,
                                internal: false,
                            },
                        ])),
                        optional: false,
                        tainting: false,
                        internal: false,
                    },
                ]))
            );
        }
        "empty" => {
            assert_eq!(expected_type, Type::object(ObjectDefinition::strict(vec![])));
        }
        _ => {}
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
struct NamedStructAllTypes {
    bool_true: bool,
    bool_false: bool,
    u8: u8,
    u16: u16,
    u32: u32,
    u64: u64,
    u128: u128,
    i8: i8,
    i16: i16,
    i32: i32,
    i64: i64,
    i128: i128,
    f32: f32,
    f64: f64,
    bytes: Vec<u8>,
    string: String,
    opt_string_some: Option<String>,
    opt_string_none: Option<String>,
    vec: Vec<String>,
    // tuple: (u8, String),
    fixed: [u8; 3],
}

#[derive(BorshSerialize, BorshDeserialize)]
struct TupleStructAllTypes(
    bool,
    bool,
    u8,
    u16,
    u32,
    u64,
    u128,
    i8,
    i16,
    i32,
    i64,
    i128,
    f32,
    f64,
    Vec<u8>,
    String,
    Option<String>,
    Option<String>,
    Vec<String>,
    // (u8, String),
    [u8; 3],
);

#[derive(BorshSerialize, BorshDeserialize)]
enum MyEnum {
    UnitVariant,
    NamedVariant { foo: u64 },
    TupleVariant(u8, String),
}

#[test_case(borsh::to_vec(&true).unwrap(), IdlTypeDefTy::Type { alias: IdlType::Bool }, Value::bool(true); "bool true")]
#[test_case(borsh::to_vec(&false).unwrap(), IdlTypeDefTy::Type { alias: IdlType::Bool }, Value::bool(false); "bool false")]
#[test_case(borsh::to_vec(&(u8::MAX as u8)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::U8 }, SvmValue::u8(u8::MAX); "u8 max")]
#[test_case(borsh::to_vec(&(u8::MIN as u8)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::U8 }, SvmValue::u8(u8::MIN); "u8 min")]
#[test_case(borsh::to_vec(&(u16::MAX as u16)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::U16 }, SvmValue::u16(u16::MAX); "u16 max")]
#[test_case(borsh::to_vec(&(u16::MIN as u16)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::U16 }, SvmValue::u16(u16::MIN); "u16 min")]
#[test_case(borsh::to_vec(&(u32::MAX as u32)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::U32 }, SvmValue::u32(u32::MAX); "u32 max")]
#[test_case(borsh::to_vec(&(u32::MIN as u32)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::U32 }, SvmValue::u32(u32::MIN); "u32 min")]
#[test_case(borsh::to_vec(&(u64::MAX as u64)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::U64 }, SvmValue::u64(u64::MAX); "u64 max")]
#[test_case(borsh::to_vec(&(u64::MIN as u64)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::U64 }, SvmValue::u64(u64::MIN); "u64 min")]
#[test_case(borsh::to_vec(&(u128::MAX as u128)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::U128 }, SvmValue::u128(u128::MAX); "u128 max")]
#[test_case(borsh::to_vec(&(u128::MIN as u128)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::U128 }, SvmValue::u128(u128::MIN); "u128 min")]
#[test_case(borsh::to_vec(&(i8::MAX as i8)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::I8 }, SvmValue::i8(i8::MAX); "i8 max")]
#[test_case(borsh::to_vec(&(i8::MIN as i8)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::I8 }, SvmValue::i8(i8::MIN); "i8 min")]
#[test_case(borsh::to_vec(&(i16::MAX as i16)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::I16 }, SvmValue::i16(i16::MAX); "i16 max")]
#[test_case(borsh::to_vec(&(i16::MIN as i16)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::I16 }, SvmValue::i16(i16::MIN); "i16 min")]
#[test_case(borsh::to_vec(&(i32::MAX as i32)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::I32 }, SvmValue::i32(i32::MAX); "i32 max")]
#[test_case(borsh::to_vec(&(i32::MIN as i32)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::I32 }, SvmValue::i32(i32::MIN); "i32 min")]
#[test_case(borsh::to_vec(&(i64::MAX as i64)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::I64 }, SvmValue::i64(i64::MAX); "i64 max")]
#[test_case(borsh::to_vec(&(i64::MIN as i64)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::I64 }, SvmValue::i64(i64::MIN); "i64 min")]
#[test_case(borsh::to_vec(&(i128::MAX as i128)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::I128 }, SvmValue::i128(i128::MAX); "i128 max")]
#[test_case(borsh::to_vec(&(i128::MIN as i128)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::I128 }, SvmValue::i128(i128::MIN); "i128 min")]
#[test_case(borsh::to_vec(&1.0f32).unwrap(), IdlTypeDefTy::Type { alias: IdlType::F32 }, SvmValue::f32(1.0); "f32 pos 1.0")]
#[test_case(borsh::to_vec(&-1.0f32).unwrap(), IdlTypeDefTy::Type { alias: IdlType::F32 }, SvmValue::f32(-1.0); "f32 neg 1.0")]
#[test_case(borsh::to_vec(&1.0f64).unwrap(), IdlTypeDefTy::Type { alias: IdlType::F64 }, SvmValue::f64(1.0); "f64 pos 1.0")]
#[test_case(borsh::to_vec(&-1.0f64).unwrap(), IdlTypeDefTy::Type { alias: IdlType::F64 }, SvmValue::f64(-1.0); "f64 neg 1.0")]
#[test_case(borsh::to_vec(&None::<bool>).unwrap(), IdlTypeDefTy::Type { alias: IdlType::Option(Box::new(IdlType::Bool)) }, Value::null(); "option none")]
#[test_case(borsh::to_vec(&Some(true)).unwrap(), IdlTypeDefTy::Type { alias: IdlType::Option(Box::new(IdlType::Bool)) }, Value::bool(true); "option some")]
// #[test_case(borsh::to_vec(&PUBKEY).unwrap(), IdlTypeDefTy::Type { alias: IdlType::Pubkey }, SvmValue::pubkey(PUBKEY.to_bytes().to_vec()); "pubkey")]
#[test_case(
    borsh::to_vec(&b"hello world".to_vec()).unwrap(),
    IdlTypeDefTy::Type { alias: IdlType::Bytes },
    Value::buffer(b"hello world".to_vec());
    "bytes"
)]
#[test_case(
    borsh::to_vec(&"hello world".to_string()).unwrap(),
    IdlTypeDefTy::Type { alias: IdlType::String },
    Value::string("hello world".to_string());
    "short string"
)]
#[test_case(
    borsh::to_vec(&"a".repeat(1000)).unwrap(),
    IdlTypeDefTy::Type { alias: IdlType::String },
    Value::string("a".repeat(1000));
    "long string"
)]
#[test_case(
    borsh::to_vec(&NamedStructAllTypes {
        bool_true: true,
        bool_false: false,
        u8: 8,
        u16: 16,
        u32: 32,
        u64: 64,
        u128: 128,
        i8: -8,
        i16: -16,
        i32: -32,
        i64: -64,
        i128: -128,
        f32: 3.14,
        f64: 2.718281234234234,
        bytes: b"hello world".to_vec(),
        string: "hello world".to_string(),
        opt_string_some: Some("some string".to_string()),
        opt_string_none: None,
        vec: vec!["item1".to_string(), "item2".to_string()],
        // tuple: (42, "tuple string".to_string()),
        fixed: [1, 2, 3],
    }).unwrap(),
    IdlTypeDefTy::Struct { fields: Some(IdlDefinedFields::Named(vec![
        IdlField {
            name: "bool_true".to_string(),
            docs: vec![],
            ty: IdlType::Bool,
        },
        IdlField {
            name: "bool_false".to_string(),
            docs: vec![],
            ty: IdlType::Bool,
        },
        IdlField {
            name: "u8".to_string(),
            docs: vec![],
            ty: IdlType::U8,
        },
        IdlField {
            name: "u16".to_string(),
            docs: vec![],
            ty: IdlType::U16,
        },
        IdlField {
            name: "u32".to_string(),
            docs: vec![],
            ty: IdlType::U32,
        },
        IdlField {
            name: "u64".to_string(),
            docs: vec![],
            ty: IdlType::U64,
        },
        IdlField {
            name: "u128".to_string(),
            docs: vec![],
            ty: IdlType::U128,
        },
        IdlField {
            name: "i8".to_string(),
            docs: vec![],
            ty: IdlType::I8,
        },
        IdlField {
            name: "i16".to_string(),
            docs: vec![],
            ty: IdlType::I16,
        },
        IdlField {
            name: "i32".to_string(),
            docs: vec![],
            ty: IdlType::I32,
        },
        IdlField {
            name: "i64".to_string(),
            docs: vec![],
            ty: IdlType::I64,
        },
        IdlField {
            name: "i128".to_string(),
            docs: vec![],
            ty: IdlType::I128,
        },
        IdlField {
            name: "f32".to_string(),
            docs: vec![],
            ty: IdlType::F32,
        },
        IdlField {
            name: "f64".to_string(),
            docs: vec![],
            ty: IdlType::F64,
        },
        IdlField {
            name: "bytes".to_string(),
            docs: vec![],
            ty: IdlType::Bytes,
        },
        IdlField {
            name: "string".to_string(),
            docs: vec![],
            ty: IdlType::String,
        },
        IdlField {
            name: "opt_string_some".to_string(),
            docs: vec![],
            ty: IdlType::Option(Box::new(IdlType::String)),
        },
        IdlField {
            name: "opt_string_none".to_string(),
            docs: vec![],
            ty: IdlType::Option(Box::new(IdlType::String)),
        },
        IdlField {
            name: "vec".to_string(),
            docs: vec![],
            ty: IdlType::Vec(Box::new(IdlType::String)),
        },
        IdlField {
            name: "fixed".to_string(),
            docs: vec![],
            ty: IdlType::Array(Box::new(IdlType::U8),
                anchor_lang_idl::types::IdlArrayLen::Value(3)),
        },
    ])) },
    ObjectType::from([
        ("bool_true", Value::bool(true)),
        ("bool_false", Value::bool(false)),
        ("u8", SvmValue::u8(8)),
        ("u16", SvmValue::u16(16)),
        ("u32", SvmValue::u32(32)),
        ("u64", SvmValue::u64(64)),
        ("u128", SvmValue::u128(128)),
        ("i8", SvmValue::i8(-8)),
        ("i16", SvmValue::i16(-16)),
        ("i32", SvmValue::i32(-32)),
        ("i64", SvmValue::i64(-64)),
        ("i128", SvmValue::i128(-128)),
        ("f32", SvmValue::f32(3.14)),
        ("f64", SvmValue::f64(2.718281234234234)),
        ("bytes", Value::buffer(b"hello world".to_vec())),
        ("string", Value::string("hello world".to_string())),
        ("opt_string_some", Value::string("some string".to_string())),
        ("opt_string_none", Value::null()),
        ("vec", Value::array(vec![Value::string("item1".to_string()), Value::string("item2".to_string())])),
        ("fixed", Value::array(vec![SvmValue::u8(1), SvmValue::u8(2), SvmValue::u8(3)])),
    ]).to_value();
    "named struct with all types"
)]
#[test_case(
    borsh::to_vec(&TupleStructAllTypes(
        true,
        false,
        8,
        16,
        32,
        64,
        128,
        -8,
        -16,
        -32,
        -64,
        -128,
        3.14,
        2.718281234234234,
        b"hello world".to_vec(),
        "hello world".to_string(),
        Some("some string".to_string()),
        None,
        vec!["item1".to_string(), "item2".to_string()],
        // (42, "tuple string".to_string()),
        [1, 2, 3],
    )).unwrap(),
    IdlTypeDefTy::Struct { fields: Some(IdlDefinedFields::Tuple(vec![
        IdlType::Bool, 
        IdlType::Bool, 
        IdlType::U8, 
        IdlType::U16, 
        IdlType::U32, 
        IdlType::U64, 
        IdlType::U128, 
        IdlType::I8, 
        IdlType::I16, 
        IdlType::I32, 
        IdlType::I64, 
        IdlType::I128, 
        IdlType::F32, 
        IdlType::F64, 
        IdlType::Bytes, 
        IdlType::String,
        IdlType::Option(Box::new(IdlType::String)),
        IdlType::Option(Box::new(IdlType::String)),
        IdlType::Vec(Box::new(IdlType::String)),
        IdlType::Array(Box::new(IdlType::U8), anchor_lang_idl::types::IdlArrayLen::Value(3)),
    ])) },
    ObjectType::from([
        ("field_0", Value::bool(true)),
        ("field_1", Value::bool(false)),
        ("field_2", SvmValue::u8(8)),
        ("field_3", SvmValue::u16(16)),
        ("field_4", SvmValue::u32(32)),
        ("field_5", SvmValue::u64(64)),
        ("field_6", SvmValue::u128(128)),
        ("field_7", SvmValue::i8(-8)),
        ("field_8", SvmValue::i16(-16)),
        ("field_9", SvmValue::i32(-32)),
        ("field_10", SvmValue::i64(-64)),
        ("field_11", SvmValue::i128(-128)),
        ("field_12", SvmValue::f32(3.14)),
        ("field_13", SvmValue::f64(2.718281234234234)),
        ("field_14", Value::buffer(b"hello world".to_vec())),
        ("field_15", Value::string("hello world".to_string())),
        ("field_16", Value::string("some string".to_string())),
        ("field_17", Value::null()),
        ("field_18", Value::array(vec![Value::string("item1".to_string()), Value::string("item2".to_string())])),
        ("field_19", Value::array(vec![SvmValue::u8(1), SvmValue::u8(2), SvmValue::u8(3)])),
    ]).to_value();
    "tuple struct with all types"
)]
#[test_case(
    borsh::to_vec(&MyEnum::UnitVariant).unwrap(),
    IdlTypeDefTy::Enum { variants: vec![
        IdlEnumVariant { name: "UnitVariant".to_string(), fields: None },
        IdlEnumVariant { name: "NamedVariant".to_string(), fields: Some(IdlDefinedFields::Named(
            vec![IdlField { name: "foo".to_string(), docs: vec![], ty: IdlType::U64 }])) 
        },
        IdlEnumVariant { name: "TupleVariant".to_string(), fields: Some(IdlDefinedFields::Tuple(vec![
            IdlType::U8, IdlType::String
        ])) },
    ] },
    ObjectType::from([
        ("UnitVariant", Value::null())
    ]).to_value();
    "enum unit variant"
)]
#[test_case(
    borsh::to_vec(&MyEnum::NamedVariant { foo: u64::MAX }).unwrap(),
    IdlTypeDefTy::Enum { variants: vec![
        IdlEnumVariant { name: "UnitVariant".to_string(), fields: None },
        IdlEnumVariant { name: "NamedVariant".to_string(), fields: Some(IdlDefinedFields::Named(
            vec![IdlField { name: "foo".to_string(), docs: vec![], ty: IdlType::U64 }])) 
        },
        IdlEnumVariant { name: "TupleVariant".to_string(), fields: Some(IdlDefinedFields::Tuple(vec![
            IdlType::U8, IdlType::String
        ])) },
    ] },
    ObjectType::from([
        ("NamedVariant", ObjectType::from([
            ("foo", SvmValue::u64(u64::MAX)),
        ]).to_value())
    ]).to_value();
    "enum named variant"
)]
#[test_case(
    borsh::to_vec(&MyEnum::TupleVariant(u8::MAX, "hello world".to_string())).unwrap(),
    IdlTypeDefTy::Enum { variants: vec![
        IdlEnumVariant { name: "UnitVariant".to_string(), fields: None },
        IdlEnumVariant { name: "NamedVariant".to_string(), fields: Some(IdlDefinedFields::Named(
            vec![IdlField { name: "foo".to_string(), docs: vec![], ty: IdlType::U64 }])) 
        },
        IdlEnumVariant { name: "TupleVariant".to_string(), fields: Some(IdlDefinedFields::Tuple(vec![
            IdlType::U8, IdlType::String
        ])) },
    ] },
    ObjectType::from([
        ("TupleVariant", ObjectType::from([
            ("field_0", SvmValue::u8(u8::MAX)),
            ("field_1", Value::string("hello world".to_string())),
        ]).to_value())
    ]).to_value();
    "enum tuple variant"
)]
fn test_borsh_encoded_data(data: Vec<u8>, expected_type: IdlTypeDefTy, expected_value: Value) {
    let decoded =
        parse_bytes_to_value_with_expected_idl_type_def_ty(&data, &expected_type, &vec![], &vec![], &vec![]).unwrap();
    assert_eq!(decoded, expected_value, "Decoded value does not match expected value");
}


#[derive(BorshSerialize, BorshDeserialize)]
struct ParentStruct {
    child: ChildStruct,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct ChildStruct {
    field: u64,
}


#[derive(BorshSerialize, BorshDeserialize)]
pub struct ParentGenericStruct {
    pub my_generic_field: u64,
    pub nested: ChildGenericStruct<i32>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ChildGenericStruct<U> {
    pub my_other_generic_field: U,
}


#[test_case(
    borsh::to_vec(&ParentStruct {
        child: ChildStruct { field: 42 },
    }).unwrap(),
    IdlTypeDefTy::Struct {
        fields: Some(IdlDefinedFields::Named(vec![IdlField {
            name: "child".to_string(),
            docs: vec![],
            ty: IdlType::Defined { name: "ChildStruct".to_string(), generics: vec![] },
        }])),
    },
    ObjectType::from([("child", ObjectType::from([("field", SvmValue::u64(42))]).to_value())]).to_value(),
    vec![
        IdlTypeDef {
            name: "ChildStruct".to_string(),
            ty: IdlTypeDefTy::Struct{fields:Some(IdlDefinedFields::Named(vec![IdlField{name:"field".to_string(),docs:vec![],ty:IdlType::U64,}])),}, 
            docs: vec![], 
            serialization: anchor_lang_idl::types::IdlSerialization::Borsh, 
            repr: None, 
            generics: vec![]
        }
    ],
    vec![];
    "nested struct with child"
)]
#[test_case(
    borsh::to_vec(&
    ParentGenericStruct {
        my_generic_field: u64::MAX,
        nested: ChildGenericStruct { my_other_generic_field: i32::MIN },
    }).unwrap(),
    IdlTypeDefTy::Struct {  fields: 
        Some(IdlDefinedFields::Named(
            vec![
                IdlField {
                    name: "my_generic_field".to_string(),
                    docs: vec![],
                    ty: IdlType::U64,
                },
                IdlField {
                    name: "nested".to_string(),
                    docs: vec![],
                    ty: IdlType::Defined { 
                        name: "ChildGenericStruct".to_string(), 
                        generics: vec![IdlGenericArg::Type { ty: IdlType::I32 }] 
                    },
                },
            ]
        )),
    },
    ObjectType::from([("my_generic_field", SvmValue::u64(u64::MAX)), ("nested", ObjectType::from([("my_other_generic_field", SvmValue::i32(i32::MIN))]).to_value())]).to_value(),
    vec![
        IdlTypeDef {
            name: "ChildGenericStruct".to_string(),
            ty: IdlTypeDefTy::Struct{fields:Some(IdlDefinedFields::Named(vec![IdlField{name:"my_other_generic_field".to_string(),docs:vec![], ty:IdlType::Generic("U".to_string()),}])),}, 
            docs: vec![], 
            serialization: anchor_lang_idl::types::IdlSerialization::Borsh, 
            repr: None, 
            generics: vec![IdlTypeDefGeneric::Type { name: "U".into() }]
        }
    ],
    vec![];
    "struct with generic fields"
)]

fn test_borsh_encoded_data_with_additional_types(
    data: Vec<u8>, 
    expected_type: IdlTypeDefTy, 
    expected_value: Value,
    idl_types: Vec<IdlTypeDef>,
    idl_type_def_generics: Vec<IdlTypeDefGeneric>,  
) {
    let decoded =
        parse_bytes_to_value_with_expected_idl_type_def_ty(&data, &expected_type, &idl_types, &vec![], &idl_type_def_generics).unwrap();
    assert_eq!(decoded, expected_value, "Decoded value does not match expected value");
}


#[test]
fn rejects_leftover_bytes() {
    let str = "hello world".to_string();
    let utf8_bytes = str.as_bytes().to_vec();
    let wrong_len = (utf8_bytes.len() - 4) as u32;
    let len_bytes = wrong_len.to_le_bytes().to_vec();

    let data = [len_bytes, utf8_bytes].concat();
    let expected_type = IdlTypeDefTy::Type { alias: IdlType::String };
    let err =
        parse_bytes_to_value_with_expected_idl_type_def_ty(
            &data,
            &expected_type,
            &vec![],
            &vec![],
            &vec![]
        ).expect_err("Expected error for leftover bytes");
    assert_eq!(
        err,
        format!(
        "expected no leftover bytes after parsing type {:?}, but found {} bytes",
        expected_type,
        4)
    );
}


#[test]
fn error_enum_variant() {
    let mut data = borsh::to_vec(&MyEnum::NamedVariant { foo: u64::MAX }).unwrap();
    let expected_type = IdlTypeDefTy::Enum {
        variants: vec![
            IdlEnumVariant { name: "UnitVariant".to_string(), fields: None },
            IdlEnumVariant { name: "NamedVariant".to_string(), fields: Some(IdlDefinedFields::Named(
                vec![IdlField { name: "foo".to_string(), docs: vec![], ty: IdlType::U64 }])) 
            },
            IdlEnumVariant { name: "TupleVariant".to_string(), fields: Some(IdlDefinedFields::Tuple(vec![
                IdlType::U8, IdlType::String
            ])) },
        ],
    };

    let wrong_variant_idx = 4 as u32;
    let variant_bytes = u32::to_le_bytes(wrong_variant_idx);
    data.splice(0..4, variant_bytes.iter().cloned());
    let err =
        parse_bytes_to_value_with_expected_idl_type_def_ty(
            &data,
            &expected_type,
            &vec![],
            &vec![],
            &vec![]
        ).unwrap_err();
    assert_eq!(err, "invalid enum variant index: 4 for enum with 3 variants");
}



#[test_case(vec![], IdlTypeDefTy::Type { alias: IdlType::Bool }, "unable to decode bool: not enough bytes"; "bool not enough bytes")]
#[test_case(vec![], IdlTypeDefTy::Type { alias: IdlType::U8 }, "unable to decode u8: not enough bytes"; "u8 not enough bytes")]
#[test_case(vec![1], IdlTypeDefTy::Type { alias: IdlType::U16 }, "unable to decode u16: not enough bytes"; "u16 not enough bytes")]
#[test_case(vec![1], IdlTypeDefTy::Type { alias: IdlType::U32 }, "unable to decode u32: not enough bytes"; "u32 not enough bytes")]
#[test_case(vec![1], IdlTypeDefTy::Type { alias: IdlType::U64 }, "unable to decode u64: not enough bytes"; "u64 not enough bytes")]
#[test_case(vec![1], IdlTypeDefTy::Type { alias: IdlType::U128 }, "unable to decode u128: not enough bytes"; "u128 not enough bytes")]
#[test_case(vec![], IdlTypeDefTy::Type { alias: IdlType::I8 }, "unable to decode i8: not enough bytes"; "i8 not enough bytes")]
#[test_case(vec![1], IdlTypeDefTy::Type { alias: IdlType::I16 }, "unable to decode i16: not enough bytes"; "i16 not enough bytes")]
#[test_case(vec![1], IdlTypeDefTy::Type { alias: IdlType::I32 }, "unable to decode i32: not enough bytes"; "i32 not enough bytes")]
#[test_case(vec![1], IdlTypeDefTy::Type { alias: IdlType::I64 }, "unable to decode i64: not enough bytes"; "i64 not enough bytes")]
#[test_case(vec![1], IdlTypeDefTy::Type { alias: IdlType::I128 }, "unable to decode i128: not enough bytes"; "i128 not enough bytes")]
#[test_case(vec![1], IdlTypeDefTy::Type { alias: IdlType::F32 }, "unable to decode f32: not enough bytes"; "f32 not enough bytes")]
#[test_case(vec![1], IdlTypeDefTy::Type { alias: IdlType::F64 }, "unable to decode f64: not enough bytes"; "f64 not enough bytes")]
#[test_case(vec![1], IdlTypeDefTy::Type { alias: IdlType::String }, "unable to decode string length: not enough bytes"; "string length not enough bytes")]
#[test_case(vec![1], IdlTypeDefTy::Type { alias: IdlType::Bytes }, "unable to decode bytes length: not enough bytes"; "bytes length not enough bytes")]
#[test_case(vec![0,1,2,3], IdlTypeDefTy::Type { alias: IdlType::String }, "unable to decode string: not enough bytes"; "string not enough bytes")]
#[test_case(vec![0,1,2,3], IdlTypeDefTy::Type { alias: IdlType::Bytes }, "unable to decode bytes: not enough bytes"; "bytes not enough bytes")]
#[test_case(
    borsh::to_vec(&ParentStruct {
        child: ChildStruct { field: 42 },
    }).unwrap(),
    IdlTypeDefTy::Struct {
        fields: Some(IdlDefinedFields::Named(vec![IdlField {
            name: "child".to_string(),
            docs: vec![],
            ty: IdlType::Defined { name: "ChildStruct".to_string(), generics: vec![] },
        }])),
    }, "unable to decode ChildStruct: not found in IDL types"; "defined struct not found in IDL types"
)]
fn test_bad_data(bad_data: Vec<u8>, expected_type: IdlTypeDefTy, expected_err: &str) {
    let actual_err =
        parse_bytes_to_value_with_expected_idl_type_def_ty(&bad_data, &expected_type, &vec![], &vec![], &vec![]).unwrap_err();
    assert_eq!(actual_err, expected_err);
}
