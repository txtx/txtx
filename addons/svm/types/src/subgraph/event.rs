use std::collections::HashMap;

use anchor_lang_idl::types::Idl;
use serde::{Deserialize, Serialize};
use solana_clock::Slot;
use solana_signature::Signature;
use solana_transaction_status_client_types::InnerInstructions;
use txtx_addon_kit::{
    diagnosed_error,
    types::{diagnostics::Diagnostic, types::Value},
};

use crate::subgraph::{
    idl::parse_bytes_to_value_with_expected_idl_type_def_ty, IntrinsicField, SubgraphRequest,
    SubgraphSourceType, SLOT_INTRINSIC_FIELD, TRANSACTION_SIGNATURE_INTRINSIC_FIELD,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubgraphSource {
    // The event being indexed
    pub event: anchor_lang_idl::types::IdlEvent,
    // The type of the event, found from the IDL
    pub ty: anchor_lang_idl::types::IdlTypeDef,
}

impl SubgraphSourceType for EventSubgraphSource {
    fn intrinsic_fields() -> Vec<IntrinsicField> {
        vec![SLOT_INTRINSIC_FIELD.clone(), TRANSACTION_SIGNATURE_INTRINSIC_FIELD.clone()]
    }
}

impl EventSubgraphSource {
    pub fn from_value(
        value: &Value,
        idl: &Idl,
    ) -> Result<(Self, Option<Vec<Value>>, Option<Vec<Value>>), Diagnostic> {
        let event_map = value.as_map().ok_or(diagnosed_error!("subgraph event must be a map"))?;

        if event_map.len() != 1 {
            return Err(diagnosed_error!("exactly one 'event' should be defined"));
        }
        let entry = event_map.get(0).unwrap();

        let entry = entry
            .as_object()
            .ok_or(diagnosed_error!("each entry of a subgraph event should contain an object"))?;
        let name = entry
            .get("name")
            .ok_or(diagnosed_error!("could not deserialize subgraph event: expected 'name' key"))?;
        let name = name.as_string().ok_or(diagnosed_error!(
            "could not deserialize subgraph event: expected 'name' to be a string"
        ))?;

        let fields = entry.get("field").and_then(|v| v.as_map().map(|s| s.to_vec()));
        let intrinsic_fields =
            entry.get("intrinsic_field").and_then(|v| v.as_map().map(|s| s.to_vec()));
        let event = Self::new(name, idl)?;
        return Ok((event, fields, intrinsic_fields));
    }
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

    pub fn evaluate_inner_instructions(
        &self,
        inner_instructions: &Vec<InnerInstructions>,
        subgraph_request: &SubgraphRequest,
        slot: Slot,
        transaction_signature: Signature,
        entries: &mut Vec<HashMap<String, Value>>,
    ) -> Result<(), String> {
        let SubgraphRequest::V0(subgraph_request) = subgraph_request;
        let empty_vec = vec![];
        let idl_type_def_generics = subgraph_request
            .idl_types
            .iter()
            .find(|t| t.name == self.ty.name)
            .map(|t| &t.generics)
            .unwrap_or(&empty_vec);
        for inner_instructions in inner_instructions.iter() {
            for instruction in inner_instructions.instructions.iter() {
                let instruction = &instruction.instruction;
                // it's not valid cpi event data if there isn't an 8-byte signature
                // well, that ^ is what I thought, but it looks like the _second_ 8 bytes
                // are matching the discriminator
                if instruction.data.len() < 16 {
                    continue;
                }

                let eight_bytes = instruction.data[8..16].to_vec();
                let rest = instruction.data[16..].to_vec();

                if self.event.discriminator.eq(eight_bytes.as_slice()) {
                    let parsed_value =
                        parse_bytes_to_value_with_expected_idl_type_def_ty(&rest, &self.ty.ty, &subgraph_request.idl_types, &vec![], idl_type_def_generics).map_err(
                            |e| format!("event '{}' was emitted in a transaction, but the data could not be parsed as the expected idl type: {e}", self.event.name)
                        )?;

                    let obj = parsed_value.as_object().unwrap().clone();
                    let mut entry = HashMap::new();
                    for field in subgraph_request.defined_fields.iter() {
                        if let Some(v) = obj.get(&field.source_key) {
                            entry.insert(field.display_name.clone(), v.clone());
                        }
                    }

                    subgraph_request.intrinsic_fields.iter().for_each(|field| {
                        if let Some((entry_key, entry_value)) = field.extract_intrinsic(
                            Some(slot),
                            Some(transaction_signature),
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                        ) {
                            entry.insert(entry_key, entry_value);
                        }
                    });

                    entries.push(entry);
                }
            }
        }
        Ok(())
    }
}
