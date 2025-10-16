use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_request::RpcRequest;
use solana_pubkey::Pubkey;
use spl_associated_token_account_interface::address::get_associated_token_address_with_program_id;

use serde::de::Visitor;
use serde::{Deserializer, Serializer};
use std::fmt;
use txtx_addon_kit::types::frontend::LogDispatcher;
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{diagnostics::Diagnostic, stores::ValueStore, types::Value},
};
use txtx_addon_network_svm_types::SvmValue;

use crate::constants::SET_TOKEN_ACCOUNT;

use super::tokens::get_token_by_name;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenProgram {
    Token2020,
    Token2022,
    Custom(Pubkey),
}

impl Default for TokenProgram {
    fn default() -> Self {
        TokenProgram::Token2020
    }
}

impl TokenProgram {
    pub fn pubkey(&self) -> Pubkey {
        match self {
            TokenProgram::Token2020 => spl_token_interface::ID,
            TokenProgram::Token2022 => spl_token_2022_interface::ID,
            TokenProgram::Custom(pubkey) => *pubkey,
        }
    }

    pub fn from_value(value: Value) -> Result<Self, Diagnostic> {
        if let Some(s) = value.as_string() {
            match s {
                "token2020" => return Ok(TokenProgram::Token2020),
                "token2022" => return Ok(TokenProgram::Token2022),
                _ => {}
            }
        }
        SvmValue::to_pubkey(&value).map(|p| TokenProgram::Custom(p)).map_err(|_| {
            diagnosed_error!("'token_program' field is not a known token program or a valid pubkey")
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfpoolTokenAccountUpdate {
    #[serde(skip)]
    pub public_key: Pubkey,
    #[serde(skip)]
    pub token: Pubkey,
    pub token_program: TokenProgram,
    #[serde(skip)]
    pub associated_token_account: Pubkey,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegate: Option<SetSomeAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegated_amount: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_authority: Option<SetSomeAccount>,
}

impl SurfpoolTokenAccountUpdate {
    pub fn new(
        public_key: Pubkey,
        token: Pubkey,
        token_program: TokenProgram,
        amount: Option<u64>,
        delegate: Option<SetSomeAccount>,
        state: Option<String>,
        delegated_amount: Option<u64>,
        close_authority: Option<SetSomeAccount>,
    ) -> Self {
        let associated_token_account = get_associated_token_address_with_program_id(
            &public_key,
            &token,
            &token_program.pubkey(),
        );

        Self {
            public_key,
            token,
            token_program,
            associated_token_account,
            amount,
            delegate,
            state,
            delegated_amount,
            close_authority,
        }
    }

    pub fn from_map(map: &mut IndexMap<String, Value>) -> Result<Self, Diagnostic> {
        let some_public_key = map.swap_remove("public_key");
        let public_key = some_public_key
            .map(|p| {
                SvmValue::to_pubkey(&p)
                    .map_err(|e| diagnosed_error!("invalid 'public_key' field: {e}"))
            })
            .ok_or_else(|| diagnosed_error!("missing required 'public_key'"))??;

        let some_token = map.swap_remove("token");
        let token = some_token
            .map(|p| {
                if let Some(s) = p.as_string() {
                    let token_address = get_token_by_name("mainnet", s); // todo: pass in upstream network id for surfpool
                    if let Some(token_address) = token_address {
                        return Ok(token_address);
                    }
                }
                SvmValue::to_pubkey(&p).map_err(|_| {
                    diagnosed_error!("'token' field is not a known token or a valid pubkey")
                })
            })
            .ok_or_else(|| diagnosed_error!("missing required 'token'"))??;

        let some_token_program = map.swap_remove("token_program");
        let token_program =
            some_token_program.map(TokenProgram::from_value).transpose()?.unwrap_or_default();

        let some_amount = map.swap_remove("amount");
        let amount = some_amount
            .map(|v| {
                v.as_uint()
                    .map(|r| r.map_err(|e| diagnosed_error!("{e}")))
                    .ok_or_else(|| diagnosed_error!("expected 'amount' field to be a u64"))
            })
            .transpose()?
            .transpose()?;

        let some_delegate = map.swap_remove("delegate");
        let delegate = some_delegate
            .map(|p| {
                if p.as_null().is_some() {
                    return Ok(SetSomeAccount::NoAccount);
                }
                SvmValue::to_pubkey(&p)
                    .map_err(|e| diagnosed_error!("invalid 'delegate' field: {e}"))
                    .map(|p| SetSomeAccount::Account(p.to_string()))
            })
            .transpose()?;

        let some_state = map.swap_remove("state");
        let state = some_state
            .map(|v| {
                v.as_string()
                    .map(|s| match s {
                        "uninitialized" | "initialized" | "frozen" => Ok(s.to_string()),
                        _ => Err(diagnosed_error!("invalid 'state' field value: expected 'uninitialized', 'initialized', or 'frozen'")),
                    })
                    .ok_or_else(|| diagnosed_error!("expected 'state' field to be a string"))
            })
            .transpose()?
            .transpose()?;

        let some_delegated_amount = map.swap_remove("delegated_amount");
        let delegated_amount = some_delegated_amount
            .map(|v| {
                v.as_uint().map(|r| r.map_err(|e| diagnosed_error!("{e}"))).ok_or_else(|| {
                    diagnosed_error!("expected 'delegated_amount' field to be a u64")
                })
            })
            .transpose()?
            .transpose()?;

        let some_close_authority = map.swap_remove("close_authority");
        let close_authority = some_close_authority
            .map(|p| {
                if p.as_null().is_some() {
                    return Ok(SetSomeAccount::NoAccount);
                }
                SvmValue::to_pubkey(&p)
                    .map_err(|e| diagnosed_error!("invalid 'close_authority' field: {e}"))
                    .map(|p| SetSomeAccount::Account(p.to_string()))
            })
            .transpose()?;

        if amount.is_none()
            && delegate.is_none()
            && state.is_none()
            && delegated_amount.is_none()
            && close_authority.is_none()
        {
            return Err(diagnosed_error!("at least one of 'amount', 'delegate', 'state', 'delegated_amount', or 'close_authority' must be provided"));
        }
        Ok(Self::new(
            public_key,
            token,
            token_program,
            amount,
            delegate,
            state,
            delegated_amount,
            close_authority,
        ))
    }

    pub fn parse_value_store(values: &ValueStore) -> Result<Vec<Self>, Diagnostic> {
        let mut account_updates = vec![];

        let account_update_data = values
            .get_value(SET_TOKEN_ACCOUNT)
            .map(|v| {
                v.as_map().ok_or_else(|| diagnosed_error!("'set_token_account' must be a map type"))
            })
            .transpose()?;

        let Some(account_update_data) = account_update_data else {
            return Ok(vec![]);
        };

        let mut account_update_data = account_update_data
            .iter()
            .map(|i| {
                i.as_object()
                    .map(|o| o.clone())
                    .ok_or(diagnosed_error!("'set_token_account' must be a map type"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        for (i, account_update) in account_update_data.iter_mut().enumerate() {
            let prefix = format!("failed to parse `set_token_account` map #{}", i + 1);
            let account = SurfpoolTokenAccountUpdate::from_map(account_update)
                .map_err(|e| diagnosed_error!("{prefix}: {e}"))?;

            account_updates.push(account);
        }

        Ok(account_updates)
    }

    fn to_request_params(&self) -> serde_json::Value {
        let pubkey = json![self.public_key.to_string()];
        let token = json![self.token.to_string()];
        let token_program = json![self.token_program.pubkey().to_string()];
        let account_update = serde_json::to_value(&self).unwrap();
        json!(vec![pubkey, token, account_update, token_program])
    }

    fn rpc_method() -> &'static str {
        "surfnet_setTokenAccount"
    }

    fn update_status(&self, logger: &LogDispatcher, index: usize, total: usize) {
        logger.success_info(
            "Token Account Updated",
            &format!(
                "Processed surfpool token account update #{}/{} for {}",
                index + 1,
                total,
                self.associated_token_account.to_string()
            ),
        );
    }

    async fn send_request(&self, rpc_client: &RpcClient) -> Result<serde_json::Value, Diagnostic> {
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

#[derive(Debug, Clone)]
pub enum SetSomeAccount {
    Account(String),
    NoAccount,
}

impl<'de> Deserialize<'de> for SetSomeAccount {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SetSomeAccountVisitor;

        impl<'de> Visitor<'de> for SetSomeAccountVisitor {
            type Value = SetSomeAccount;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a Pubkey String or the String 'null'")
            }

            fn visit_some<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer).map(|v: String| match v.as_str() {
                    "null" => SetSomeAccount::NoAccount,
                    _ => SetSomeAccount::Account(v.to_string()),
                })
            }
        }

        deserializer.deserialize_option(SetSomeAccountVisitor)
    }
}

impl Serialize for SetSomeAccount {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            SetSomeAccount::Account(val) => serializer.serialize_str(&val),
            SetSomeAccount::NoAccount => serializer.serialize_str("null"),
        }
    }
}
