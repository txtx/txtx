use crate::SVM_U64;

use super::*;
use anchor_lang_idl::types::{
    IdlAccount, IdlEvent, IdlInstruction, IdlInstructionAccount, IdlInstructionAccountItem,
};
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
