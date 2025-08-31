//! Event extraction and parsing utilities for EVM transaction logs
//! 
//! This module provides simplified event extraction from transaction receipts
//! stored in test outputs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use txtx_addon_kit::types::types::Value;
use crate::tests::test_harness::ValueComparison;

/// Simplified parsed event from a transaction log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedEvent {
    pub name: String,
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
    pub decoded_args: Option<HashMap<String, Value>>,
    pub block_number: Option<u64>,
    pub transaction_hash: Option<String>,
    pub log_index: Option<u64>,
}

/// Extract events from a transaction receipt stored in the test output
pub fn extract_events_from_receipt(receipt_value: &Value) -> Vec<ParsedEvent> {
    let mut events = Vec::new();
    
    // Try to find logs in the receipt structure
    if let Some(logs) = receipt_value.get_path("logs") {
        if let Value::Array(logs_array) = logs {
            for log_value in logs_array.iter() {
                if let Some(parsed) = parse_log_from_value(log_value) {
                    events.push(parsed);
                }
            }
        }
    }
    
    events
}

fn parse_log_from_value(log_value: &Value) -> Option<ParsedEvent> {
    let address = log_value.get_path("address")
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })?;
    
    let mut topics = Vec::new();
    if let Some(Value::Array(topics_array)) = log_value.get_path("topics") {
        for topic_value in topics_array.iter() {
            if let Value::String(s) = topic_value {
                topics.push(s.clone());
            }
        }
    }
    
    let data = log_value.get_path("data")
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();
    
    Some(ParsedEvent {
        name: identify_event_name(&topics),
        address,
        topics,
        data,
        decoded_args: None,
        block_number: log_value.get_path("blockNumber")
            .and_then(|v| match v {
                Value::Integer(i) => Some(*i as u64),
                _ => None,
            }),
        transaction_hash: log_value.get_path("transactionHash")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            }),
        log_index: log_value.get_path("logIndex")
            .and_then(|v| match v {
                Value::Integer(i) => Some(*i as u64),
                _ => None,
            }),
    })
}

/// Identify common event names based on topic0 (event signature hash)
fn identify_event_name(topics: &[String]) -> String {
    if let Some(topic0) = topics.first() {
        // Common ERC20/ERC721 Transfer event
        if topic0 == "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef" {
            return "Transfer".to_string();
        }
        // Common Approval event
        if topic0 == "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925" {
            return "Approval".to_string();
        }
        // Add more known events as needed
    }
    "Unknown".to_string()
}

/// Helper to filter events by name
pub fn filter_events_by_name<'a>(events: &'a [ParsedEvent], name: &str) -> Vec<&'a ParsedEvent> {
    events.iter()
        .filter(|e| e.name == name)
        .collect()
}

/// Helper to filter events by address
pub fn filter_events_by_address<'a>(events: &'a [ParsedEvent], address: &str) -> Vec<&'a ParsedEvent> {
    events.iter()
        .filter(|e| e.address == address)
        .collect()
}