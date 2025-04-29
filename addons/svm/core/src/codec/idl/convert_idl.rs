use spl_token::solana_program;
use txtx_addon_kit::types::diagnostics::Diagnostic;

use crate::typing::anchor as anchor_lang_idl;

use anchor_lang_idl::types::Idl as AnchorIdl;
use anchor_lang_idl::types::IdlAccount as AnchorIdlAccount;
use anchor_lang_idl::types::IdlArrayLen as AnchorIdlArrayLen;
use anchor_lang_idl::types::IdlConst as AnchorIdlConst;
use anchor_lang_idl::types::IdlDefinedFields as AnchorIdlDefinedFields;
use anchor_lang_idl::types::IdlEnumVariant as AnchorIdlEnumVariant;
use anchor_lang_idl::types::IdlErrorCode as AnchorIdlErrorCode;
use anchor_lang_idl::types::IdlEvent as AnchorIdlEvent;
use anchor_lang_idl::types::IdlField as AnchorIdlField;
use anchor_lang_idl::types::IdlInstruction as AnchorIdlInstruction;
use anchor_lang_idl::types::IdlInstructionAccount as AnchorIdlInstructionAccount;
use anchor_lang_idl::types::IdlInstructionAccountItem as AnchorIdlInstructionAccountItem;
use anchor_lang_idl::types::IdlInstructionAccounts as AnchorIdlInstructionAccounts;
use anchor_lang_idl::types::IdlMetadata as AnchorIdlMetadata;
use anchor_lang_idl::types::IdlSerialization as AnchorIdlSerialization;
use anchor_lang_idl::types::IdlType as AnchorIdlType;
use anchor_lang_idl::types::IdlTypeDef as AnchorIdlTypeDef;
use anchor_lang_idl::types::IdlTypeDefTy as AnchorIdlTypeDefTy;
use solana_idl::EnumFields as ClassicEnumFields;
use solana_idl::Idl as ClassicIdl;
use solana_idl::IdlAccountItem as ClassicIdlAccountItem;
use solana_idl::IdlConst as ClassicIdlConst;
use solana_idl::IdlEvent as ClassicIdlEvent;
use solana_idl::IdlField as ClassicIdlField;
use solana_idl::IdlInstruction as ClassicIdlInstruction;
use solana_idl::IdlType as ClassicIdlType;
use solana_idl::IdlTypeDefinition as ClassicIdlTypeDefinition;
use solana_idl::IdlTypeDefinitionTy as ClassicIdlTypeDefinitionTy;

pub fn classic_idl_to_anchor_idl(classic_idl: ClassicIdl) -> Result<AnchorIdl, Diagnostic> {
    let idl_serializer = match &classic_idl.metadata {
        Some(metadata) => {
            match metadata.serializer.as_ref().and_then(|s| Some::<&str>(s.as_ref())) {
                Some("bytemuck") => AnchorIdlSerialization::Bytemuck,
                Some("bytemuckunsafe") => AnchorIdlSerialization::BytemuckUnsafe,
                Some("borsh") | None => AnchorIdlSerialization::Borsh,
                Some(ser) => AnchorIdlSerialization::Custom(ser.to_string()),
            }
        }
        None => AnchorIdlSerialization::Borsh,
    };

    let mut instructions = vec![];
    let mut type_defs = vec![];
    let mut tuple_idx = 0;
    for instruction in classic_idl.instructions.iter() {
        let (anchor_instruction, anchor_type_defs) = classic_instruction_to_anchor_instruction(
            &instruction,
            &classic_idl.types,
            &idl_serializer,
            &mut tuple_idx,
        )?;
        instructions.push(anchor_instruction);
        type_defs.extend(anchor_type_defs);
    }

    let mut constants = vec![];
    for classic_const in classic_idl.constants.iter() {
        let (anchor_const, anchor_type_defs) = classic_const_to_anchor_const(
            &classic_const,
            &classic_idl.types,
            &idl_serializer,
            &mut tuple_idx,
        )?;
        constants.push(anchor_const);
        type_defs.extend(anchor_type_defs);
    }

    classic_idl_type_defs_to_anchor_type_defs(
        &mut type_defs,
        &classic_idl.types,
        &idl_serializer,
        &mut tuple_idx,
    )
    .map_err(|e| {
        diagnosed_error!("failed to convert classic idl types to anchor idl types: {e}")
    })?;

    let address = classic_idl
        .metadata
        .clone()
        .ok_or(diagnosed_error!("missing metadata in classic IDL, cannot convert to anchor IDL"))?
        .address
        .ok_or(diagnosed_error!(
            "missing address in classic IDL metadata, cannot convert to anchor IDL"
        ))?;
    let idl = AnchorIdl {
        address,
        metadata: classic_idl_to_anchor_metadata(&classic_idl),
        docs: vec![],
        instructions,
        accounts: classic_idl
            .accounts
            .iter()
            .map(|a| classic_account_to_anchor_account(a))
            .collect(),
        errors: classic_idl
            .errors
            .unwrap_or_default()
            .iter()
            .map(|e| AnchorIdlErrorCode { code: e.code, name: e.name.clone(), msg: e.msg.clone() })
            .collect(),
        types: type_defs,
        constants,
        events: classic_idl
            .events
            .map(|events| events.iter().map(|event| classic_event_to_anchor_event(event)).collect())
            .unwrap_or_default(),
    };

    Ok(idl)
}

fn classic_idl_to_anchor_metadata(classic_idl: &ClassicIdl) -> AnchorIdlMetadata {
    AnchorIdlMetadata {
        name: classic_idl.name.to_string(),
        version: classic_idl.version.clone(),
        spec: classic_idl.version.clone(),
        description: None,
        repository: None,
        dependencies: vec![],
        contact: None,
        deployments: None,
    }
}

fn classic_instruction_to_anchor_instruction(
    classic_instruction: &ClassicIdlInstruction,
    classic_types: &Vec<ClassicIdlTypeDefinition>,
    idl_serializer: &AnchorIdlSerialization,
    tuple_idx: &mut usize,
) -> Result<(AnchorIdlInstruction, Vec<AnchorIdlTypeDef>), Diagnostic> {
    let mut discriminator = vec![];

    if let Some(classic_discriminator) = &classic_instruction.discriminant {
        if let Some(bytes) = &classic_discriminator.bytes {
            discriminator = bytes.clone();
        } else {
            discriminator = [classic_discriminator.value].to_vec()
        }
    }

    if discriminator.is_empty() {
        discriminator = compute_discriminator("global", &classic_instruction.name);
    }

    let (anchor_arg_fields, anchor_type_defs) = classic_idl_fields_to_anchor_fields_and_type_defs(
        &classic_instruction.args,
        classic_types,
        &idl_serializer,
        tuple_idx,
    )?;

    Ok((
        AnchorIdlInstruction {
            name: classic_instruction.name.to_string(),
            accounts: classic_instruction
                .accounts
                .iter()
                .map(|a| classic_account_item_to_anchor_instruction_account(a))
                .collect(),
            args: anchor_arg_fields,
            docs: vec![],
            discriminator,
            returns: None,
        },
        anchor_type_defs,
    ))
}

fn classic_account_item_to_anchor_instruction_account(
    classic_account: &ClassicIdlAccountItem,
) -> AnchorIdlInstructionAccountItem {
    match classic_account {
        ClassicIdlAccountItem::IdlAccount(a) => {
            AnchorIdlInstructionAccountItem::Single(AnchorIdlInstructionAccount {
                name: a.name.to_string(),
                docs: a.docs.clone().unwrap_or_default(),
                writable: a.is_mut,
                signer: a.is_signer,
                optional: a.optional,
                address: a.address.clone(),
                pda: None,
                relations: vec![],
            })
        }
        ClassicIdlAccountItem::IdlAccounts(a) => {
            AnchorIdlInstructionAccountItem::Composite(AnchorIdlInstructionAccounts {
                name: a.name.to_string(),
                accounts: a
                    .accounts
                    .iter()
                    .map(|a| classic_account_item_to_anchor_instruction_account(a))
                    .collect(),
            })
        }
    }
}

fn classic_account_to_anchor_account(
    classic_account: &ClassicIdlTypeDefinition,
) -> AnchorIdlAccount {
    AnchorIdlAccount {
        name: classic_account.name.clone(),
        discriminator: compute_discriminator("account", &classic_account.name),
    }
}

fn classic_event_to_anchor_event(classic_event: &ClassicIdlEvent) -> AnchorIdlEvent {
    AnchorIdlEvent {
        name: classic_event.name.clone(),
        discriminator: compute_discriminator("event", &classic_event.name),
    }
}

fn classic_idl_type_def_ty_to_anchor_type_def_ty(
    classic_type_def_ty: &ClassicIdlTypeDefinitionTy,
    classic_types: &Vec<ClassicIdlTypeDefinition>,
    idl_serializer: &AnchorIdlSerialization,
    tuple_idx: &mut usize,
) -> Result<(AnchorIdlTypeDefTy, Vec<AnchorIdlTypeDef>), Diagnostic> {
    match &classic_type_def_ty {
        ClassicIdlTypeDefinitionTy::Struct { fields } => {
            let (anchor_fields, type_defs) = classic_idl_fields_to_anchor_fields_and_type_defs(
                &fields,
                classic_types,
                idl_serializer,
                tuple_idx,
            )?;
            Ok((
                AnchorIdlTypeDefTy::Struct {
                    fields: Some(AnchorIdlDefinedFields::Named(anchor_fields)),
                },
                type_defs,
            ))
        }
        ClassicIdlTypeDefinitionTy::Enum { variants } => {
            let mut anchor_variants = vec![];
            let mut anchor_type_defs = vec![];
            for variant in variants {
                let res = if let Some(classic_fields) = &variant.fields {
                    let res = match classic_fields {
                        ClassicEnumFields::Named(idl_fields) => {
                            let (anchor_fields, type_defs) =
                                classic_idl_fields_to_anchor_fields_and_type_defs(
                                    &idl_fields,
                                    classic_types,
                                    idl_serializer,
                                    tuple_idx,
                                )?;

                            (AnchorIdlDefinedFields::Named(anchor_fields), type_defs)
                        }
                        ClassicEnumFields::Tuple(idl_types) => {
                            let mut anchor_types = vec![];
                            let mut all_type_defs = vec![];
                            for classic_type in idl_types.iter() {
                                let (anchor_type, mut type_def) =
                                    classic_type_to_anchor_type_and_type_def(
                                        classic_type,
                                        classic_types,
                                        idl_serializer,
                                        tuple_idx,
                                    )?;
                                anchor_types.push(anchor_type);
                                all_type_defs.append(&mut type_def);
                            }
                            (AnchorIdlDefinedFields::Tuple(anchor_types), all_type_defs)
                        }
                    };

                    Some(res)
                } else {
                    None
                };

                let anchor_fields = if let Some((variant_fields, type_defs)) = res {
                    anchor_type_defs.extend(type_defs);
                    Some(variant_fields)
                } else {
                    None
                };
                anchor_variants.push(AnchorIdlEnumVariant {
                    name: variant.name.clone(),
                    fields: anchor_fields,
                });
            }
            Ok((AnchorIdlTypeDefTy::Enum { variants: anchor_variants }, anchor_type_defs))
        }
    }
}

fn classic_idl_type_def_to_anchor_type_def(
    classic_type_def: &ClassicIdlTypeDefinition,
    classic_idl_types: &Vec<ClassicIdlTypeDefinition>,
    idl_serializer: &AnchorIdlSerialization,
    tuple_idx: &mut usize,
) -> Result<Vec<AnchorIdlTypeDef>, Diagnostic> {
    let (anchor_type_def_ty, mut type_defs) = classic_idl_type_def_ty_to_anchor_type_def_ty(
        &classic_type_def.ty,
        classic_idl_types,
        idl_serializer,
        tuple_idx,
    )?;
    let type_def = AnchorIdlTypeDef {
        name: classic_type_def.name.clone(),
        docs: vec![],
        serialization: idl_serializer.clone(),
        repr: None,
        generics: vec![], // todo
        ty: anchor_type_def_ty,
    };
    type_defs.push(type_def);
    Ok(type_defs)
}

fn compute_discriminator(prefix: &str, input: &str) -> Vec<u8> {
    let prefixed_input = format!("{}:{}", prefix, input);
    let mut result = [0u8; 8];
    result.copy_from_slice(&solana_program::hash::hash(prefixed_input.as_bytes()).to_bytes()[..8]);
    let result = result.to_vec();
    result
}

fn classic_idl_fields_to_anchor_fields_and_type_defs(
    classic_fields: &Vec<ClassicIdlField>,
    classic_types: &Vec<ClassicIdlTypeDefinition>,
    idl_serializer: &AnchorIdlSerialization,
    tuple_idx: &mut usize,
) -> Result<(Vec<AnchorIdlField>, Vec<AnchorIdlTypeDef>), Diagnostic> {
    let mut fields = vec![];
    let mut all_type_defs = vec![];

    for classic_field in classic_fields.iter() {
        let (ty, mut type_defs) = classic_type_to_anchor_type_and_type_def(
            &classic_field.ty,
            classic_types,
            idl_serializer,
            tuple_idx,
        )?;
        let field = AnchorIdlField {
            name: classic_field.name.clone(),
            ty: ty.clone(),
            docs: classic_field.attrs.clone().unwrap_or_default(),
        };
        fields.push(field);
        all_type_defs.append(&mut type_defs);
    }
    Ok((fields, all_type_defs))
}

fn classic_type_to_anchor_type_and_type_def(
    classic_type: &ClassicIdlType,
    classic_types: &Vec<ClassicIdlTypeDefinition>,
    idl_serializer: &AnchorIdlSerialization,
    tuple_idx: &mut usize,
) -> Result<(AnchorIdlType, Vec<AnchorIdlTypeDef>), Diagnostic> {
    let res = match &classic_type {
        ClassicIdlType::Array(idl_type, len) => {
            let (ty, type_def) = classic_type_to_anchor_type_and_type_def(
                idl_type,
                classic_types,
                idl_serializer,
                tuple_idx,
            )?;
            let anchor_type = AnchorIdlType::Array(Box::new(ty), AnchorIdlArrayLen::Value(*len));
            (anchor_type, type_def)
        }
        ClassicIdlType::Vec(idl_type) => {
            let (ty, type_def) = classic_type_to_anchor_type_and_type_def(
                idl_type,
                classic_types,
                idl_serializer,
                tuple_idx,
            )?;
            let anchor_type = AnchorIdlType::Vec(Box::new(ty));
            (anchor_type, type_def)
        }
        ClassicIdlType::HashMap(_, _) | ClassicIdlType::BTreeMap(_, _) => {
            return Err(diagnosed_error!(
                "Map types are not yet supported when converting from classic to anchor IDL"
            ))
        }
        ClassicIdlType::HashSet(idl_type) | ClassicIdlType::BTreeSet(idl_type) => {
            let (ty, type_def) = classic_type_to_anchor_type_and_type_def(
                idl_type,
                classic_types,
                idl_serializer,
                tuple_idx,
            )?;
            let anchor_type = AnchorIdlType::Vec(Box::new(ty));
            (anchor_type, type_def)
        }
        ClassicIdlType::COption(idl_type) | ClassicIdlType::Option(idl_type) => {
            let (ty, type_def) = classic_type_to_anchor_type_and_type_def(
                idl_type,
                classic_types,
                idl_serializer,
                tuple_idx,
            )?;
            let anchor_type = AnchorIdlType::Option(Box::new(ty));
            (anchor_type, type_def)
        }
        ClassicIdlType::Defined(type_name) => {
            let type_def = classic_types.iter().find(|t| t.name.eq(type_name)).ok_or(
                diagnosed_error!("unable to find type definition for {} in idl", type_name),
            )?;

            let anchor_type =
                AnchorIdlType::Defined { name: type_name.to_string(), generics: vec![] };

            let (anchor_type_def_ty, mut type_defs) =
                classic_idl_type_def_ty_to_anchor_type_def_ty(
                    &type_def.ty,
                    classic_types,
                    idl_serializer,
                    tuple_idx,
                )?;

            let anchor_type_def = AnchorIdlTypeDef {
                name: type_name.to_string(),
                docs: vec![],
                serialization: idl_serializer.clone(),
                repr: None,
                generics: vec![],
                ty: anchor_type_def_ty,
            };
            type_defs.push(anchor_type_def);
            (anchor_type, type_defs)
        }
        ClassicIdlType::Tuple(tuple_fields) => {
            let name = format!("tuple_{tuple_idx}");
            *tuple_idx += 1;

            let mut anchor_tuple_types = vec![];
            let mut all_type_defs = vec![];
            for tuple_field in tuple_fields {
                let (anchor_tuple_type, type_defs) = classic_type_to_anchor_type_and_type_def(
                    tuple_field,
                    classic_types,
                    idl_serializer,
                    tuple_idx,
                )?;
                anchor_tuple_types.push(anchor_tuple_type);
                all_type_defs.extend(type_defs);
            }

            let anchor_type_def_ty = AnchorIdlTypeDefTy::Struct {
                fields: Some(AnchorIdlDefinedFields::Tuple(anchor_tuple_types)),
            };

            let anchor_type_def = AnchorIdlTypeDef {
                name: name.clone(),
                docs: vec![],
                serialization: idl_serializer.clone(),
                repr: None,
                generics: vec![],
                ty: anchor_type_def_ty,
            };
            let anchor_type = AnchorIdlType::Defined { name, generics: vec![] };
            all_type_defs.push(anchor_type_def);
            (anchor_type, all_type_defs)
        }
        _ => (primitive_classic_type_to_anchor_type(&classic_type), vec![]),
    };
    Ok(res)
}

fn classic_idl_type_defs_to_anchor_type_defs(
    existing_type_defs: &mut Vec<AnchorIdlTypeDef>,
    classic_types: &Vec<ClassicIdlTypeDefinition>,
    idl_serializer: &AnchorIdlSerialization,
    tuple_idx: &mut usize,
) -> Result<(), Diagnostic> {
    for classic_type in classic_types.iter() {
        // don't evaluate the types we've already checked when converting the instructions
        if !existing_type_defs.iter().any(|t| t.name == classic_type.name) {
            let res = classic_idl_type_def_to_anchor_type_def(
                &classic_type,
                classic_types,
                idl_serializer,
                tuple_idx,
            )?;
            existing_type_defs.extend(res);
        }
    }
    Ok(())
}

fn classic_const_to_anchor_const(
    classic_const: &ClassicIdlConst,
    classic_types: &Vec<ClassicIdlTypeDefinition>,
    idl_serializer: &AnchorIdlSerialization,
    tuple_idx: &mut usize,
) -> Result<(AnchorIdlConst, Vec<AnchorIdlTypeDef>), Diagnostic> {
    let (ty, type_defs) = classic_type_to_anchor_type_and_type_def(
        &classic_const.ty,
        classic_types,
        idl_serializer,
        tuple_idx,
    )?;
    Ok((
        AnchorIdlConst {
            name: classic_const.name.clone(),
            ty,
            value: classic_const.value.clone(),
            docs: vec![],
        },
        type_defs,
    ))
}

fn primitive_classic_type_to_anchor_type(classic_type: &ClassicIdlType) -> AnchorIdlType {
    match classic_type {
        ClassicIdlType::Bool => AnchorIdlType::Bool,
        ClassicIdlType::U8 => AnchorIdlType::U8,
        ClassicIdlType::I8 => AnchorIdlType::I8,
        ClassicIdlType::U16 => AnchorIdlType::U16,
        ClassicIdlType::I16 => AnchorIdlType::I16,
        ClassicIdlType::U32 => AnchorIdlType::U32,
        ClassicIdlType::I32 => AnchorIdlType::I32,
        ClassicIdlType::F32 => AnchorIdlType::F32,
        ClassicIdlType::U64 => AnchorIdlType::U64,
        ClassicIdlType::I64 => AnchorIdlType::I64,
        ClassicIdlType::F64 => AnchorIdlType::F64,
        ClassicIdlType::U128 => AnchorIdlType::U128,
        ClassicIdlType::I128 => AnchorIdlType::I128,
        ClassicIdlType::Bytes => AnchorIdlType::Bytes,
        ClassicIdlType::String => AnchorIdlType::String,
        ClassicIdlType::PublicKey => AnchorIdlType::Pubkey,
        _ => unreachable!(),
    }
}
