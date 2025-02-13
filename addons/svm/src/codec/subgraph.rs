use anchor_lang_idl::types::Idl;
use serde::{Deserialize, Serialize};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::RpcRequest};
use solana_sdk::pubkey::Pubkey;
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    frontend::{ProgressBarStatus, ProgressBarStatusColor, StatusUpdater},
    types::Value,
};

use crate::typing::{SVM_SUBGRAPH_DATA_SOURCE, SVM_SUBGRAPH_REQUEST};

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
        let params = serde_json::to_value(vec![self.plugin_config.clone()])
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphRequest {
    pub fields: Vec<IndexedSubgraphField>,
    pub program_id: Pubkey,
    pub block_height: u64,
    pub subgraph_name: String,
}

impl SubgraphRequest {
    pub fn new(
        subgraph_name: &str,
        program_id: &Pubkey,
        source: SubgraphDataSource,
        fields: Vec<SubgraphField>,
        block_height: u64,
    ) -> Result<Self, Diagnostic> {
        let fields = fields
            .iter()
            .map(|f| IndexedSubgraphField::new(source.clone(), f.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            fields,
            program_id: *program_id,
            block_height,
            subgraph_name: subgraph_name.to_string(),
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
pub struct SubgraphDataSource {
    pub idl: Idl,
    pub source: SubgraphSourceType,
}

impl SubgraphDataSource {
    pub fn instruction_source(idl: &str, instruction_name: &str) -> Result<Self, Diagnostic> {
        let idl = serde_json::from_str(idl)
            .map_err(|e| diagnosed_error!("could not deserialize IDL: {e}"))?;
        Ok(Self { idl, source: SubgraphSourceType::Instruction(instruction_name.to_string()) })
    }

    pub fn account_source(idl: &str, account_name: &str) -> Result<Self, Diagnostic> {
        let idl = serde_json::from_str(idl)
            .map_err(|e| diagnosed_error!("could not deserialize IDL: {e}"))?;
        Ok(Self { idl, source: SubgraphSourceType::Account(account_name.to_string()) })
    }

    pub fn event_source(idl: &str, event_name: &str) -> Result<Self, Diagnostic> {
        let idl = serde_json::from_str(idl)
            .map_err(|e| diagnosed_error!("could not deserialize IDL: {e}"))?;
        Ok(Self { idl, source: SubgraphSourceType::Event(event_name.to_string()) })
    }

    pub fn to_value(&self) -> Result<Value, Diagnostic> {
        Ok(Value::addon(
            serde_json::to_vec(self)
                .map_err(|e| diagnosed_error!("could not serialize subgraph data source: {e}"))?,
            SVM_SUBGRAPH_DATA_SOURCE,
        ))
    }
    pub fn from_value(value: &Value) -> Result<Self, Diagnostic> {
        let addon_data = value.as_addon_data().ok_or(diagnosed_error!(
            "could not deserialize subgraph data source: expected addon"
        ))?;
        if addon_data.id != SVM_SUBGRAPH_DATA_SOURCE {
            return Err(diagnosed_error!(
                "could not deserialize subgraph data source: expected addon type '{}'",
                SVM_SUBGRAPH_DATA_SOURCE
            ));
        }
        let bytes = addon_data.bytes.clone();

        serde_json::from_slice(&bytes)
            .map_err(|e| diagnosed_error!("could not deserialize subgraph data source: {e}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubgraphSourceType {
    Instruction(String),
    Account(String),
    Event(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphField {
    name: String,
    source: Option<String>,
}

impl SubgraphField {
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
            let source = entry.get("source").and_then(|v| v.as_string().map(|s| s.to_string()));
            fields.push(Self { name: name.to_string(), source });
        }
        Ok(fields)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedSubgraphField {
    pub display_name: String,
    pub source_key: String,
    pub data_source: IndexedSubgraphSourceType,
}

impl IndexedSubgraphField {
    pub fn new(source: SubgraphDataSource, field: SubgraphField) -> Result<Self, Diagnostic> {
        let source_key = field.source.unwrap_or(field.name.clone());
        let display_name = field.name;
        let data_source = match source.source {
            SubgraphSourceType::Instruction(instruction_name) => {
                let instruction = source
                    .idl
                    .instructions
                    .iter()
                    .find(|i| i.name == instruction_name)
                    .ok_or(diagnosed_error!(
                        "could not find instruction '{}' in IDL",
                        instruction_name
                    ))?
                    .clone();
                IndexedSubgraphSourceType::Instruction(InstructionSubgraphSource { instruction })
            }
            SubgraphSourceType::Event(event_name) => {
                let event =
                    source.idl.events.iter().find(|e| e.name == event_name).unwrap().clone();
                let ty = source.idl.types.iter().find(|t| t.name == event_name).unwrap().clone();
                IndexedSubgraphSourceType::Event(EventSubgraphSource { event, ty })
            }
            SubgraphSourceType::Account(_) => {
                todo!()
            }
        };
        Ok(Self { display_name, source_key, data_source })
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
