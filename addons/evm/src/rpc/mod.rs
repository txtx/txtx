use std::fmt::Debug;
use std::future::Future;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

use alloy::consensus::TxEnvelope;
use alloy::hex;
use alloy::network::EthereumWallet;
use alloy::primitives::{Address, BlockHash, Bytes, FixedBytes, Uint};
use alloy::providers::utils::Eip1559Estimation;
use alloy::providers::{ext::DebugApi, Provider, ProviderBuilder, RootProvider};
use alloy::rpc::types::{TransactionReceipt, TransactionRequest};
use alloy_provider::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
};
use alloy_provider::utils::{
    EIP1559_FEE_ESTIMATION_PAST_BLOCKS, EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE,
};
use alloy_provider::Identity;
use alloy_rpc_types::trace::geth::{
    GethDebugTracingCallOptions, GethDebugTracingOptions, GethTrace,
};
use alloy_rpc_types::{Block, BlockId, BlockNumberOrTag, FeeHistory};
use txtx_addon_kit::reqwest::Url;

#[derive(Debug)]
pub enum RpcError {
    Generic,
    StatusCode(u16),
    Message(String),
    MessageWithCode(String, i64),
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            RpcError::Message(e) => write!(f, "{}", e),
            RpcError::StatusCode(e) => write!(f, "error status code {}", e),
            RpcError::Generic => write!(f, "unknown error"),
            RpcError::MessageWithCode(m, s) => write!(f, "error (code {}): {}", s, m),
        }
    }
}

impl Into<String> for RpcError {
    fn into(self) -> String {
        self.to_string()
    }
}

pub type WalletProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<
            EthereumWallet,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
    >,
    RootProvider,
>;
pub struct EvmWalletRpc {
    pub url: Url,
    pub wallet: EthereumWallet,
    pub provider: FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
}
impl EvmWalletRpc {
    pub fn new(url: &str, wallet: EthereumWallet) -> Result<Self, String> {
        let url = Url::try_from(url).map_err(|e| format!("invalid rpc url {}: {}", url, e))?;

        let provider = ProviderBuilder::new().on_http(url.clone());
        Ok(Self { url, wallet, provider })
    }
    pub async fn sign_and_send_tx(&self, tx_envelope: TxEnvelope) -> Result<[u8; 32], RpcError> {
        let pending_tx =
            self.provider.send_tx_envelope(tx_envelope).await.map_err(|e| {
                RpcError::Message(format!("failed to sign and send transaction: {e}"))
            })?;
        let tx_hash = pending_tx.tx_hash().0;
        Ok(tx_hash)
    }
}

#[derive(Clone, Debug)]
pub struct EvmRpc {
    pub url: Url,
    pub provider: FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
}

impl EvmRpc {
    async fn retry_async<F, Fut, T>(mut operation: F) -> Result<T, RpcError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, RpcError>>,
    {
        let mut attempts = 0;
        let max_retries = 5;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(_) if attempts < max_retries => {
                    attempts += 1;
                    sleep(Duration::from_secs(2));
                }
                Err(err) => return Err(err),
            }
        }
    }
    pub fn new(url: &str) -> Result<Self, String> {
        let url = Url::try_from(url).map_err(|e| format!("invalid rpc url {}: {}", url, e))?;
        let provider = ProviderBuilder::new().on_http(url.clone());
        Ok(Self { url, provider })
    }

    pub async fn get_chain_id(&self) -> Result<u64, RpcError> {
        self.provider
            .get_chain_id()
            .await
            .map_err(|e| RpcError::Message(format!("error getting chain id: {}", e.to_string())))
    }

    pub async fn get_nonce(&self, address: &Address) -> Result<u64, RpcError> {
        EvmRpc::retry_async(|| async {
            self.provider.get_transaction_count(address.clone()).await.map_err(|e| {
                RpcError::Message(format!("error getting transaction count: {}", e.to_string()))
            })
        })
        .await
    }

    pub async fn get_gas_price(&self) -> Result<u128, RpcError> {
        EvmRpc::retry_async(|| async {
            self.provider.get_gas_price().await.map_err(|e| {
                RpcError::Message(format!("error getting gas price: {}", e.to_string()))
            })
        })
        .await
    }

    pub async fn estimate_gas(&self, tx: &TransactionRequest) -> Result<u64, RpcError> {
        EvmRpc::retry_async(|| async {
            self.provider.estimate_gas(tx.clone()).await.map_err(|e| {
                RpcError::Message(format!("error getting gas estimate: {}", e.to_string()))
            })
        })
        .await
    }

    pub async fn estimate_eip1559_fees(&self) -> Result<Eip1559Estimation, RpcError> {
        EvmRpc::retry_async(|| async {
            self.provider.estimate_eip1559_fees().await.map_err(|e| {
                RpcError::Message(format!("error getting EIP 1559 fees: {}", e.to_string()))
            })
        })
        .await
    }

    pub async fn get_fee_history(&self) -> Result<FeeHistory, RpcError> {
        EvmRpc::retry_async(|| async {
            self.provider
                .get_fee_history(
                    EIP1559_FEE_ESTIMATION_PAST_BLOCKS,
                    BlockNumberOrTag::Latest,
                    &[EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE],
                )
                .await
                .map_err(|e| {
                    RpcError::Message(format!("error getting fee history: {}", e.to_string()))
                })
        })
        .await
    }

    pub async fn get_base_fee_per_gas(&self) -> Result<u128, RpcError> {
        let fee_history = EvmRpc::retry_async(|| async {
            self.get_fee_history()
                .await
                .map_err(|e| RpcError::Message(format!("error getting base fee per gas: {}", e)))
        })
        .await?;

        fee_history
            .latest_block_base_fee()
            .ok_or(RpcError::Message(format!("error getting latest base fee")))
    }

    pub async fn get_balance(&self, address: &Address) -> Result<Uint<256, 4>, RpcError> {
        EvmRpc::retry_async(|| async {
            self.provider.get_balance(address.clone()).await.map_err(|e| {
                RpcError::Message(format!("error getting account balance: {}", e.to_string()))
            })
        })
        .await
    }

    pub async fn call(
        &self,
        tx: &TransactionRequest,
        retry: bool,
    ) -> Result<String, CallFailureResult> {
        let call_res = if retry {
            EvmRpc::retry_async(|| async {
                self.provider.call(tx.clone()).block(BlockId::pending()).await.map_err(|e| {
                    if let Some(e) = e.as_error_resp() {
                        RpcError::MessageWithCode(e.message.to_string(), e.code)
                    } else {
                        RpcError::Message(e.to_string())
                    }
                })
            })
            .await
        } else {
            self.provider.call(tx.clone()).block(BlockId::latest()).await.map_err(|e| {
                if let Some(e) = e.as_error_resp() {
                    RpcError::MessageWithCode(e.message.to_string(), e.code)
                } else {
                    RpcError::Message(e.to_string())
                }
            })
        };

        let result = match call_res {
            Ok(res) => res,
            Err(e) => match e {
                RpcError::MessageWithCode(message, code) => {
                    // code 3 for revert
                    if code == 3 {
                        let trace = self.trace_call(&tx).await.ok();
                        return Err(CallFailureResult::RevertData { reason: message, trace });
                    } else {
                        return Err(CallFailureResult::Error(message));
                    }
                }
                e => {
                    return Err(CallFailureResult::Error(e.to_string()));
                }
            },
        };

        Ok(hex::encode(result))
    }

    pub async fn get_code(&self, address: &Address) -> Result<Bytes, RpcError> {
        EvmRpc::retry_async(|| async {
            self.provider.get_code_at(address.clone()).await.map_err(|e| {
                RpcError::Message(format!(
                    "error getting code at address {}: {}",
                    address.to_string(),
                    e.to_string()
                ))
            })
        })
        .await
    }

    pub async fn get_transaction_return_value(&self, tx_hash: &Vec<u8>) -> Result<String, String> {
        let result = EvmRpc::retry_async(|| async {
            self.provider
                .debug_trace_transaction(
                    FixedBytes::from_slice(&tx_hash),
                    GethDebugTracingOptions::default(),
                )
                .await
                .map_err(|e| {
                    RpcError::Message(format!(
                        "received error result from RPC API during debug_trace_transaction: {}",
                        e
                    ))
                })
        })
        .await
        .map_err(|e| e.to_string())?;

        match result {
            GethTrace::Default(default_frame) => {
                Ok(hex::encode(&default_frame.return_value.to_vec()))
            }
            _ => {
                let result = serde_json::to_string(&result)
                    .map_err(|e| format!("failed to serialize trace response: {}", e))?;
                return Ok(result);
            }
        }
    }

    pub async fn trace_call(&self, tx: &TransactionRequest) -> Result<String, String> {
        let result = EvmRpc::retry_async(|| async {
            self.provider
                .debug_trace_call(
                    tx.clone(),
                    BlockId::latest(),
                    GethDebugTracingCallOptions::default(),
                )
                .await
                .map_err(|e| {
                    RpcError::Message(format!(
                        "received error result from RPC API during trace_call: {}",
                        e
                    ))
                })
        })
        .await
        .map_err(|e| e.to_string())?;

        let result = serde_json::to_string(&result)
            .map_err(|e| format!("failed to serialize trace response: {}", e))?;
        Ok(result)
    }

    pub async fn get_receipt(
        &self,
        tx_hash: &Vec<u8>,
    ) -> Result<Option<TransactionReceipt>, RpcError> {
        self.provider.get_transaction_receipt(FixedBytes::from_slice(&tx_hash)).await.map_err(|e| {
            RpcError::Message(format!("error getting transaction receipt: {}", e.to_string()))
        })
    }

    pub async fn get_block_number(&self) -> Result<u64, RpcError> {
        EvmRpc::retry_async(|| async {
            self.provider.get_block_number().await.map_err(|e| {
                RpcError::Message(format!("error getting block number: {}", e.to_string()))
            })
        })
        .await
    }

    pub async fn get_block_by_hash(&self, block_hash: &str) -> Result<Option<Block>, RpcError> {
        let block_hash = BlockHash::from_str(&block_hash).map_err(|e| {
            RpcError::Message(format!("error parsing block hash: {}", e.to_string()))
        })?;
        EvmRpc::retry_async(|| async {
            self.provider.get_block_by_hash(block_hash).await.map_err(|e| {
                RpcError::Message(format!("error getting block by hash: {}", e.to_string()))
            })
        })
        .await
    }

    pub async fn get_latest_block(&self) -> Result<Option<Block>, RpcError> {
        EvmRpc::retry_async(|| async {
            self.provider
                .get_block(BlockId::latest())
                .await
                .map_err(|e| RpcError::Message(format!("error getting block: {}", e.to_string())))
        })
        .await
    }
}

#[derive(Debug)]
pub enum CallFailureResult {
    RevertData { reason: String, trace: Option<String> },
    Error(String),
}

impl CallFailureResult {
    pub fn to_string(&self) -> String {
        match self {
            CallFailureResult::RevertData { reason, .. } => {
                format!("{}", reason)
            }
            CallFailureResult::Error(e) => e.clone(),
        }
    }

    pub fn to_string_with_trace(&self) -> String {
        match self {
            CallFailureResult::RevertData { reason, trace } => {
                if let Some(trace) = trace {
                    format!("{}\ntrace: {}", reason, trace)
                } else {
                    format!(" {}", reason)
                }
            }
            CallFailureResult::Error(e) => e.clone(),
        }
    }
}
