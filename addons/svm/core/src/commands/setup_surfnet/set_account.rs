use std::{collections::HashMap, path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_account_decoder_client_types::UiAccount;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_request::RpcRequest;
use solana_pubkey::Pubkey;
use txtx_addon_kit::{
    hex,
    indexmap::IndexMap,
    types::{
        diagnostics::Diagnostic, frontend::LogDispatcher, stores::ValueStore, types::Value,
        AuthorizationContext,
    },
};
use txtx_addon_network_svm_types::SvmValue;

use crate::constants::SET_ACCOUNT;

macro_rules! parse_num {
    ($type:ty, $value:expr) => {{
        let num = <$type>::from_str($value).map_err(|e| {
            diagnosed_error!("failed to parse field_value as {}: {e}", stringify!($type))
        })?;
        num.to_le_bytes().to_vec()
    }};
}

fn field_value_to_bytes(field_type: &str, field_value: &Value) -> Result<Vec<u8>, Diagnostic> {
    match field_type {
        "u8" => Ok(parse_num!(u8, &field_value.to_string())),
        "i8" => Ok(parse_num!(i8, &field_value.to_string())),
        "u16" => Ok(parse_num!(u16, &field_value.to_string())),
        "i16" => Ok(parse_num!(i16, &field_value.to_string())),
        "u32" => Ok(parse_num!(u32, &field_value.to_string())),
        "i32" => Ok(parse_num!(i32, &field_value.to_string())),
        "u64" => Ok(parse_num!(u64, &field_value.to_string())),
        "i64" => Ok(parse_num!(i64, &field_value.to_string())),
        "u128" => Ok(parse_num!(u128, &field_value.to_string())),
        "i128" => Ok(parse_num!(i128, &field_value.to_string())),
        "f32" => Ok(parse_num!(f32, &field_value.to_string())),
        "f64" => Ok(parse_num!(f64, &field_value.to_string())),
        "pubkey" => {
            let pubkey = Pubkey::from_str(&field_value.to_string())
                .map_err(|e| diagnosed_error!("failed to parse field_value as Pubkey: {e}"))?;
            Ok(pubkey.to_bytes().to_vec())
        }
        "string" => Ok(field_value.to_string().into_bytes()),
        "boolean" => {
            let b = bool::from_str(&field_value.to_string())
                .map_err(|e| diagnosed_error!("failed to parse field_value as boolean: {e}"))?;
            Ok(vec![b as u8])
        }
        "buffer" => hex::decode(&field_value.to_string())
            .map_err(|e| diagnosed_error!("failed to parse field_value as hex string: {e}")),
        _ => Err(diagnosed_error!(
            "invalid 'field_type' field in patch_raw item: must be one of \
            'u8', 'i8', 'u16', 'i16', 'u32', 'i32', 'u64', 'i64', \
            'u128', 'i128', 'f32', 'f64', 'pubkey', 'string', 'boolean', or 'buffer'"
        )),
    }
}

fn apply_patches_raw(
    data_bytes: Option<Vec<u8>>,
    patch: &Value,
    prefetched_data: &HashMap<String, Option<Vec<u8>>>,
    public_key: &Pubkey,
) -> Result<Option<Vec<u8>>, Diagnostic> {
    let patches = patch.as_array().ok_or_else(|| {
        diagnosed_error!(
            "expected 'patch_raw' field to be an array of maps with 'offset', 'length', 'field_value', and 'field_type' fields"
        )
    })?;

    let mut data_bytes = match data_bytes {
        Some(d) => d,
        None => match prefetched_data.get(&public_key.to_string()) {
            Some(Some(d)) => d.clone(),
            Some(None) => {
                eprintln!(
                    "Warning: skipping patch_raw for account {}: account does not exist on-chain",
                    public_key
                );
                return Ok(None);
            }
            None => {
                return Err(diagnosed_error!(
                    "account data must be provided or prefetched for patching"
                ));
            }
        },
    };

    for patch_item in patches.iter() {
        let patch_map = patch_item.as_object().ok_or_else(|| {
            diagnosed_error!(
                "expected each item in 'patch_raw' array to be a map with 'offset', 'length', 'field_value', and 'field_type' fields"
            )
        })?;

        let PatchRawAccountData { offset, length, field_value, field_type } =
            PatchRawAccountData::from_map(patch_map)?;
        let range = offset as usize..(offset + length) as usize;
        let bytes = field_value_to_bytes(&field_type, &field_value)?;
        if bytes.len() != length as usize {
            return Err(diagnosed_error!(
                "patch field_type '{}' produced {} bytes, but 'length' was set to {}",
                field_type,
                bytes.len(),
                length
            ));
        }
        if (offset + length) as usize > data_bytes.len() {
            return Err(diagnosed_error!(
                "patch range {}..{} exceeds account data length ({})",
                offset,
                offset + length,
                data_bytes.len()
            ));
        }
        data_bytes[range].copy_from_slice(&bytes);
    }

    Ok(Some(data_bytes))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchRawAccountData {
    pub offset: u64,
    pub length: u64,
    pub field_value: Value,
    pub field_type: String,
}

impl PatchRawAccountData {
    pub fn new(offset: u64, length: u64, field_value: Value, field_type: String) -> Self {
        Self { offset, length, field_value, field_type }
    }

    pub fn from_map(map: &IndexMap<String, Value>) -> Result<Self, Diagnostic> {
        let get_field = |key: &str| -> Result<&Value, Diagnostic> {
            map.get(key).ok_or_else(|| diagnosed_error!("missing '{key}' field in patch_raw item"))
        };

        let offset = get_field("offset")?
            .as_uint()
            .ok_or_else(|| diagnosed_error!("expected 'offset' field in patch_raw item to be a u64"))?
            .map_err(|e| diagnosed_error!("{e}"))?;

        let length = get_field("length")?
            .as_uint()
            .ok_or_else(|| diagnosed_error!("expected 'length' field in patch_raw item to be a u64"))?
            .map_err(|e| diagnosed_error!("{e}"))?;

        let field_value = get_field("field_value")?.clone();

        let field_type = get_field("field_type")?
            .as_string()
            .ok_or_else(|| {
                diagnosed_error!("expected 'field_type' field in patch_raw item to be a string")
            })?
            .to_string();

        Ok(Self::new(offset, length, field_value, field_type))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfpoolAccountUpdate {
    // Skipping serialization of public_key to avoid sending it in the request
    // as it is already included in the request parameters.
    #[serde(skip)]
    pub public_key: Pubkey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lamports: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rent_epoch: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUpdateFile {
    pub pubkey: Option<String>,
    pub account: UiAccount,
}

impl SurfpoolAccountUpdate {
    pub fn new(
        public_key: Pubkey,
        lamports: Option<u64>,
        data: Option<String>,
        owner: Option<String>,
        executable: Option<bool>,
        rent_epoch: Option<u64>,
    ) -> Self {
        Self { public_key, lamports, data, owner, executable, rent_epoch }
    }

    pub fn from_map(
        map: &mut IndexMap<String, Value>,
        auth_ctx: &AuthorizationContext,
        prefetched_data: &HashMap<String, Option<Vec<u8>>>,
    ) -> Result<Self, Diagnostic> {
        let some_public_key = map.swap_remove("public_key");

        let some_account_path = map.swap_remove("account_path");
        if let Some(account_path) = some_account_path {
            let account_path = account_path
                .as_string()
                .ok_or_else(|| diagnosed_error!("'account_path' field must be a string"))?;
            let account_path = auth_ctx
                .get_file_location_from_path_buf(&PathBuf::from(account_path))
                .map_err(|e| diagnosed_error!("failed to get account file path: {e}"))?;

            let account_data = account_path.read_content_as_utf8().map_err(|e| {
                diagnosed_error!(
                    "invalid account file location {}: {}",
                    &account_path.to_string(),
                    e
                )
            })?;

            let account_update: AccountUpdateFile =
                serde_json::from_str(&account_data).map_err(|e| {
                    diagnosed_error!(
                        "failed to parse account file at '{}' as Solana Account: {}",
                        account_path.to_string(),
                        e
                    )
                })?;

            let pubkey = if let Some(public_key) = some_public_key {
                SvmValue::to_pubkey(&public_key)
                    .map_err(|e| diagnosed_error!("invalid 'public_key' field: {e}"))?
            } else if let Some(pubkey_str) = account_update.pubkey {
                Pubkey::from_str(&pubkey_str)
                    .map_err(|e| diagnosed_error!("invalid 'pubkey' field in account file: {e}"))?
            } else {
                return Err(diagnosed_error!(
                    "missing required 'public_key' field and 'pubkey' field in account file"
                ));
            };

            let account_update = SurfpoolAccountUpdate::new(
                pubkey,
                Some(account_update.account.lamports),
                account_update.account.data.decode().map(|d| hex::encode(d)),
                Some(account_update.account.owner.to_string()),
                Some(account_update.account.executable),
                Some(account_update.account.rent_epoch),
            );
            Ok(account_update)
        } else {
            let public_key = some_public_key
                .map(|p| {
                    SvmValue::to_pubkey(&p)
                        .map_err(|e| diagnosed_error!("invalid 'public_key' field: {e}"))
                })
                .ok_or_else(|| diagnosed_error!("missing required 'public_key'"))??;
            let some_lamports = map.swap_remove("lamports");
            let lamports = some_lamports
                .map(|v| {
                    v.as_uint()
                        .map(|r| r.map_err(|e| diagnosed_error!("{e}")))
                        .ok_or_else(|| diagnosed_error!("expected 'lamports' field to be a u64"))
                })
                .transpose()?
                .transpose()?;

            let some_data = map.swap_remove("data");
            let data_bytes = some_data.map(|v| v.to_le_bytes());

            let some_owner = map.swap_remove("owner");
            let owner = some_owner
                .map(|p| {
                    SvmValue::to_pubkey(&p)
                        .map_err(|e| diagnosed_error!("invalid 'owner' field: {e}"))
                        .map(|p| p.to_string())
                })
                .transpose()?;

            let some_executable = map.swap_remove("executable");
            let executable = some_executable
                .map(|v| {
                    v.as_bool().ok_or_else(|| {
                        diagnosed_error!("expected 'executable' field to be a boolean")
                    })
                })
                .transpose()?;

            let some_rent_epoch = map.swap_remove("rent_epoch");
            let rent_epoch = some_rent_epoch
                .map(|v| {
                    v.as_uint()
                        .map(|r| r.map_err(|e| diagnosed_error!("{e}")))
                        .ok_or_else(|| diagnosed_error!("expected 'rent_epoch' field to be a u64"))
                })
                .transpose()?
                .transpose()?;

            let data_bytes = if let Some(patch) = map.swap_remove("patch_raw") {
                apply_patches_raw(data_bytes, &patch, prefetched_data, &public_key)?
            } else {
                data_bytes
            };

            let data = data_bytes.map(hex::encode);

            if lamports.is_none()
                && data.is_none()
                && owner.is_none()
                && executable.is_none()
                && rent_epoch.is_none()
            {
                return Err(diagnosed_error!("at least one of 'lamports', 'data', 'owner', 'executable', or 'rent_epoch' must be provided"));
            }
            Ok(Self::new(public_key, lamports, data, owner, executable, rent_epoch))
        }
    }

    fn get_account_update_maps(
        values: &ValueStore,
    ) -> Result<Option<Vec<IndexMap<String, Value>>>, Diagnostic> {
        let account_update_data = match values.get_value(SET_ACCOUNT) {
            None => return Ok(None),
            Some(v) => {
                v.as_map().ok_or_else(|| diagnosed_error!("'set_account' must be a map type"))?
            }
        };

        let maps = account_update_data
            .iter()
            .map(|i| {
                i.as_object()
                    .cloned()
                    .ok_or_else(|| diagnosed_error!("'set_account' must be a map type"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(maps))
    }

    /// Checks if any of the account updates require prefetched data and fetches it if needed.
    /// Returns a map of account public keys to their prefetched data.
    pub async fn get_accounts_data_if_needed(
        values: &ValueStore,
        rpc_client: &RpcClient,
    ) -> Result<HashMap<String, Option<Vec<u8>>>, Diagnostic> {
        let account_updates = match Self::get_account_update_maps(values)? {
            None => return Ok(HashMap::new()),
            Some(maps) => maps,
        };

        let accounts_to_fetch = account_updates
            .iter()
            .enumerate()
            .filter_map(|(i, update)| {
                update.get("patch_raw")?;
                let prefix = format!("failed to parse `set_account` map #{}", i + 1);
                Some(
                    update
                        .get("public_key")
                        .ok_or_else(|| {
                            diagnosed_error!(
                                "{prefix} missing required 'public_key' field and 'pubkey' field in account file"
                            )
                        })
                        .and_then(|pk| {
                            SvmValue::to_pubkey(pk).map_err(|e| {
                                diagnosed_error!(
                                    "{prefix} invalid 'public_key' field: {e}"
                                )
                            })
                        }),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        if accounts_to_fetch.is_empty() {
            return Ok(HashMap::new());
        }

        let accounts_data = rpc_client
            .get_multiple_accounts(&accounts_to_fetch)
            .await
            .map_err(|e| diagnosed_error!("failed to prefetch account infos: {e}"))?
            .into_iter()
            .zip(accounts_to_fetch.iter())
            .map(|(acc, pubkey)| (pubkey.to_string(), acc.map(|a| a.data)))
            .collect();

        Ok(accounts_data)
    }

    pub fn parse_value_store(
        values: &ValueStore,
        auth_ctx: &AuthorizationContext,
        prefetched_data: HashMap<String, Option<Vec<u8>>>,
    ) -> Result<Vec<Self>, Diagnostic> {
        let mut account_update_data = match Self::get_account_update_maps(values)? {
            None => return Ok(vec![]),
            Some(maps) => maps,
        };

        let mut account_updates = vec![];

        for (i, account_update) in account_update_data.iter_mut().enumerate() {
            let prefix = format!("failed to parse `set_account` map #{}", i + 1);
            let account =
                SurfpoolAccountUpdate::from_map(account_update, auth_ctx, &prefetched_data)
                    .map_err(|e| diagnosed_error!("{prefix}: {e}"))?;

            account_updates.push(account);
        }

        Ok(account_updates)
    }

    fn to_request_params(&self) -> serde_json::Value {
        let pubkey = json![self.public_key.to_string()];
        let account_update = serde_json::to_value(&self).unwrap();
        json!(vec![pubkey, account_update])
    }

    fn rpc_method() -> &'static str {
        "surfnet_setAccount"
    }

    fn update_status(&self, logger: &LogDispatcher, index: usize, total: usize) {
        logger.success_info(
            "Account Updated",
            &format!(
                "Processed surfpool account update #{}/{} for {}",
                index + 1,
                total,
                self.public_key.to_string()
            ),
        );
    }

    pub async fn send_request(
        &self,
        rpc_client: &RpcClient,
    ) -> Result<serde_json::Value, Diagnostic> {
        rpc_client
            .send::<serde_json::Value>(
                RpcRequest::Custom { method: Self::rpc_method() },
                self.to_request_params(),
            )
            .await
            .map_err(|e| diagnosed_error!("`{}` RPC call failed: {e}", Self::rpc_method()))
    }

    pub async fn process_updates(
        account_updates: Vec<Self>,
        rpc_client: &RpcClient,
        logger: &LogDispatcher,
    ) -> Result<(), Diagnostic> {
        for (i, account_update) in account_updates.iter().enumerate() {
            let _ = account_update.send_request(rpc_client).await?;
            account_update.update_status(logger, i, account_updates.len());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use solana_pubkey::pubkey;

    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_parse_num() -> Result<(), Diagnostic> {
        assert_eq!(parse_num!(u8, "255"), vec![255]);
        assert_eq!(parse_num!(i8, "-128"), vec![128]);
        assert_eq!(parse_num!(u16, "65535"), vec![255, 255]);
        assert_eq!(parse_num!(i16, "-32768"), vec![0, 128]);
        assert_eq!(parse_num!(u32, "4294967295"), vec![255, 255, 255, 255]);
        assert_eq!(parse_num!(i32, "-2147483648"), vec![0, 0, 0, 128]);
        assert_eq!(
            parse_num!(u64, "18446744073709551615"),
            vec![255, 255, 255, 255, 255, 255, 255, 255]
        );
        assert_eq!(parse_num!(i64, "-9223372036854775808"), vec![0, 0, 0, 0, 0, 0, 0, 128]);
        assert_eq!(parse_num!(u128, "340282366920938463463374607431768211455"), vec![255; 16]);
        assert_eq!(parse_num!(i128, "0"), vec![0; 16]);
        assert_eq!(parse_num!(f32, "3.14"), 3.14f32.to_le_bytes().to_vec());
        assert_eq!(parse_num!(f64, "3.14"), 3.14f64.to_le_bytes().to_vec());
        Ok(())
    }

    #[test]
    fn test_patch_raw_account_data_from_map() -> Result<(), Diagnostic> {
        let mut map = IndexMap::new();
        map.insert("offset".to_string(), Value::Integer(0));
        map.insert("length".to_string(), Value::Integer(4));
        map.insert("field_value".to_string(), Value::String("255".to_string()));
        map.insert("field_type".to_string(), Value::String("u32".to_string()));

        let patch_data = PatchRawAccountData::from_map(&map)?;
        assert_eq!(patch_data.offset, 0);
        assert_eq!(patch_data.length, 4);
        assert_eq!(patch_data.field_value, Value::String("255".to_string()));
        assert_eq!(patch_data.field_type, "u32");
        Ok(())
    }

    #[test]
    fn test_surfpool_account_update_from_map_with_patch_application() -> Result<(), Diagnostic> {
        let mut map = IndexMap::new();
        const PUBKEY: Pubkey = pubkey!("11111111111111111111111111111111");
        map.insert("public_key".to_string(), SvmValue::pubkey(PUBKEY.to_bytes().to_vec()));

        let patch = Value::Array(Box::new(vec![Value::Object({
            let mut m = IndexMap::new();
            m.insert("offset".to_string(), Value::Integer(0));
            m.insert("length".to_string(), Value::Integer(4));
            m.insert("field_value".to_string(), Value::String("1".to_string()));
            m.insert("field_type".to_string(), Value::String("u32".to_string()));
            m
        })]));

        map.insert("patch_raw".to_string(), patch);

        let auth_ctx = AuthorizationContext::empty();
        let mut prefetched_data = HashMap::new();

        prefetched_data.insert(PUBKEY.to_string(), Some(vec![0; 8]));
        let account_update =
            SurfpoolAccountUpdate::from_map(&mut map, &auth_ctx, &prefetched_data)?;
        assert_eq!(account_update.public_key.to_string(), PUBKEY.to_string());
        assert_eq!(account_update.data, Some("0100000000000000".to_string()));
        Ok(())
    }

    #[test]
    fn test_apply_multiple_patches_raw() -> Result<(), Diagnostic> {
        let patch = Value::array(vec![
            Value::object({
                let mut m = IndexMap::new();
                m.insert("offset".to_string(), Value::Integer(0));
                m.insert("length".to_string(), Value::Integer(4));
                m.insert("field_value".to_string(), Value::String("100".to_string()));
                m.insert("field_type".to_string(), Value::String("u32".to_string()));
                m
            }),
            Value::object({
                let mut m = IndexMap::new();
                m.insert("offset".to_string(), Value::Integer(4));
                m.insert("length".to_string(), Value::Integer(4));
                m.insert("field_value".to_string(), Value::String("200".to_string()));
                m.insert("field_type".to_string(), Value::String("u32".to_string()));
                m
            }),
            Value::object({
                let mut m = IndexMap::new();
                m.insert("offset".to_string(), Value::Integer(8));
                m.insert("length".to_string(), Value::Integer(1));
                m.insert("field_value".to_string(), Value::String("true".to_string()));
                m.insert("field_type".to_string(), Value::String("boolean".to_string()));
                m
            }),
        ]);

        let data = Some(vec![0u8; 16]);
        let prefetched = HashMap::new();
        let pubkey = pubkey!("11111111111111111111111111111111");

        let result = apply_patches_raw(data, &patch, &prefetched, &pubkey)?.unwrap();

        assert_eq!(&result[0..4], &100u32.to_le_bytes());
        assert_eq!(&result[4..8], &200u32.to_le_bytes());
        assert_eq!(result[8], 1u8);
        assert_eq!(&result[9..16], &[0u8; 7]);
        Ok(())
    }

    #[test]
    fn test_apply_patches_raw_uses_provided_data_over_prefetched() -> Result<(), Diagnostic> {
        let patch = Value::array(vec![Value::object({
            let mut m = IndexMap::new();
            m.insert("offset".to_string(), Value::Integer(0));
            m.insert("length".to_string(), Value::Integer(1));
            m.insert("field_value".to_string(), Value::String("42".to_string()));
            m.insert("field_type".to_string(), Value::String("u8".to_string()));
            m
        })]);

        let pubkey = pubkey!("11111111111111111111111111111111");

        let provided_data = Some(vec![0xFF; 4]);
        let mut prefetched = HashMap::new();
        prefetched.insert(pubkey.to_string(), Some(vec![0xAA; 4]));

        let result = apply_patches_raw(provided_data, &patch, &prefetched, &pubkey)?.unwrap();
        assert_eq!(result[0], 42);
        assert_eq!(&result[1..4], &[0xFF; 3]);
        Ok(())
    }

    #[test]
    fn test_apply_patches_raw_falls_back_to_prefetched() -> Result<(), Diagnostic> {
        let patch = Value::array(vec![Value::object({
            let mut m = IndexMap::new();
            m.insert("offset".to_string(), Value::Integer(0));
            m.insert("length".to_string(), Value::Integer(1));
            m.insert("field_value".to_string(), Value::String("42".to_string()));
            m.insert("field_type".to_string(), Value::String("u8".to_string()));
            m
        })]);

        let pubkey = pubkey!("11111111111111111111111111111111");

        let mut prefetched = HashMap::new();
        prefetched.insert(pubkey.to_string(), Some(vec![0xBB; 4]));

        let result = apply_patches_raw(None, &patch, &prefetched, &pubkey)?.unwrap();
        assert_eq!(result[0], 42);
        assert_eq!(&result[1..4], &[0xBB; 3]);
        Ok(())
    }

    #[test]
    fn test_apply_patches_raw_skips_nonexistent_account() -> Result<(), Diagnostic> {
        let patch = Value::array(vec![Value::object({
            let mut m = IndexMap::new();
            m.insert("offset".to_string(), Value::Integer(0));
            m.insert("length".to_string(), Value::Integer(1));
            m.insert("field_value".to_string(), Value::String("42".to_string()));
            m.insert("field_type".to_string(), Value::String("u8".to_string()));
            m
        })]);

        let pubkey = pubkey!("11111111111111111111111111111111");

        let mut prefetched = HashMap::new();
        prefetched.insert(pubkey.to_string(), None);

        let result = apply_patches_raw(None, &patch, &prefetched, &pubkey)?;
        assert_eq!(result, None);
        Ok(())
    }

    #[test]
    fn test_apply_patches_raw_errors_when_not_prefetched() {
        let patch = Value::array(vec![Value::object({
            let mut m = IndexMap::new();
            m.insert("offset".to_string(), Value::Integer(0));
            m.insert("length".to_string(), Value::Integer(1));
            m.insert("field_value".to_string(), Value::String("42".to_string()));
            m.insert("field_type".to_string(), Value::String("u8".to_string()));
            m
        })]);

        let pubkey = pubkey!("11111111111111111111111111111111");

        let prefetched = HashMap::new();

        let result = apply_patches_raw(None, &patch, &prefetched, &pubkey);
        assert!(result.is_err());
    }
}
