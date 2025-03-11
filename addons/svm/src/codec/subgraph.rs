use anchor_lang_idl::types::{Idl, IdlDefinedFields, IdlTypeDefTy};
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::RpcRequest};
use solana_sdk::pubkey::Pubkey;
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    frontend::{ProgressBarStatus, ProgressBarStatusColor, StatusUpdater},
    types::{Type, Value},
};

use crate::{constants::FIELD, typing::SVM_SUBGRAPH_REQUEST};

use super::idl::get_expected_field_type_from_idl_type_def_ty;

pub struct SubgraphRequestClient {
    rpc_client: RpcClient,
    plugin_config: PluginConfig,
    status_updater: StatusUpdater,
}

impl SubgraphRequestClient {
    pub fn new(
        rpc_api_url: &str,
        request: SubgraphRequest,
        plugin_name: SubgraphPluginType,
        status_updater: StatusUpdater,
    ) -> Self {
        Self {
            rpc_client: RpcClient::new(rpc_api_url.to_string()),
            plugin_config: PluginConfig::new(plugin_name, request),
            status_updater,
        }
    }

    pub async fn deploy_subgraph(&mut self) -> Result<String, Diagnostic> {
        let stringified_config = json![self.plugin_config.clone()];
        let params = serde_json::to_value(vec![stringified_config.to_string()])
            .map_err(|e| diagnosed_error!("could not serialize subgraph request: {e}"))?;
        let res = self
            .rpc_client
            .send::<String>(RpcRequest::Custom { method: "loadPlugin" }, params)
            .await
            .map_err(|e| diagnosed_error!("could not deploy subgraph: {e}"))?;

        self.status_updater.propagate_status(ProgressBarStatus::new_msg(
            ProgressBarStatusColor::Green,
            "Subgraph Deployed",
            &format!(
                "Subgraph {} for program {} has been deployed",
                self.plugin_config.data.subgraph_name, self.plugin_config.data.program_id,
            ),
        ));

        self.status_updater.propagate_info(&format!("Your subgraph can be reached at {}", res));

        Ok(res)
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
impl SubgraphPluginType {
    pub fn to_string(&self) -> String {
        match self {
            SubgraphPluginType::SurfpoolSubgraph => "surfpool-subgraph".to_string(),
        }
    }
}
impl std::fmt::Display for SubgraphPluginType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphRequest {
    pub fields: Vec<IndexedSubgraphField>,
    pub program_id: Pubkey,
    pub block_height: u64,
    pub subgraph_name: String,
    pub subgraph_description: Option<String>,
}

impl SubgraphRequest {
    pub fn new(
        subgraph_name: &str,
        subgraph_description: Option<String>,
        program_id: &Pubkey,
        idl_str: &str,
        events: Vec<SubgraphEventDefinition>,
        block_height: u64,
    ) -> Result<Self, Diagnostic> {
        let idl = serde_json::from_str(idl_str)
            .map_err(|e| diagnosed_error!("could not deserialize IDL: {e}"))?;

        let fields = events
            .iter()
            .map(|f| IndexedSubgraphField::from_event_definition(&idl, f.clone()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(Self {
            fields,
            program_id: *program_id,
            block_height,
            subgraph_name: subgraph_name.to_string(),
            subgraph_description,
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
pub enum SubgraphSourceType {
    Instruction(String),
    Account(String),
    Event(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphEventDefinition {
    pub name: String,
    pub fields: Vec<SubgraphFieldDefinition>,
}

impl SubgraphEventDefinition {
    pub fn parse_map_values(values: &Vec<Value>, idl_str: &str) -> Result<Vec<Self>, Diagnostic> {
        if values.len() == 0 {
            return Err(diagnosed_error!("subgraph event should not be empty"));
        }
        let idl: Idl = serde_json::from_str(idl_str)
            .map_err(|e| diagnosed_error!("could not deserialize IDL: {e}"))?;
        let mut events = Vec::new();
        for entry in values.iter() {
            let entry = entry.as_object().ok_or(diagnosed_error!(
                "each entry of a subgraph event should contain an object"
            ))?;
            let name = entry.get("name").ok_or(diagnosed_error!(
                "could not deserialize subgraph event: expected 'name' key"
            ))?;
            let name = name.as_string().ok_or(diagnosed_error!(
                "could not deserialize subgraph event: expected 'name' to be a string"
            ))?;

            let idl_event = idl
                .events
                .iter()
                .find(|e| e.name == name)
                .ok_or(diagnosed_error!("could not find event '{}' in IDL", name))?;

            let fields = if let Some(fields) = entry.get(FIELD) {
                let fields = fields.as_array().ok_or(diagnosed_error!(
                    "could not deserialize subgraph event: expected 'fields' to be an array"
                ))?;
                SubgraphFieldDefinition::parse_map_values(fields)?
            } else {
                let ty = idl
                    .types
                    .iter()
                    .find(|t| t.name == idl_event.name)
                    .ok_or(diagnosed_error!("could not find type '{}' in IDL", name))?;
                SubgraphFieldDefinition::from_idl_type(&ty.ty)?
            };
            events.push(Self { name: name.to_string(), fields });
        }
        Ok(events)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphFieldDefinition {
    name: String,
    source: Option<String>,
    description: Option<String>,
}

impl SubgraphFieldDefinition {
    pub fn new(name: &str, source: Option<String>, description: Option<String>) -> Self {
        Self { name: name.to_string(), source, description }
    }
    pub fn parse_map_values(values: &Vec<Value>) -> Result<Vec<Self>, Diagnostic> {
        if values.len() == 0 {
            return Err(diagnosed_error!("subgraph field should not be empty"));
        }
        let mut fields = Vec::new();
        for entry in values.iter() {
            let entry = entry.as_object().ok_or(diagnosed_error!(
                "each entry of a subgraph field should contain an object"
            ))?;
            let name = entry.get("name").ok_or(diagnosed_error!(
                "could not deserialize subgraph field: expected 'name' key"
            ))?;
            let name = name.as_string().ok_or(diagnosed_error!(
                "could not deserialize subgraph field: expected 'name' to be a string"
            ))?;
            let source = entry.get("idl_key").and_then(|v| v.as_string().map(|s| s.to_string()));
            let description =
                entry.get("description").and_then(|v| v.as_string().map(|s| s.to_string()));
            fields.push(Self { name: name.to_string(), source, description });
        }
        Ok(fields)
    }

    fn from_idl_type(ty: &IdlTypeDefTy) -> Result<Vec<Self>, Diagnostic> {
        match ty {
            IdlTypeDefTy::Struct { fields } => {
                if let Some(fields) = fields {
                    match fields {
                        IdlDefinedFields::Named(idl_fields) => Ok(idl_fields
                            .iter()
                            .map(|f| {
                                Self::new(
                                    &f.name,
                                    None,
                                    if f.docs.is_empty() { None } else { Some(f.docs.join(" ")) },
                                )
                            })
                            .collect()),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedSubgraphField {
    pub display_name: String,
    pub source_key: String,
    pub expected_type: Type,
    pub description: Option<String>,
    pub data_source: IndexedSubgraphSourceType,
}

impl IndexedSubgraphField {
    pub fn from_event_definition(
        idl: &Idl,
        event_def: SubgraphEventDefinition,
    ) -> Result<Vec<Self>, Diagnostic> {
        let mut fields = Vec::new();
        for field_def in event_def.fields.iter() {
            let source_key = field_def.source.clone().unwrap_or(field_def.name.clone());
            let display_name = field_def.name.clone();
            let idl_event = idl
                .events
                .iter()
                .find(|e| e.name == event_def.name)
                .ok_or(diagnosed_error!("could not find event '{}' in IDL", event_def.name))?;
            let ty = idl
                .types
                .iter()
                .find(|t| t.name == event_def.name)
                .ok_or(diagnosed_error!("could not find type '{}' in IDL", event_def.name))?;

            let expected_type = get_expected_field_type_from_idl_type_def_ty(&source_key, &ty.ty)
                .map_err(|e| {
                diagnosed_error!(
                    "could not determine expected type for subgraph field '{}': {e}",
                    source_key
                )
            })?;

            fields.push(Self {
                display_name,
                source_key,
                expected_type,
                description: field_def.description.clone(),
                data_source: IndexedSubgraphSourceType::Event(EventSubgraphSource {
                    event: idl_event.clone(),
                    ty: ty.clone(),
                }),
            })
        }
        Ok(fields)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexedSubgraphSourceType {
    Instruction(InstructionSubgraphSource),
    Event(EventSubgraphSource),
    // Account(AccountSubgraphSource),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSubgraphSource {}
