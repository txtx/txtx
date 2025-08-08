use std::str::FromStr;

use anchor_lang_idl::types::{Idl, IdlConst, IdlDefinedFields, IdlTypeDef, IdlTypeDefTy};
use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};
use solana_clock::Slot;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use txtx_addon_kit::{
    diagnosed_error,
    types::{
        diagnostics::Diagnostic,
        stores::ValueStore,
        types::{ObjectDefinition, Type, Value},
        ConstructDid,
    },
};

mod event;
pub mod idl;
mod pda;

pub use event::EventSubgraphSource;
pub use pda::PdaSubgraphSource;

use crate::{
    subgraph::idl::{get_expected_type_from_idl_type_def_ty, idl_type_to_txtx_type},
    SVM_PUBKEY, SVM_SIGNATURE,
};

// Subgraph keys
pub const SVM_SUBGRAPH_REQUEST: &str = "svm::subgraph_request";
pub const FIELD: &str = "field";

lazy_static! {
    pub static ref SLOT_INTRINSIC_FIELD: IntrinsicField = IntrinsicField {
        name: "slot".into(),
        expected_type: Type::integer(),
        description: "The slot in which the event was emitted.".into(),
        is_indexed: true,
    };
    pub static ref TRANSACTION_SIGNATURE_INTRINSIC_FIELD: IntrinsicField = IntrinsicField {
        name: "transaction_signature".into(),
        expected_type: Type::addon(SVM_SIGNATURE),
        description: "The transaction signature in which the event was emitted.".into(),
        is_indexed: true,
    };
    pub static ref PUBKEY_INTRINSIC_FIELD: IntrinsicField = IntrinsicField {
        name: "pubkey".into(),
        expected_type: Type::addon(SVM_PUBKEY),
        description: "The public key of the account.".into(),
        is_indexed: true,
    };
    pub static ref OWNER_INTRINSIC_FIELD: IntrinsicField = IntrinsicField {
        name: "owner".into(),
        expected_type: Type::addon(SVM_PUBKEY),
        description: "The owner of the account.".into(),
        is_indexed: false,
    };
    pub static ref LAMPORTS_INTRINSIC_FIELD: IntrinsicField = IntrinsicField {
        name: "lamports".into(),
        expected_type: Type::integer(),
        description: "The lamports of the account.".into(),
        is_indexed: false,
    };
    pub static ref WRITE_VERSION_INTRINSIC_FIELD: IntrinsicField = IntrinsicField {
        name: "write_version".into(),
        expected_type: Type::integer(),
        description: "A monotonically increasing index of the account update.".into(),
        is_indexed: true,
    };
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
pub enum SubgraphRequest {
    V0(SubgraphRequestV0),
}

impl SubgraphRequest {
    pub fn parse_value_store_v0(
        subgraph_name: Option<String>,
        subgraph_description: Option<String>,
        program_id: &Pubkey,
        idl_str: &str,
        slot: u64,
        construct_did: &ConstructDid,
        values: &ValueStore,
    ) -> Result<Self, Diagnostic> {
        let request = SubgraphRequestV0::parse_value_store(
            subgraph_name,
            subgraph_description,
            program_id,
            idl_str,
            slot,
            construct_did,
            values,
        )?;
        Ok(SubgraphRequest::V0(request))
    }

    pub fn from_value_v0(value: &Value) -> Result<Self, Diagnostic> {
        Ok(SubgraphRequest::V0(SubgraphRequestV0::from_value(value)?))
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

    pub fn subgraph_name(&self) -> &str {
        match self {
            SubgraphRequest::V0(request) => request.subgraph_name.as_str(),
        }
    }

    pub fn program_id(&self) -> Pubkey {
        match self {
            SubgraphRequest::V0(request) => request.program_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphRequestV0 {
    /// The program id of the program to index.
    #[serde(serialize_with = "pubkey_serialize", deserialize_with = "pubkey_deserialize")]
    pub program_id: Pubkey,
    /// The slot at which the subgraph begins indexing.
    pub slot: u64,
    /// The name of the subgraph. Either provided in the `deploy_subgraph` action, or the name of the data source.
    pub subgraph_name: String,
    /// The description of the subgraph. Either provided in the `deploy_subgraph` action, or the docs from the IDL for the associated data source.
    pub subgraph_description: Option<String>,
    /// The data source to index, with the IDL context needed for the data source type.
    pub data_source: IndexedSubgraphSourceType,
    /// The metadata of the fields to index. These fields are intrinsic to the data source type.
    /// For example, an event subgraph will include `slot`, while a PDA subgraph will include `pubkey`, `lamports`, and `owner`.
    pub intrinsic_fields: Vec<IndexedSubgraphField>,
    /// The metadata of the fields to index. These fields are defined in the IDL.
    pub defined_fields: Vec<IndexedSubgraphField>,
    /// The Construct Did of the subgraph request action.
    pub construct_did: ConstructDid,
    /// The network to index. This is used to determine the network of the subgraph.
    pub network: String,
    /// The IDL types defined in the IDL.
    pub idl_types: Vec<IdlTypeDef>,
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

impl SubgraphRequestV0 {
    pub fn parse_value_store(
        subgraph_name: Option<String>,
        subgraph_description: Option<String>,
        program_id: &Pubkey,
        idl_str: &str,
        slot: u64,
        construct_did: &ConstructDid,
        values: &ValueStore,
    ) -> Result<Self, Diagnostic> {
        let idl = serde_json::from_str(idl_str)
            .map_err(|e| diagnosed_error!("could not deserialize IDL: {e}"))?;

        let (data_source, defined_field_values, intrinsic_field_values) =
            IndexedSubgraphSourceType::parse_values(values, &idl)?;

        let defined_fields = IndexedSubgraphField::parse_defined_field_values(
            data_source.clone(),
            &defined_field_values,
            &idl.types,
            &idl.constants,
        )?;

        let intrinsic_fields = IndexedSubgraphField::parse_intrinsic_field_values(
            data_source.clone(),
            intrinsic_field_values,
        )?;

        Ok(Self {
            program_id: *program_id,
            slot,
            subgraph_name: subgraph_name.unwrap_or(data_source.name()),
            subgraph_description: subgraph_description.or(data_source.description()),
            data_source,
            construct_did: construct_did.clone(),
            defined_fields,
            intrinsic_fields,
            network: "solana-devnet".into(),
            idl_types: idl.types,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrinsicField {
    /// The name of the intrinsic field.
    pub name: String,
    /// The expected type of the intrinsic field.
    pub expected_type: Type,
    /// A description of the intrinsic field.
    pub description: String,
    /// Whether the intrinsic field is indexed in the subgraph.
    pub is_indexed: bool,
}
impl IntrinsicField {
    pub fn to_indexed_field(&self) -> IndexedSubgraphField {
        IndexedSubgraphField {
            display_name: self.name.to_case(Case::Camel),
            source_key: self.name.clone(),
            expected_type: self.expected_type.clone(),
            description: Some(self.description.clone()),
            is_indexed: self.is_indexed,
        }
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
    /// Whether the field is indexed in the subgraph.
    pub is_indexed: bool,
}

impl IndexedSubgraphField {
    pub fn extract_intrinsic(
        &self,
        slot: Option<Slot>,
        transaction_signature: Option<Signature>,
        pubkey: Option<Pubkey>,
        owner: Option<Pubkey>,
        lamports: Option<u64>,
        write_version: Option<u64>,
    ) -> Option<(String, Value)> {
        match self.source_key.as_str() {
            "slot" => slot.map(|s| (self.display_name.clone(), Value::integer(s as i128))),
            "transaction_signature" => transaction_signature.map(|s| {
                (self.display_name.clone(), Value::addon(s.as_ref().to_vec(), SVM_SIGNATURE))
            }),
            "pubkey" => pubkey.map(|p| {
                (self.display_name.clone(), Value::addon(p.to_bytes().to_vec(), SVM_PUBKEY))
            }),
            "owner" => owner.map(|o| {
                (self.display_name.clone(), Value::addon(o.to_bytes().to_vec(), SVM_PUBKEY))
            }),
            "lamports" => lamports.map(|l| (self.display_name.clone(), Value::integer(l as i128))),
            "write_version" => {
                write_version.map(|w| (self.display_name.clone(), Value::integer(w as i128)))
            }
            _ => None,
        }
    }

    pub fn parse_intrinsic_field_values(
        data_source: IndexedSubgraphSourceType,
        intrinsic_field_values: Option<Vec<Value>>,
    ) -> Result<Vec<Self>, Diagnostic> {
        data_source.index_intrinsics(intrinsic_field_values)
    }

    pub fn parse_defined_field_values(
        data_source: IndexedSubgraphSourceType,
        field_values: &Option<Vec<Value>>,
        idl_types: &Vec<IdlTypeDef>,
        idl_constants: &Vec<IdlConst>,
    ) -> Result<Vec<Self>, Diagnostic> {
        match data_source {
            IndexedSubgraphSourceType::Instruction(_) => {
                Err(diagnosed_error!("instruction subgraph not supported yet"))
            }
            IndexedSubgraphSourceType::Event(event_subgraph_source) => {
                IndexedSubgraphField::parse_user_defined_field_values_against_idl(
                    field_values,
                    &event_subgraph_source.ty.ty,
                    idl_types,
                    idl_constants,
                )
            }
            IndexedSubgraphSourceType::Pda(pda_subgraph_source) => {
                IndexedSubgraphField::parse_user_defined_field_values_against_idl(
                    field_values,
                    &pda_subgraph_source.account_type.ty,
                    idl_types,
                    idl_constants,
                )
            }
        }
    }

    fn parse_user_defined_field_values_against_idl(
        field_values: &Option<Vec<Value>>,
        idl_type_def_ty: &IdlTypeDefTy,
        idl_types: &Vec<IdlTypeDef>,
        idl_constants: &Vec<IdlConst>,
    ) -> Result<Vec<Self>, Diagnostic> {
        let mut fields = vec![];

        let expected_type_for_type_def = get_expected_type_from_idl_type_def_ty(
            idl_type_def_ty,
            idl_types,
            idl_constants,
            &vec![],
            &vec![],
        )?;

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

                let idl_key_value = field_value.get("idl_key");
                let idl_key = idl_key_value
                    .and_then(|v| v.as_string().map(|s| s.to_string()))
                    .unwrap_or(name.to_string());

                let display_name = if idl_key_value.is_some() {
                    // key and name were specified, meaning the `name` field is an intentional renam
                    name.to_string()
                } else {
                    name.to_case(Case::Camel)
                };

                let description = field_value
                    .get("description")
                    .and_then(|v| v.as_string().map(|s| s.to_string()));

                let expected_type = expected_type_for_type_def
                    .as_object()
                    .and_then(|obj| match obj {
                        ObjectDefinition::Strict(items) => items
                            .iter()
                            .find(|item| item.name == idl_key)
                            .map(|item| item.typing.clone()),
                        other => unreachable!(
                            "Strict object definition expected for subgraph field, found {:?}",
                            other
                        ),
                    })
                    .ok_or(diagnosed_error!(
                        "could not find field '{}' in expected type for subgraph field",
                        idl_key
                    ))?;

                let is_indexed =
                    field_value.get("is_indexed").and_then(|v| v.as_bool()).unwrap_or(false);

                fields.push(Self {
                    display_name,
                    source_key: idl_key,
                    expected_type,
                    description,
                    is_indexed,
                });
            }
        } else {
            match idl_type_def_ty {
                IdlTypeDefTy::Struct { fields: idl_fields } => {
                    if let Some(idl_fields) = idl_fields {
                        match idl_fields {
                            IdlDefinedFields::Named(idl_fields) => {
                                fields.append(
                                    &mut idl_fields
                                        .iter()
                                        .map(|f|{
                                            idl_type_to_txtx_type(
                                                f.ty.clone(),
                                                idl_types,
                                                idl_constants,
                                                &vec![],
                                                &vec![],
                                            ).map_err(|e| {
                                                diagnosed_error!(
                                                    "could not determine expected type for subgraph field '{}': {e}",
                                                    f.name
                                                )
                                            }).map(|expected_type| Self {
                                            display_name: f.name.to_case(Case::Camel),
                                            source_key: f.name.clone(),
                                            expected_type ,
                                            description: if f.docs.is_empty() {
                                                None
                                            } else {
                                                Some(f.docs.join(" "))
                                            },
                                            is_indexed: false,
                                        })
                                    })
                                    .collect::<Result<Vec<_>, _>>()?,
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
        Ok(fields)
    }
}

trait SubgraphSourceType {
    fn intrinsic_fields() -> Vec<IntrinsicField>;
    fn index_intrinsics(
        &self,
        intrinsic_field_values: Option<Vec<Value>>,
    ) -> Result<Vec<IndexedSubgraphField>, Diagnostic> {
        let available_fields = Self::intrinsic_fields();
        match intrinsic_field_values {
            Some(intrinsic_field_values) => {
                let mut indexed = vec![];
                for field_value in intrinsic_field_values {
                    let field_value = field_value.as_object().ok_or(diagnosed_error!(
                        "each entry of a subgraph intrinsic field should contain an object"
                    ))?;
                    let name = field_value.get("name").ok_or(diagnosed_error!(
                        "could not deserialize subgraph intrinsic field: expected 'name' key"
                    ))?;
                    let name = name.as_string().ok_or(diagnosed_error!(
                        "could not deserialize subgraph intrinsic field: expected 'name' to be a string"
                    ))?;

                    let default_display_name = name.to_case(Case::Camel);

                    let display_name = field_value
                        .get("display_name")
                        .and_then(|v| v.as_string().map(|s| s.to_string()))
                        .unwrap_or(default_display_name);

                    let description = field_value
                        .get("description")
                        .and_then(|v| v.as_string().map(|s| s.to_string()));

                    let is_indexed = field_value.get("is_indexed").and_then(|v| v.as_bool());

                    let matching = available_fields.iter().find(|f| f.name == name).ok_or(
                        diagnosed_error!(
                            "could not find intrinsic field '{}' in subgraph source type",
                            name
                        ),
                    )?;
                    indexed.push(IndexedSubgraphField {
                        display_name,
                        source_key: name.to_string(),
                        expected_type: matching.expected_type.clone(),
                        description: description.or(Some(matching.description.clone())),
                        is_indexed: is_indexed.unwrap_or(matching.is_indexed),
                    })
                }
                Ok(indexed)
            }
            None => Ok(available_fields.into_iter().map(|f| f.to_indexed_field()).collect()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexedSubgraphSourceType {
    /// Index a program instruction
    Instruction(InstructionSubgraphSource),
    /// Index a program event
    Event(EventSubgraphSource),
    // Account(AccountSubgraphSource),
    /// Index a program derived account
    Pda(PdaSubgraphSource),
}

impl IndexedSubgraphSourceType {
    pub fn parse_values(
        values: &ValueStore,
        idl: &Idl,
    ) -> Result<(Self, Option<Vec<Value>>, Option<Vec<Value>>), Diagnostic> {
        if let Some(event) = values.get_value("event") {
            let (event, fields, intrinsic_fields) = EventSubgraphSource::from_value(event, idl)?;
            return Ok((Self::Event(event), fields, intrinsic_fields));
        } else if let Some(_) = values.get_value("instruction") {
            return Err(diagnosed_error!("subgraph instruction not supported yet"));
        } else if let Some(_) = values.get_value("account") {
            return Err(diagnosed_error!("subgraph account not supported yet"));
        } else if let Some(pda) = values.get_value("pda") {
            let (pda_source, fields, intrinsic_fields) = PdaSubgraphSource::from_value(pda, idl)?;
            return Ok((Self::Pda(pda_source), fields, intrinsic_fields));
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
            IndexedSubgraphSourceType::Pda(pda_subgraph_source) => {
                if pda_subgraph_source.account_type.docs.is_empty() {
                    None
                } else {
                    Some(pda_subgraph_source.account_type.docs.join(" "))
                }
            }
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
            IndexedSubgraphSourceType::Pda(pda_subgraph_source) => {
                pda_subgraph_source.account.name.clone()
            }
        }
    }

    pub fn index_intrinsics(
        &self,
        intrinsic_field_values: Option<Vec<Value>>,
    ) -> Result<Vec<IndexedSubgraphField>, Diagnostic> {
        match self {
            IndexedSubgraphSourceType::Instruction(_) => {
                Err(diagnosed_error!("instruction subgraph not supported yet"))
            }
            IndexedSubgraphSourceType::Event(event_subgraph_source) => {
                event_subgraph_source.index_intrinsics(intrinsic_field_values)
            }
            IndexedSubgraphSourceType::Pda(pda_subgraph_source) => {
                pda_subgraph_source.index_intrinsics(intrinsic_field_values)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionSubgraphSource {
    // The instruction being indexed
    pub instruction: anchor_lang_idl::types::IdlInstruction,
}

/// Recursively find an [IdlInstructionAccount] by name in an [IdlInstructionAccountItem].
pub fn find_idl_instruction_account(
    account_item: &IdlInstructionAccountItem,
    name: &str,
) -> Option<IdlInstructionAccount> {
    match account_item {
        IdlInstructionAccountItem::Composite(idl_instruction_accounts) => idl_instruction_accounts
            .accounts
            .iter()
            .find_map(|a| find_idl_instruction_account(a, name)),
        IdlInstructionAccountItem::Single(idl_instruction_account) => {
            if idl_instruction_account.name == name {
                Some(idl_instruction_account.clone())
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests;
