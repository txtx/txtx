use std::str::FromStr;

use anchor_lang_idl::types::{Idl, IdlDefinedFields, IdlType, IdlTypeDefTy};
use serde::{Deserialize, Serialize};
use solana_pubkey::Pubkey;
use txtx_addon_kit::{
    diagnosed_error,
    types::{
        diagnostics::Diagnostic,
        stores::ValueStore,
        types::{Type, Value},
        ConstructDid,
    },
};

use crate::{SVM_I256, SVM_PUBKEY, SVM_U128, SVM_U256};

// Subgraph keys
pub const SVM_SUBGRAPH_REQUEST: &str = "svm::subgraph_request";
pub const FIELD: &str = "field";

pub fn get_expected_field_type_from_idl_type_def_ty(
    field_name: &str,
    idl_type_def_ty: &IdlTypeDefTy,
) -> Result<Type, String> {
    let ty = match idl_type_def_ty {
        IdlTypeDefTy::Struct { fields } => {
            let ty = if let Some(fields) = fields {
                let field = match fields {
                    IdlDefinedFields::Named(idl_fields) => idl_fields
                        .iter()
                        .find(|f| f.name == field_name)
                        .ok_or(format!("unable to find field '{}' in struct", field_name))?,
                    IdlDefinedFields::Tuple(_) => {
                        return Err("cannot find field by name for tuple type".to_string())
                    }
                };
                field.ty.clone()
            } else {
                return Err(format!("unable to find field '{}' in struct", field_name));
            };
            ty
        }
        IdlTypeDefTy::Enum { variants } => {
            return Err(format!("unsupported enum type: {:?}", variants)); // todo
        }
        IdlTypeDefTy::Type { alias } => {
            return Err(format!("unsupported type alias: {:?}", alias)); // todo
        }
    };

    Ok(idl_type_to_txtx_type(ty))
}

pub fn idl_type_to_txtx_type(idl_type: IdlType) -> Type {
    match idl_type {
        IdlType::Bool => Type::bool(),
        IdlType::U8 => Type::integer(),
        IdlType::I8 => Type::integer(),
        IdlType::U16 => Type::integer(),
        IdlType::I16 => Type::integer(),
        IdlType::U32 => Type::integer(),
        IdlType::I32 => Type::integer(),
        IdlType::U64 => Type::integer(),
        IdlType::I64 => Type::integer(),
        IdlType::I128 => Type::integer(),
        IdlType::F32 => Type::float(),
        IdlType::F64 => Type::float(),
        IdlType::U128 => Type::addon(SVM_U128),
        IdlType::U256 => Type::addon(SVM_U256),
        IdlType::I256 => Type::addon(SVM_I256),
        IdlType::Bytes => Type::buffer(),
        IdlType::String => Type::string(),
        IdlType::Pubkey => Type::addon(SVM_PUBKEY),
        IdlType::Option(idl_type) => idl_type_to_txtx_type(*idl_type),
        IdlType::Vec(idl_type) => Type::array(idl_type_to_txtx_type(*idl_type)),
        IdlType::Array(idl_type, ..) => Type::array(idl_type_to_txtx_type(*idl_type)),
        IdlType::Defined { .. } => todo!(),
        IdlType::Generic(_) => todo!(),
        _ => todo!(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub plugin_name: SubgraphPluginType,
    pub data: SubgraphRequest,
    pub workspace: String,
}

impl PluginConfig {
    pub fn new(plugin_name: SubgraphPluginType, data: SubgraphRequest) -> Self {
        Self { plugin_name, data, workspace: "".into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SubgraphPluginType {
    SurfpoolSubgraph,
}

impl std::fmt::Display for SubgraphPluginType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            SubgraphPluginType::SurfpoolSubgraph => "surfpool-subgraph".to_string(),
        };
        write!(f, "{}", val)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphRequest {
    /// The program id of the program to index.
    #[serde(serialize_with = "pubkey_serialize", deserialize_with = "pubkey_deserialize")]
    pub program_id: Pubkey,
    /// The block height at which the subgraph begins indexing.
    pub block_height: u64,
    /// The name of the subgraph. Either provided in the `deploy_subgraph` action, or the name of the data source.
    pub subgraph_name: String,
    /// The description of the subgraph. Either provided in the `deploy_subgraph` action, or the docs from the IDL for the associated data source.
    pub subgraph_description: Option<String>,
    /// The data source to index, with the IDL context needed for the data source type.
    pub data_source: IndexedSubgraphSourceType,
    /// The metadata of the fields to index.
    pub fields: Vec<IndexedSubgraphField>,
    /// The Construct Did of the subgraph request action.
    pub construct_did: ConstructDid,
}

fn pubkey_serialize<S>(value: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn pubkey_deserialize<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Pubkey::from_str(&s).map_err(serde::de::Error::custom)
}

impl SubgraphRequest {
    pub fn parse_value_store(
        subgraph_name: Option<String>,
        subgraph_description: Option<String>,
        program_id: &Pubkey,
        idl_str: &str,
        block_height: u64,
        construct_did: &ConstructDid,
        values: &ValueStore,
    ) -> Result<Self, Diagnostic> {
        let idl = serde_json::from_str(idl_str)
            .map_err(|e| diagnosed_error!("could not deserialize IDL: {e}"))?;

        let (data_source, field_values) = IndexedSubgraphSourceType::parse_values(values, &idl)?;

        let fields = IndexedSubgraphField::new(data_source.clone(), &field_values)?;

        Ok(Self {
            program_id: *program_id,
            block_height,
            subgraph_name: subgraph_name.unwrap_or(data_source.name()),
            subgraph_description: subgraph_description.or(data_source.description()),
            data_source,
            construct_did: construct_did.clone(),
            fields,
        })
    }

    pub fn from_value(value: &Value) -> Result<Self, Diagnostic> {
        let addon_data = value
            .as_addon_data()
            .ok_or(diagnosed_error!("could not deserialize subgraph request: expected addon"))?;
        if addon_data.id != SVM_SUBGRAPH_REQUEST {
            return Err(diagnosed_error!(
                "could not deserialize subgraph request: expected addon type '{}'",
                SVM_SUBGRAPH_REQUEST
            ));
        }
        let bytes = addon_data.bytes.clone();

        serde_json::from_slice(&bytes)
            .map_err(|e| diagnosed_error!("could not deserialize subgraph request: {e}"))
    }

    pub fn to_value(&self) -> Result<Value, Diagnostic> {
        Ok(Value::addon(
            serde_json::to_vec(self)
                .map_err(|e| diagnosed_error!("could not serialize subgraph request: {e}"))?,
            SVM_SUBGRAPH_REQUEST,
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedSubgraphField {
    /// The name of the field, as it will be indexed in the graphql database.
    pub display_name: String,
    /// The name of the field, as it is defined in the IDL. By default, this is the same as the display name.
    pub source_key: String,
    /// The expected type of the field as it will appear in the graphql database. This is parsed from the associated source key in the IDL.
    pub expected_type: Type,
    /// A description of the field. If not provided, the docs in the IDL Event's field will be used, if available.
    pub description: Option<String>,
}

impl IndexedSubgraphField {
    pub fn new(
        data_source: IndexedSubgraphSourceType,
        field_values: &Option<Vec<Value>>,
    ) -> Result<Vec<Self>, Diagnostic> {
        let mut fields = vec![];
        match data_source {
            IndexedSubgraphSourceType::Instruction(_) => {
                return Err(diagnosed_error!("instruction subgraph not supported yet"))
            }
            IndexedSubgraphSourceType::Event(event_subgraph_source) => {
                if let Some(field_values) = field_values {
                    for field_value in field_values.iter() {
                        let field_value = field_value.as_object().ok_or(diagnosed_error!(
                            "each entry of a subgraph field should contain an object"
                        ))?;
                        let name = field_value.get("name").ok_or(diagnosed_error!(
                            "could not deserialize subgraph field: expected 'name' key"
                        ))?;
                        let name = name.as_string().ok_or(diagnosed_error!(
                            "could not deserialize subgraph field: expected 'name' to be a string"
                        ))?;
                        let idl_key = field_value
                            .get("idl_key")
                            .and_then(|v| v.as_string().map(|s| s.to_string()))
                            .unwrap_or(name.to_string());

                        let description = field_value
                            .get("description")
                            .and_then(|v| v.as_string().map(|s| s.to_string()));

                        let expected_type = get_expected_field_type_from_idl_type_def_ty(
                            &idl_key,
                            &event_subgraph_source.ty.ty,
                        )
                        .map_err(|e| {
                            diagnosed_error!(
                                "could not determine expected type for subgraph field '{}': {e}",
                                idl_key
                            )
                        })?;

                        fields.push(Self {
                            display_name: name.to_string(),
                            source_key: idl_key,
                            expected_type,
                            description,
                        });
                    }
                } else {
                    match event_subgraph_source.ty.ty {
                        IdlTypeDefTy::Struct { fields: idl_fields } => {
                            if let Some(idl_fields) = idl_fields {
                                match idl_fields {
                                    IdlDefinedFields::Named(idl_fields) => {
                                        fields.append(
                                            &mut idl_fields
                                                .iter()
                                                .map(|f| Self {
                                                    display_name: f.name.clone(),
                                                    source_key: f.name.clone(),
                                                    expected_type: idl_type_to_txtx_type(
                                                        f.ty.clone(),
                                                    ),
                                                    description: if f.docs.is_empty() {
                                                        None
                                                    } else {
                                                        Some(f.docs.join(" "))
                                                    },
                                                })
                                                .collect(),
                                        );
                                    }

                                    IdlDefinedFields::Tuple(_) => todo!(),
                                }
                            } else {
                                todo!()
                            }
                        }
                        IdlTypeDefTy::Enum { .. } => todo!(),
                        IdlTypeDefTy::Type { .. } => todo!(),
                    }
                }
            }
        }
        Ok(fields)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexedSubgraphSourceType {
    /// Index a program instruction
    Instruction(InstructionSubgraphSource),
    /// Index a program event
    Event(EventSubgraphSource),
    // Account(AccountSubgraphSource),
}

impl IndexedSubgraphSourceType {
    pub fn parse_values(
        values: &ValueStore,
        idl: &Idl,
    ) -> Result<(Self, Option<Vec<Value>>), Diagnostic> {
        if let Some(event) = values.get_value("event") {
            let event_map =
                event.as_map().ok_or(diagnosed_error!("subgraph event must be a map"))?;

            if event_map.len() != 1 {
                return Err(diagnosed_error!("exactly one 'event' should be defined"));
            }
            let entry = event_map.get(0).unwrap();

            let entry = entry.as_object().ok_or(diagnosed_error!(
                "each entry of a subgraph event should contain an object"
            ))?;
            let name = entry.get("name").ok_or(diagnosed_error!(
                "could not deserialize subgraph event: expected 'name' key"
            ))?;
            let name = name.as_string().ok_or(diagnosed_error!(
                "could not deserialize subgraph event: expected 'name' to be a string"
            ))?;
            let fields = entry.get("field").and_then(|v| v.as_map().map(|s| s.to_vec()));
            let event = EventSubgraphSource::new(name, idl)?;
            return Ok((Self::Event(event), fields));
        } else if let Some(_) = values.get_value("instruction") {
            return Err(diagnosed_error!("subgraph instruction not supported yet"));
        } else if let Some(_) = values.get_value("account") {
            return Err(diagnosed_error!("subgraph account not supported yet"));
        }

        Err(diagnosed_error!("no event, instruction, or account map provided"))
    }

    pub fn description(&self) -> Option<String> {
        match self {
            IndexedSubgraphSourceType::Instruction(instruction_subgraph_source) => {
                if instruction_subgraph_source.instruction.docs.is_empty() {
                    None
                } else {
                    Some(instruction_subgraph_source.instruction.docs.join(" "))
                }
            }
            IndexedSubgraphSourceType::Event(event_subgraph_source) => {
                if event_subgraph_source.ty.docs.is_empty() {
                    None
                } else {
                    Some(event_subgraph_source.ty.docs.join(" "))
                }
            } // IndexedSubgraphSourceType::Account(_) => None,
        }
    }

    pub fn name(&self) -> String {
        match self {
            IndexedSubgraphSourceType::Instruction(instruction_subgraph_source) => {
                instruction_subgraph_source.instruction.name.clone()
            }
            IndexedSubgraphSourceType::Event(event_subgraph_source) => {
                event_subgraph_source.event.name.clone()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionSubgraphSource {
    // The instruction being indexed
    pub instruction: anchor_lang_idl::types::IdlInstruction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubgraphSource {
    // The event being indexed
    pub event: anchor_lang_idl::types::IdlEvent,
    // The type of the event, found from the IDL
    pub ty: anchor_lang_idl::types::IdlTypeDef,
}

impl EventSubgraphSource {
    pub fn new(event_name: &str, idl: &Idl) -> Result<Self, Diagnostic> {
        let event = idl
            .events
            .iter()
            .find(|e| e.name == event_name)
            .ok_or(diagnosed_error!("could not find event '{}' in IDL", event_name))?;
        let ty = idl
            .types
            .iter()
            .find(|t| t.name == event_name)
            .ok_or(diagnosed_error!("could not find type '{}' in IDL", event_name))?;
        Ok(Self { event: event.clone(), ty: ty.clone() })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSubgraphSource {}
