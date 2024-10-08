use crate::codec::codec::{StacksTransaction, TransactionPayload};
use async_recursion::async_recursion;
use clarity::util::hash::bytes_to_hex;
use clarity::vm::types::QualifiedContractIdentifier;
use clarity_repl::clarity::codec::StacksMessageCodec;
use clarity_repl::clarity::util::hash::{hex_bytes, to_hex};
use clarity_repl::clarity::vm::types::Value;
use serde_json::Value as JsonValue;
use std::io::Cursor;
use txtx_addon_kit::helpers::format_currency;
use txtx_addon_kit::reqwest::header::{HeaderMap, AUTHORIZATION};

use serde_json::json;
use txtx_addon_kit::reqwest::Client;

#[derive(Debug)]
pub enum RpcError {
    Generic,
    StatusCode(u16),
    Message(String),
    ContractAlreadyDeployed(String),
    ContractIdAlreadyUsed(String),
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            RpcError::Message(e) => write!(f, "{}", e),
            RpcError::ContractAlreadyDeployed(e) => {
                write!(f, "contract already deployed {:?}", e)
            }
            RpcError::ContractIdAlreadyUsed(e) => {
                write!(
                    f,
                    "a different contract with the same contract identifier already exists {:?}",
                    e
                )
            }
            RpcError::StatusCode(e) => write!(f, "error status code {}", e),
            RpcError::Generic => write!(f, "unknown error"),
        }
    }
}

pub struct StacksRpc {
    pub url: String,
    pub client: Client,
}

pub struct PostTransactionResult {
    pub txid: String,
}

pub struct CallReadOnlyFnResult {
    pub result: Value,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct NodeInfo {
    pub peer_version: u64,
    pub pox_consensus: String,
    pub burn_block_height: u64,
    pub stable_pox_consensus: String,
    pub stable_burn_block_height: u64,
    pub server_version: String,
    pub network_id: u32,
    pub parent_network_id: u32,
    pub stacks_tip_height: u64,
    pub stacks_tip: String,
    pub stacks_tip_consensus_hash: String,
    pub genesis_chainstate_hash: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PoxInfo {
    pub contract_id: String,
    pub pox_activation_threshold_ustx: u64,
    pub first_burnchain_block_height: u32,
    pub current_burnchain_block_height: u32,
    pub prepare_phase_block_length: u32,
    pub reward_phase_block_length: u32,
    pub reward_slots: u32,
    pub reward_cycle_id: u32,
    pub reward_cycle_length: u32,
    pub total_liquid_supply_ustx: u64,
    pub current_cycle: CurrentPoxCycle,
    pub next_cycle: NextPoxCycle,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct CurrentPoxCycle {
    pub id: u64,
    pub min_threshold_ustx: u64,
    pub stacked_ustx: u64,
    pub is_pox_active: bool,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct NextPoxCycle {
    pub min_threshold_ustx: u64,
    pub stacked_ustx: u64,
    pub blocks_until_prepare_phase: i16,
    pub blocks_until_reward_phase: i16,
}

#[derive(Deserialize, Debug)]
pub struct Balance {
    #[serde(rename = "balance")]
    pub balance_hex: String,
    #[serde(skip)]
    pub balance: u128,
    pub nonce: u64,
    pub balance_proof: String,
    pub nonce_proof: String,
}

impl Balance {
    pub fn get_formatted_balance(&self) -> String {
        format_currency(self.balance, 6, "STX")
    }
}

#[derive(Deserialize, Debug)]
pub struct Contract {
    pub source: String,
    pub publish_height: u64,
}

#[derive(Deserialize, Debug)]
pub struct FeeEstimationReport {
    pub estimations: Vec<FeeEstimation>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct GetNodeInfoResponse {
    pub burn_block_height: u64,
    pub stable_burn_block_height: u64,
    pub server_version: String,
    pub network_id: u32,
    pub parent_network_id: u32,
    pub stacks_tip_height: u64,
    pub stacks_tip: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct PostTransactionResponseError {
    pub txid: String,
    pub error: Option<String>,
    pub reason: Option<String>,
    pub reason_data: Option<JsonValue>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct GetTransactionResponse {
    pub tx_id: String,
    pub tx_status: TransactionStatus,
    pub tx_result: GetTransactionResult,
    pub block_height: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Success,
    AbortByResponse,
    AbortByPostCondition,
}

impl Default for TransactionStatus {
    fn default() -> Self {
        TransactionStatus::Success
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct GetTransactionResult {
    pub hex: String,
    pub repr: String,
}

#[derive(Deserialize, Debug)]
pub struct FeeEstimation {
    pub fee: u64,
}

impl StacksRpc {
    pub fn new(url: &str, auth_token: &Option<String>) -> Self {
        let mut default_headers = HeaderMap::new();
        if let Some(auth_token) = auth_token {
            default_headers
                .insert(AUTHORIZATION, format!("Bearer {}", auth_token).parse().unwrap());
        }
        Self {
            url: url.into(),
            client: Client::builder().default_headers(default_headers).build().unwrap(),
        }
    }

    #[cfg(not(feature = "wasm"))]
    #[async_recursion]
    pub async fn estimate_transaction_fee(
        &self,
        transaction_payload: &TransactionPayload,
        priority: usize,
        default_to_transaction_payload: &TransactionPayload,
    ) -> Result<u64, RpcError> {
        let tx = transaction_payload.serialize_to_vec();
        let payload = json!({ "transaction_payload": to_hex(&tx) });
        let path = format!("{}/v2/fees/transaction", self.url);
        let res = self
            .client
            .post(path)
            .json(&payload)
            .send()
            .await
            .map_err(|e| RpcError::Message(e.to_string()))?;

        if !res.status().is_success() {
            let err = match res.text().await {
                Ok(message) => {
                    if message.contains("NoEstimateAvailable") {
                        if tx.eq(&default_to_transaction_payload.serialize_to_vec()) {
                            return Ok(2100);
                        } else {
                            return self
                                .estimate_transaction_fee(
                                    &default_to_transaction_payload,
                                    priority,
                                    &default_to_transaction_payload,
                                )
                                .await;
                        }
                    } else {
                        RpcError::Message(message)
                    }
                }
                Err(e) => RpcError::Message(e.to_string()),
            };
            return Err(err);
        }

        let fee_report: FeeEstimationReport =
            res.json().await.map_err(|e| RpcError::Message(e.to_string()))?;

        Ok(fee_report.estimations[priority].fee)
    }

    pub async fn post_transaction(
        &self,
        transaction: &Vec<u8>,
    ) -> Result<PostTransactionResult, RpcError> {
        let path = format!("{}/v2/transactions", self.url);
        let res = self
            .client
            .post(path)
            .header("Content-Type", "application/octet-stream")
            .body(transaction.clone())
            .send()
            .await
            .map_err(|e| RpcError::Message(e.to_string()))?;

        if !res.status().is_success() {
            let error_ser = match res.text().await {
                Ok(value) => value,
                Err(e) => return Err(RpcError::Message(e.to_string())),
            };
            let err = if let Ok(error) =
                serde_json::from_str::<PostTransactionResponseError>(&error_ser)
            {
                match error.reason {
                    Some(r) if r.eq("ContractAlreadyExists") => {
                        let contract_identifier = error
                            .reason_data
                            .clone()
                            .map(|ref d| {
                                d.get("contract_identifier")
                                    .map(|c| c.as_str().unwrap().to_string())
                            })
                            .unwrap();
                        // Get Contract
                        if let Some(contract_identifier_src) = contract_identifier {
                            let contract_identifier =
                                QualifiedContractIdentifier::parse(&contract_identifier_src)
                                    .expect("unable to parse contract_identifier");
                            let res = self
                                .get_contract_source(
                                    &contract_identifier.issuer.to_address(),
                                    &contract_identifier.name.to_string(),
                                )
                                .await
                                .map_err(|e| RpcError::Message(e.to_string()))?;

                            // Retrieve the transaction sent
                            let transaction_bytes = transaction.clone();
                            let transaction = StacksTransaction::consensus_deserialize(
                                &mut &transaction_bytes[..],
                            )
                            .unwrap();

                            let TransactionPayload::SmartContract(source, _) = transaction.payload
                            else {
                                unreachable!()
                            };

                            // Compare
                            if res.source.eq(&source.code_body.clone().to_string()) {
                                RpcError::ContractAlreadyDeployed(contract_identifier_src)
                            } else {
                                RpcError::ContractIdAlreadyUsed(contract_identifier_src)
                            }
                        } else {
                            RpcError::Message(format!(
                                "{}: {}",
                                r.clone(),
                                error.reason_data.unwrap().to_string()
                            ))
                        }
                    }
                    Some(r) if error.reason_data.is_some() => RpcError::Message(format!(
                        "{}: {}",
                        r.clone(),
                        error.reason_data.unwrap().to_string()
                    )),

                    Some(r) => RpcError::Message(r.clone()),
                    None => RpcError::Message(format!("{:?}", error)),
                }
            } else {
                RpcError::Message(error_ser)
            };
            return Err(err);
        }

        let txid: String = res.json().await.map_err(|e| RpcError::Message(e.to_string()))?;
        let res = PostTransactionResult { txid };
        Ok(res)
    }

    pub async fn get_nonce(&self, address: &str) -> Result<u64, RpcError> {
        let balance = self.get_balance(address).await?;
        let nonce = balance.nonce;
        Ok(nonce)
    }

    pub async fn get_balance(&self, address: &str) -> Result<Balance, RpcError> {
        let request_url =
            format!("{}/v2/accounts/{addr}?unanchored=true", self.url, addr = address,);

        let mut res: Balance = self
            .client
            .get(request_url)
            .send()
            .await
            .map_err(|e| RpcError::Message(e.to_string()))?
            .json()
            .await
            .map_err(|e| RpcError::Message(e.to_string()))?;
        let balance_bytes = txtx_addon_kit::hex::decode(&res.balance_hex[2..]).unwrap();

        let mut bytes = [0u8; 16];
        let offset = 16 - balance_bytes.len();
        for (i, &byte) in balance_bytes.iter().enumerate() {
            bytes[offset + i] = byte;
        }
        res.balance = u128::from_be_bytes(bytes);
        Ok(res)
    }

    pub async fn get_pox_info(&self) -> Result<PoxInfo, RpcError> {
        let request_url = format!("{}/v2/pox", self.url);

        self.client
            .get(request_url)
            .send()
            .await
            .map_err(|e| RpcError::Message(e.to_string()))?
            .json::<PoxInfo>()
            .await
            .map_err(|e| RpcError::Message(e.to_string()))
    }

    pub async fn get_info(&self) -> Result<NodeInfo, RpcError> {
        let request_url = format!("{}/v2/info", self.url);

        self.client
            .get(request_url)
            .send()
            .await
            .map_err(|e| RpcError::Message(e.to_string()))?
            .json::<NodeInfo>()
            .await
            .map_err(|e| RpcError::Message(e.to_string()))
    }

    pub async fn get_tx(&self, txid: &str) -> Result<GetTransactionResponse, RpcError> {
        let request_url = format!("{}/extended/v1/tx/{}", self.url, txid);

        self.client
            .get(request_url)
            .send()
            .await
            .map_err(|e| RpcError::Message(e.to_string()))?
            .json::<GetTransactionResponse>()
            .await
            .map_err(|e| RpcError::Message(e.to_string()))
    }

    pub async fn get_contract_source(
        &self,
        principal: &str,
        contract_name: &str,
    ) -> Result<Contract, RpcError> {
        let request_url =
            format!("{}/v2/contracts/source/{}/{}", self.url, principal, contract_name);

        let res = self.client.get(request_url).send().await;

        match res {
            Ok(response) => match response.json().await {
                Ok(value) => Ok(value),
                Err(e) => Err(RpcError::Message(e.to_string())),
            },
            Err(e) => Err(RpcError::Message(e.to_string())),
        }
    }

    pub async fn call_readonly_fn_fn(
        &self,
        contract_addr: &str,
        contract_name: &str,
        method: &str,
        args: Vec<Value>,
        sender: &str,
    ) -> Result<Value, RpcError> {
        let path = format!(
            "{}/v2/contracts/call-read/{}/{}/{}",
            self.url, contract_addr, contract_name, method
        );
        let mut arguments = vec![];
        for arg in args.iter() {
            let bytes =
                arg.serialize_to_vec().map_err(|e| RpcError::Message(format!("{:?}", e)))?;
            arguments.push(bytes_to_hex(&bytes));
        }
        let res = self
            .client
            .post(path)
            .json(&json!({
                "sender": sender,
                "arguments": arguments,
            }))
            .send()
            .await
            .map_err(|e| RpcError::Message(e.to_string()))?;

        if !res.status().is_success() {
            let error = match res.text().await {
                Ok(message) => RpcError::Message(message),
                _ => RpcError::Generic,
            };
            return Err(error);
        }

        #[derive(Deserialize, Debug)]
        struct ReadOnlyCallResult {
            okay: bool,
            result: String,
        }

        let response: ReadOnlyCallResult =
            res.json().await.map_err(|e| RpcError::Message(e.to_string()))?;

        if response.okay {
            // Removing the 0x prefix
            let raw_value = match response.result.strip_prefix("0x") {
                Some(raw_value) => raw_value,
                _ => panic!(),
            };
            let bytes = hex_bytes(raw_value).unwrap();
            let mut cursor = Cursor::new(&bytes);
            let value = Value::consensus_deserialize(&mut cursor).unwrap();
            Ok(value)
        } else {
            Err(RpcError::Generic)
        }
    }
}
