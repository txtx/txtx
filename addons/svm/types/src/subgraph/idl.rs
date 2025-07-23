use anchor_lang_idl::types::{
    IdlConst, IdlDefinedFields, IdlGenericArg, IdlType, IdlTypeDef, IdlTypeDefGeneric, IdlTypeDefTy,
};
use txtx_addon_kit::types::types::{ObjectDefinition, ObjectProperty, Type};

use crate::SVM_PUBKEY;

pub fn get_expected_type_from_idl_defined_fields(
    fields: &IdlDefinedFields,
    idl_types: &Vec<IdlTypeDef>,
    idl_constants: &Vec<IdlConst>,
    generic_args: &Vec<IdlGenericArg>,
    idl_type_def_generics: &Vec<IdlTypeDefGeneric>,
) -> Result<Type, String> {
    match fields {
        IdlDefinedFields::Named(idl_fields) => {
            let mut props = vec![];
            for field in idl_fields {
                let ty = idl_type_to_txtx_type(
                    field.ty.clone(),
                    idl_types,
                    idl_constants,
                    generic_args,
                    idl_type_def_generics,
                )
                .map_err(|e| {
                    format!("could not determine expected type for field '{}': {e}", field.name)
                })?;
                props.push(ObjectProperty {
                    documentation: field.docs.join(" "),
                    typing: ty,
                    optional: false,
                    tainting: false,
                    name: field.name.clone(),
                    internal: false,
                });
            }
            Ok(Type::object(ObjectDefinition::strict(props)))
        }
        IdlDefinedFields::Tuple(tuple_idl_types) => {
            let mut tuple_props = vec![];
            for idl_type in tuple_idl_types {
                let expected_type = idl_type_to_txtx_type(
                    idl_type.clone(),
                    idl_types,
                    idl_constants,
                    generic_args,
                    idl_type_def_generics,
                )?;
                tuple_props.push(ObjectProperty {
                    documentation: "".into(), // Tuples do not have documentation for fields
                    typing: expected_type,
                    optional: false,
                    tainting: false,
                    name: format!("field_{}", tuple_props.len()),
                    internal: false,
                });
            }
            Ok(Type::object(ObjectDefinition::tuple(tuple_props)))
        }
    }
}

pub fn get_expected_type_from_idl_type_def_ty(
    idl_type_def_ty: &IdlTypeDefTy,
    idl_types: &Vec<IdlTypeDef>,
    idl_constants: &Vec<IdlConst>,
    generic_args: &Vec<IdlGenericArg>,
    idl_type_def_generics: &Vec<IdlTypeDefGeneric>,
) -> Result<Type, String> {
    let ty = match idl_type_def_ty {
        IdlTypeDefTy::Struct { fields } => {
            if let Some(fields) = fields {
                get_expected_type_from_idl_defined_fields(
                    fields,
                    idl_types,
                    idl_constants,
                    generic_args,
                    idl_type_def_generics,
                )?
            } else {
                Type::object(ObjectDefinition::strict(vec![])) // Empty struct
            }
        }
        IdlTypeDefTy::Enum { variants } => {
            let mut props = vec![];
            for variant in variants {
                if let Some(ref fields) = variant.fields {
                    let expected_type = get_expected_type_from_idl_defined_fields(
                        fields,
                        idl_types,
                        idl_constants,
                        generic_args,
                        idl_type_def_generics,
                    )?;
                    props.push(ObjectProperty {
                        documentation: "".into(), // Enums do not have documentation for variants
                        typing: expected_type,
                        optional: false,
                        tainting: false,
                        name: variant.name.clone(),
                        internal: false,
                    });
                } else {
                    props.push(ObjectProperty {
                        documentation: "".into(),
                        typing: Type::null(), // No fields means unit type, which we represent as null
                        optional: false,
                        tainting: false,
                        name: variant.name.clone(),
                        internal: false,
                    });
                }
            }
            Type::object(ObjectDefinition::enum_type(props))
        }
        IdlTypeDefTy::Type { alias } => todo!(),
    };
    Ok(ty)
}

pub fn idl_type_to_txtx_type(
    idl_type: IdlType,
    idl_types: &Vec<IdlTypeDef>,
    idl_constants: &Vec<IdlConst>,
    generic_args: &Vec<IdlGenericArg>,
    idl_type_def_generics: &Vec<IdlTypeDefGeneric>,
) -> Result<Type, String> {
    let res = match idl_type {
        IdlType::Bool => Type::bool(),
        IdlType::U8 => Type::addon(crate::SVM_U8),
        IdlType::U16 => Type::addon(crate::SVM_U16),
        IdlType::U32 => Type::addon(crate::SVM_U32),
        IdlType::U64 => Type::addon(crate::SVM_U64),
        IdlType::U128 => Type::addon(crate::SVM_U128),
        IdlType::U256 => Type::addon(crate::SVM_U256),
        IdlType::I8 => Type::addon(crate::SVM_I8),
        IdlType::I16 => Type::addon(crate::SVM_I16),
        IdlType::I32 => Type::addon(crate::SVM_I32),
        IdlType::I64 => Type::addon(crate::SVM_I64),
        IdlType::I128 => Type::addon(crate::SVM_I128),
        IdlType::I256 => Type::addon(crate::SVM_I256),
        IdlType::F32 => Type::addon(crate::SVM_F32),
        IdlType::F64 => Type::addon(crate::SVM_F64),
        IdlType::Bytes => Type::buffer(),
        IdlType::String => Type::string(),
        IdlType::Pubkey => Type::addon(SVM_PUBKEY),
        IdlType::Option(idl_type) => idl_type_to_txtx_type(
            *idl_type,
            idl_types,
            idl_constants,
            generic_args,
            idl_type_def_generics,
        )?,
        IdlType::Vec(idl_type) => Type::array(idl_type_to_txtx_type(
            *idl_type,
            idl_types,
            idl_constants,
            generic_args,
            idl_type_def_generics,
        )?),
        IdlType::Array(idl_type, ..) => Type::array(idl_type_to_txtx_type(
            *idl_type,
            idl_types,
            idl_constants,
            generic_args,
            idl_type_def_generics,
        )?),
        IdlType::Defined { name, generics } => {
            let Some(matching_idl_type) = idl_types.iter().find(|t| t.name == name) else {
                return Err(format!("unable to find defined type '{}'", name));
            };
            let expected_type = get_expected_type_from_idl_type_def_ty(
                &matching_idl_type.ty,
                idl_types,
                idl_constants,
                &generics,
                &matching_idl_type.generics,
            )?;
            expected_type
        }
        IdlType::Generic(generic_name) => {
            let index_of_matching_generic = idl_type_def_generics
                .iter()
                .position(|g| match g {
                    IdlTypeDefGeneric::Type { name } => name.eq(&generic_name),
                    IdlTypeDefGeneric::Const { name, .. } => name.eq(&generic_name),
                })
                .ok_or(format!("unable to find generic type '{}'", generic_name))?;

            let generic_arg = generic_args
                .get(index_of_matching_generic)
                .ok_or(format!("unable to find generic argument for '{}'", generic_name))?;

            match generic_arg {
                IdlGenericArg::Type { ty } => idl_type_to_txtx_type(
                    ty.clone(),
                    idl_types,
                    idl_constants,
                    generic_args,
                    idl_type_def_generics,
                )
                .map_err(|e| format!("unable to resolve generic type '{}': {}", generic_name, e))?,
                IdlGenericArg::Const { .. } => todo!(),
            }
        }
        _ => todo!(),
    };
    Ok(res)
}
