use anchor_lang_idl::types::Idl;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use txtx_addon_kit::types::{diagnostics::Diagnostic, types::Value};

use crate::typing::SUBGRAPH_DATA_SOURCE;

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
        field: Vec<SubgraphField>,
        block_height: u64,
    ) -> Result<Self, Diagnostic> {
        let fields = field
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphDataSource {
    pub idl: Idl,
    pub source: SubgraphSourceType,
}

impl SubgraphDataSource {
    pub fn to_value(&self) -> Result<Value, Diagnostic> {
        Ok(Value::addon(
            serde_json::to_vec(self)
                .map_err(|e| diagnosed_error!("could not serialize subgraph data source: {e}"))?,
            SUBGRAPH_DATA_SOURCE,
        ))
    }
    pub fn from_value(value: &Value) -> Result<Self, Diagnostic> {
        let addon_data = value.as_addon_data().ok_or(diagnosed_error!(
            "could not deserialize subgraph data source: expected addon"
        ))?;
        if addon_data.id != SUBGRAPH_DATA_SOURCE {
            return Err(diagnosed_error!(
                "could not deserialize subgraph data source: expected addon type '{}'",
                SUBGRAPH_DATA_SOURCE
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
    pub fn parse_map_value(value: &Value) -> Result<Vec<Self>, Diagnostic> {
        let map = value.as_map().ok_or(diagnosed_error!("subgraph field should be a map"))?;
        if map.len() == 0 {
            return Err(diagnosed_error!("subgraph field should not be empty"));
        }
        let mut fields = Vec::new();
        for entry in map.iter() {
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
