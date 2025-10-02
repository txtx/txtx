use crate::codec::DeploymentTransaction;
use crate::typing::{
    SVM_CLOSE_TEMP_AUTHORITY_TRANSACTION_PARTS, SVM_DEPLOYMENT_TRANSACTION, SVM_TRANSACTION,
};
use solana_transaction::Transaction;
use txtx_addon_kit::hex;
use txtx_addon_kit::types::{diagnostics::Diagnostic, types::Value};

fn is_hex(str: &str) -> bool {
    decode_hex(str).map(|_| true).unwrap_or(false)
}

pub fn decode_hex(str: &str) -> Result<Vec<u8>, Diagnostic> {
    let stripped = if str.starts_with("0x") { &str[2..] } else { &str[..] };
    hex::decode(stripped)
        .map_err(|e| diagnosed_error!("string '{}' could not be decoded to hex bytes: {}", str, e))
}

pub fn build_transaction_from_svm_value(value: &Value) -> Result<Transaction, Diagnostic> {
    match value {
        Value::String(s) => {
            if is_hex(s) {
                let hex = decode_hex(s)?;
                return serde_json::from_slice(&hex)
                    .map_err(|e| diagnosed_error!("could not deserialize transaction: {e}"));
            }
            return serde_json::from_str(s)
                .map_err(|e| diagnosed_error!("could not deserialize transaction: {e}"));
        }
        Value::Addon(addon_data) => {
            if addon_data.id == SVM_TRANSACTION {
                return serde_json::from_slice(&addon_data.bytes)
                    .map_err(|e| diagnosed_error!("could not deserialize transaction: {e}"));
            } else if addon_data.id == SVM_DEPLOYMENT_TRANSACTION
                || addon_data.id == SVM_CLOSE_TEMP_AUTHORITY_TRANSACTION_PARTS
            {
                let deployment_transaction = DeploymentTransaction::from_value(value)
                    .map_err(|e| diagnosed_error!("could not deserialize transaction: {e}"))?;
                return Ok(deployment_transaction.transaction.as_ref().unwrap().clone());
            } else {
                return Err(diagnosed_error!(
                    "could not deserialize addon type '{}' into transaction",
                    addon_data.id
                ));
            }
        }
        _ => {
            return Err(diagnosed_error!(
                "could not deserialize transaction: expected string or addon"
            ))
        }
    };
}
