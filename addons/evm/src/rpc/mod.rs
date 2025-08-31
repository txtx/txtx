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

// Import error-stack types
use error_stack::{Report, ResultExt};
use crate::errors::{
    EvmError, EvmResult, RpcError as EvmRpcError, RpcContext, ConfigError, TransactionError
};

// Keep old RpcError for gradual migration
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

// Helper to convert old RpcError to new error-stack
impl From<RpcError> for Report<EvmError> {
    fn from(err: RpcError) -> Self {
        match err {
            RpcError::Message(msg) => Report::new(EvmError::Rpc(EvmRpcError::NodeError(msg))),
            RpcError::MessageWithCode(msg, code) => {
                Report::new(EvmError::Rpc(EvmRpcError::NodeError(format!("error (code {}): {}", code, msg))))
            }
            RpcError::StatusCode(code) => {
                Report::new(EvmError::Rpc(EvmRpcError::NodeError(format!("error status code {}", code))))
            }
            RpcError::Generic => Report::new(EvmError::Rpc(EvmRpcError::NodeError("unknown error".to_string()))),
        }
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
    pub fn new(url: &str, wallet: EthereumWallet) -> EvmResult<Self> {
        let url = Url::try_from(url)
            .map_err(|e| Report::new(EvmError::Config(ConfigError::InvalidValue {
                field: "rpc_url".to_string(),
                value: format!("{}: {}", url.to_string(), e),
            })))?;

        let provider = ProviderBuilder::new().on_http(url.clone());
        Ok(Self { url, wallet, provider })
    }

    pub async fn sign_and_send_tx(&self, tx_envelope: TxEnvelope) -> EvmResult<[u8; 32]> {
        let pending_tx = self.provider
            .send_tx_envelope(tx_envelope.clone())
            .await
            .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
            .attach(RpcContext {
                endpoint: self.url.to_string(),
                method: "eth_sendRawTransaction".to_string(),
                params: Some(format!("{:?}", tx_envelope)),
            })
            .attach_printable("Failed to sign and send transaction")?;
        
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
    pub fn new(url: &str) -> EvmResult<Self> {
        let url = Url::try_from(url)
            .map_err(|e| Report::new(EvmError::Config(ConfigError::InvalidValue {
                field: "rpc_url".to_string(),
                value: format!("{}: {}", url.to_string(), e),
            })))?;

        let provider = ProviderBuilder::new().on_http(url.clone());
        Ok(Self { url, provider })
    }

    pub async fn get_chain_id(&self) -> EvmResult<u64> {
        EvmRpc::retry_async(|| async {
            self.provider.get_chain_id()
                .await
                .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
                .attach(RpcContext {
                    endpoint: self.url.to_string(),
                    method: "eth_chainId".to_string(),
                    params: None,
                })
        })
        .await
    }

    pub async fn get_nonce(&self, address: &Address) -> EvmResult<u64> {
        EvmRpc::retry_async(|| async {
            self.provider.get_transaction_count(address.clone())
                .await
                .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
                .attach(RpcContext {
                    endpoint: self.url.to_string(),
                    method: "eth_getTransactionCount".to_string(),
                    params: Some(format!("[\"{:?}\", \"pending\"]", address)),
                })
                .attach_printable(format!("Getting nonce for address {}", address))
        })
        .await
    }

    pub async fn get_gas_price(&self) -> EvmResult<u128> {
        EvmRpc::retry_async(|| async {
            self.provider.get_gas_price()
                .await
                .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
                .attach(RpcContext {
                    endpoint: self.url.to_string(),
                    method: "eth_gasPrice".to_string(),
                    params: None,
                })
        })
        .await
    }

    pub async fn estimate_gas(&self, tx: &TransactionRequest) -> EvmResult<u64> {
        let tx_clone = tx.clone();
        let provider = self.provider.clone();
        let url = self.url.clone();
        
        EvmRpc::retry_async(move || {
            let tx = tx_clone.clone();
            let provider = provider.clone();
            let url = url.clone();
            
            async move {
                match provider.estimate_gas(tx.clone()).await {
                    Ok(gas) => Ok(gas),
                    Err(e) => {
                        let error_str = e.to_string();
                        // Check for insufficient funds error
                        if error_str.contains("gas required exceeds allowance") || 
                           error_str.contains("insufficient funds") {
                            // When gas estimation fails due to insufficient funds, we calculate
                            // the required amount to help users understand how much ETH they need.
                            //
                            // Calculation formula:
                            // required = (gas_price * estimated_gas_units) + transaction_value
                            //
                            // Where:
                            // - gas_price: Current network gas price fetched via eth_gasPrice
                            // - estimated_gas_units: We use 3M gas as a reasonable estimate for
                            //   contract deployments since we can't get the actual estimate
                            //   (the estimation itself is failing due to insufficient funds)
                            // - transaction_value: Any ETH being sent with the transaction
                            //
                            // This gives users a concrete amount to fund their account with,
                            // rather than just saying "insufficient funds" with no context.
                            
                            let mut available = 0u128;
                            let mut required = 0u128;
                            
                            if let Some(from) = tx.from {
                                // Get actual balance from the account
                                if let Ok(balance) = provider.get_balance(from).await {
                                    available = balance.to::<u128>();
                                }
                                
                                // Calculate estimated required amount
                                if let Ok(gas_price) = provider.get_gas_price().await {
                                    // Use 3M gas as a reasonable estimate for contract deployment
                                    // This is conservative but ensures users fund enough for most cases
                                    let estimated_gas = 3_000_000u128;
                                    required = gas_price * estimated_gas;
                                    
                                    // Add transaction value if any (e.g., payable constructors)
                                    if let Some(value) = tx.value {
                                        required = required.saturating_add(value.to::<u128>());
                                    }
                                }
                            }
                            
                            Err(Report::new(EvmError::Transaction(TransactionError::InsufficientFunds {
                                required,
                                available,
                            }))
                            .attach_printable(format!("Account {} has insufficient funds", 
                                tx.from.map(|a| format!("{:?}", a)).unwrap_or_else(|| "unknown".to_string())))
                            .attach_printable(format!("Available: {} wei, Estimated required: {} wei", available, required))
                            .attach_printable("Suggested fix: Fund the account with ETH before deploying contracts"))
                        } else {
                            Err(Report::new(EvmError::Rpc(EvmRpcError::NodeError(error_str))))
                        }
                    }
                }
                .attach(RpcContext {
                    endpoint: url.to_string(),
                    method: "eth_estimateGas".to_string(),
                    params: Some(format!("{:?}", tx)),
                })
                .attach_printable("Estimating gas for transaction")
            }
        })
        .await
    }

    pub async fn estimate_eip1559_fees(&self) -> EvmResult<Eip1559Estimation> {
        EvmRpc::retry_async(|| async {
            self.provider.estimate_eip1559_fees()
                .await
                .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
                .attach(RpcContext {
                    endpoint: self.url.to_string(),
                    method: "eth_feeHistory".to_string(),
                    params: None,
                })
        })
        .await
    }

    pub async fn get_fee_history(&self) -> EvmResult<FeeHistory> {
        EvmRpc::retry_async(|| async {
            self.provider
                .get_fee_history(
                    EIP1559_FEE_ESTIMATION_PAST_BLOCKS.into(),
                    BlockNumberOrTag::Latest,
                    &[EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE],
                )
                .await
                .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
                .attach(RpcContext {
                    endpoint: self.url.to_string(),
                    method: "eth_feeHistory".to_string(),
                    params: Some(format!("[{}, \"latest\", [{}]]", 
                        EIP1559_FEE_ESTIMATION_PAST_BLOCKS,
                        EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE)),
                })
        })
        .await
    }

    pub async fn get_base_fee_per_gas(&self) -> EvmResult<u128> {
        let fee_history = self.get_fee_history()
            .await
            .attach_printable("Fetching fee history to determine base fee")?;
        
        fee_history.latest_block_base_fee()
            .ok_or_else(|| Report::new(EvmError::Rpc(EvmRpcError::InvalidResponse(
                "No base fee in fee history".to_string()
            ))))
            .attach_printable("Extracting base fee from fee history")
    }

    pub async fn get_balance(&self, address: &Address) -> EvmResult<Uint<256, 4>> {
        EvmRpc::retry_async(|| async {
            self.provider.get_balance(address.clone())
                .await
                .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
                .attach(RpcContext {
                    endpoint: self.url.to_string(),
                    method: "eth_getBalance".to_string(),
                    params: Some(format!("[\"{:?}\", \"latest\"]", address)),
                })
                .attach_printable(format!("Getting balance for address {}", address))
        })
        .await
    }

    pub async fn call(
        &self,
        tx: &TransactionRequest,
        trace: bool,
    ) -> EvmResult<String> {
        let result = if trace {
            let opts = GethDebugTracingCallOptions {
                tracing_options: GethDebugTracingOptions {
                    tracer: Some(alloy::rpc::types::trace::geth::GethDebugTracerType::BuiltInTracer(
                        alloy::rpc::types::trace::geth::GethDebugBuiltInTracerType::CallTracer,
                    )),
                    ..GethDebugTracingOptions::default()
                },
                ..GethDebugTracingCallOptions::default()
            };

            match self.provider.debug_trace_call(tx.clone(), BlockId::from(BlockNumberOrTag::Latest), opts).await {
                Ok(trace) => {
                    let traces = match trace {
                        GethTrace::Default(frame) => serde_json::to_string(&frame)
                            .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::InvalidResponse(e.to_string()))))?,
                        GethTrace::CallTracer(frame) => serde_json::to_string(&frame)
                            .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::InvalidResponse(e.to_string()))))?,
                        _ => String::new(),
                    };
                    traces
                }
                Err(e) => e.to_string(),
            }
        } else {
            match self.provider.call(tx.clone()).await {
                Ok(res) => format!("0x{}", hex::encode(res.to_vec())),
                Err(e) => e.to_string(),
            }
        };

        Ok(result)
    }

    pub async fn get_code(&self, address: &Address) -> EvmResult<Bytes> {
        EvmRpc::retry_async(|| async {
            self.provider
                .get_code_at(address.clone())
                .await
                .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
                .attach(RpcContext {
                    endpoint: self.url.to_string(),
                    method: "eth_getCode".to_string(),
                    params: Some(format!("[\"{:?}\", \"latest\"]", address)),
                })
                .attach_printable(format!("Getting code at address {}", address))
        })
        .await
    }

    pub async fn get_transaction_return_value(&self, tx_hash: &Vec<u8>) -> EvmResult<String> {
        let hash_str = format!("0x{}", hex::encode(tx_hash));
        let hash = FixedBytes::<32>::from_str(&hash_str)
            .map_err(|e| Report::new(EvmError::Config(ConfigError::InvalidValue {
                field: "tx_hash".to_string(),
                value: format!("{}: {}", hash_str.clone(), e),
            })))?;

        let receipt = self.get_receipt(&hash_str).await?;
        
        let trace_opts = GethDebugTracingOptions {
            tracer: Some(alloy::rpc::types::trace::geth::GethDebugTracerType::BuiltInTracer(
                alloy::rpc::types::trace::geth::GethDebugBuiltInTracerType::CallTracer,
            )),
            ..GethDebugTracingOptions::default()
        };

        let trace = self.provider
            .debug_trace_transaction(hash, trace_opts)
            .await
            .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
            .attach(RpcContext {
                endpoint: self.url.to_string(),
                method: "debug_traceTransaction".to_string(),
                params: Some(format!("[\"{}\", {{\"tracer\": \"callTracer\"}}]", hash_str)),
            })?;

        match trace {
            GethTrace::Default(frame) => Ok(serde_json::to_string(&frame)
                .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::InvalidResponse(e.to_string()))))?),
            GethTrace::CallTracer(frame) => Ok(serde_json::to_string(&frame)
                .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::InvalidResponse(e.to_string()))))?),
            _ => Ok(String::new()),
        }
    }

    pub async fn trace_call(&self, tx: &TransactionRequest) -> EvmResult<String> {
        let opts = GethDebugTracingCallOptions {
            tracing_options: GethDebugTracingOptions {
                tracer: Some(alloy::rpc::types::trace::geth::GethDebugTracerType::BuiltInTracer(
                    alloy::rpc::types::trace::geth::GethDebugBuiltInTracerType::CallTracer,
                )),
                ..GethDebugTracingOptions::default()
            },
            ..GethDebugTracingCallOptions::default()
        };

        let trace = self.provider
            .debug_trace_call(tx.clone(), BlockId::from(BlockNumberOrTag::Latest), opts)
            .await
            .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
            .attach(RpcContext {
                endpoint: self.url.to_string(),
                method: "debug_traceCall".to_string(),
                params: Some(format!("{:?}", tx)),
            })?;

        match trace {
            GethTrace::Default(frame) => Ok(serde_json::to_string(&frame)
                .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::InvalidResponse(e.to_string()))))?),
            GethTrace::CallTracer(frame) => Ok(serde_json::to_string(&frame)
                .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::InvalidResponse(e.to_string()))))?),
            _ => Ok(String::new()),
        }
    }

    pub async fn get_receipt(&self, tx_hash: &str) -> EvmResult<TransactionReceipt> {
        let hash = FixedBytes::<32>::from_str(tx_hash)
            .map_err(|e| Report::new(EvmError::Config(ConfigError::InvalidValue {
                field: "tx_hash".to_string(),
                value: format!("{}: {}", tx_hash.to_string(), e),
            })))?;

        self.provider.get_transaction_receipt(hash)
            .await
            .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
            .attach(RpcContext {
                endpoint: self.url.to_string(),
                method: "eth_getTransactionReceipt".to_string(),
                params: Some(format!("[\"{}\"]", tx_hash)),
            })?
            .ok_or_else(|| Report::new(EvmError::Rpc(EvmRpcError::InvalidResponse(
                format!("No receipt found for transaction {}", tx_hash)
            ))))
    }

    pub async fn get_block_number(&self) -> EvmResult<u64> {
        EvmRpc::retry_async(|| async {
            self.provider.get_block_number()
                .await
                .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
                .attach(RpcContext {
                    endpoint: self.url.to_string(),
                    method: "eth_blockNumber".to_string(),
                    params: None,
                })
        })
        .await
    }

    pub async fn get_block_by_hash(&self, block_hash: &str) -> EvmResult<Option<Block>> {
        let hash = BlockHash::from_str(block_hash)
            .map_err(|e| Report::new(EvmError::Config(ConfigError::InvalidValue {
                field: "block_hash".to_string(),
                value: format!("{}: {}", block_hash.to_string(), e),
            })))?;

        self.provider.get_block_by_hash(hash)
            .await
            .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
            .attach(RpcContext {
                endpoint: self.url.to_string(),
                method: "eth_getBlockByHash".to_string(),
                params: Some(format!("[\"{}\", true]", block_hash)),
            })
    }

    pub async fn get_latest_block(&self) -> EvmResult<Option<Block>> {
        self.provider
            .get_block(BlockId::from(BlockNumberOrTag::Latest))
            .await
            .map_err(|e| Report::new(EvmError::Rpc(EvmRpcError::NodeError(e.to_string()))))
            .attach(RpcContext {
                endpoint: self.url.to_string(),
                method: "eth_getBlockByNumber".to_string(),
                params: Some("[\"latest\", true]".to_string()),
            })
    }

    async fn retry_async<T, E, Fut, F>(f: F) -> Result<T, E>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        let mut retries = 0;
        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if retries >= 3 {
                        return Err(e);
                    }
                    retries += 1;
                    sleep(Duration::from_millis(100 * retries));
                }
            }
        }
    }

    pub fn to_string(&self) -> String {
        self.url.to_string()
    }
    
    // Compatibility constructor for gradual migration
    pub fn new_compat(url: &str) -> Result<Self, String> {
        Self::new(url).map_err(|e| e.to_string())
    }

    // Keep old interface for compatibility during migration
    pub async fn get_nonce_old(&self, address: &Address) -> Result<u64, RpcError> {
        self.get_nonce(address)
            .await
            .map_err(|e| RpcError::Message(e.to_string()))
    }

    pub async fn get_gas_price_old(&self) -> Result<u128, RpcError> {
        self.get_gas_price()
            .await
            .map_err(|e| RpcError::Message(e.to_string()))
    }

    pub async fn estimate_gas_old(&self, tx: &TransactionRequest) -> Result<u64, RpcError> {
        self.estimate_gas(tx)
            .await
            .map_err(|e| RpcError::Message(e.to_string()))
    }
}