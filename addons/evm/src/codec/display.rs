// Display formatting functions for transactions and other EVM types

use alloy::consensus::{Transaction, TypedTransaction};
use alloy::hex;
use alloy::primitives::utils::format_units;
use alloy_rpc_types::AccessList;
use txtx_addon_kit::types::types::{ObjectType, Value};

/// Format a transaction for display to the user
pub fn format_transaction_for_display(typed_transaction: &TypedTransaction) -> Value {
    let mut res = ObjectType::from(vec![
        (
            "kind",
            match typed_transaction.to() {
                None => Value::string("create".to_string()),
                Some(address) => Value::string(format!("to:{}", address.to_string())),
            },
        ),
        ("nonce", Value::integer(typed_transaction.nonce() as i128)),
        ("gas_limit", Value::integer(typed_transaction.gas_limit() as i128)),
        ("input", Value::string(hex::encode(&typed_transaction.input()))),
        ("value", Value::string(format_units(typed_transaction.value(), "ether").unwrap())),
        ("type", Value::string(typed_transaction.tx_type().to_string())),
    ]);
    if let Some(chain_id) = typed_transaction.chain_id() {
        res.insert("chain_id", Value::integer(chain_id as i128));
    }
    match typed_transaction {
        TypedTransaction::Legacy(tx) => {
            if let Some(gas_price) = tx.gas_price() {
                res.insert("gas_price", Value::integer(gas_price as i128));
            }
        }
        TypedTransaction::Eip2930(tx) => {
            res.insert(
                "access_list",
                Value::array(format_access_list_for_display(&tx.access_list)),
            );
        }
        TypedTransaction::Eip1559(tx) => {
            res.insert(
                "access_list",
                Value::array(format_access_list_for_display(&tx.access_list)),
            );
            res.insert("max_fee_per_gas", Value::integer(tx.max_fee_per_gas as i128));
            res.insert(
                "max_priority_fee_per_gas",
                Value::integer(tx.max_priority_fee_per_gas as i128),
            );
        }
        TypedTransaction::Eip4844(_tx) => {
            unimplemented!("EIP-4844 is not supported");
        }
        TypedTransaction::Eip7702(_tx) => {
            unimplemented!("EIP-7702 is not supported");
        }
    }
    res.to_value()
}

/// Format an access list for display
pub fn format_access_list_for_display(access_list: &AccessList) -> Vec<Value> {
    access_list
        .0
        .iter()
        .map(|item| {
            ObjectType::from(vec![
                ("address", Value::string(item.address.to_string())),
                (
                    "storage_keys",
                    Value::array(
                        item.storage_keys
                            .iter()
                            .map(|key| Value::string(hex::encode(key.0)))
                            .collect::<Vec<Value>>(),
                    ),
                ),
            ])
            .to_value()
        })
        .collect::<Vec<Value>>()
}