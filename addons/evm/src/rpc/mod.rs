use std::fmt::Debug;
use std::str::FromStr;

use alloy::consensus::TxEnvelope;
use alloy::hex;
use alloy::network::{Ethereum, EthereumWallet};
use alloy::primitives::{Address, BlockHash, Bytes, FixedBytes, Uint};
use alloy::providers::utils::Eip1559Estimation;
use alloy::providers::{ext::DebugApi, Provider, ProviderBuilder, RootProvider};
use alloy::rpc::types::{TransactionReceipt, TransactionRequest};
use alloy::transports::http::Http;
use alloy_provider::fillers::{FillProvider, JoinFill, WalletFiller};
use alloy_provider::utils::{
    EIP1559_FEE_ESTIMATION_PAST_BLOCKS, EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE,
};
use alloy_provider::Identity;
use alloy_rpc_types::trace::geth::{GethDebugTracingCallOptions, GethDebugTracingOptions};
use alloy_rpc_types::BlockNumberOrTag::Latest;
use alloy_rpc_types::{Block, BlockId, BlockNumberOrTag, BlockTransactionsKind, FeeHistory};
use txtx_addon_kit::reqwest::{Client, Url};

#[derive(Debug)]
pub enum RpcError {
    Generic,
    StatusCode(u16),
    Message(String),
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            RpcError::Message(e) => write!(f, "{}", e),
            RpcError::StatusCode(e) => write!(f, "error status code {}", e),
            RpcError::Generic => write!(f, "unknown error"),
        }
    }
}

impl Into<String> for RpcError {
    fn into(self) -> String {
        self.to_string()
    }
}

pub type WalletProvider = FillProvider<
    JoinFill<Identity, WalletFiller<EthereumWallet>>,
    RootProvider<Http<Client>>,
    Http<Client>,
    Ethereum,
>;
pub struct EvmWalletRpc {
    pub url: Url,
    pub wallet: EthereumWallet,
    pub provider: WalletProvider,
}
impl EvmWalletRpc {
    pub fn new(url: &str, wallet: EthereumWallet) -> Result<Self, String> {
        let url = Url::try_from(url).map_err(|e| format!("invalid rpc url {}: {}", url, e))?;

        let provider = ProviderBuilder::new().wallet(wallet.clone()).on_http(url.clone());
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
    pub provider: RootProvider<Http<Client>>,
}

impl EvmRpc {
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
        self.provider.get_transaction_count(address.clone()).await.map_err(|e| {
            RpcError::Message(format!("error getting transaction count: {}", e.to_string()))
        })
    }

    pub async fn get_gas_price(&self) -> Result<u128, RpcError> {
        self.provider
            .get_gas_price()
            .await
            .map_err(|e| RpcError::Message(format!("error getting gas price: {}", e.to_string())))
    }

    pub async fn estimate_gas(&self, tx: &TransactionRequest) -> Result<u128, RpcError> {
        self.provider.estimate_gas(&tx).await.map_err(|e| {
            RpcError::Message(format!("error getting gas estimate: {}", e.to_string()))
        })
    }

    pub async fn estimate_eip1559_fees(&self) -> Result<Eip1559Estimation, RpcError> {
        self.provider.estimate_eip1559_fees(None).await.map_err(|e| {
            RpcError::Message(format!("error getting EIP 1559 fees: {}", e.to_string()))
        })
    }

    pub async fn get_fee_history(&self) -> Result<FeeHistory, RpcError> {
        self.provider
            .get_fee_history(
                EIP1559_FEE_ESTIMATION_PAST_BLOCKS,
                BlockNumberOrTag::Latest,
                &[EIP1559_FEE_ESTIMATION_REWARD_PERCENTILE],
            )
            .await
            .map_err(|e| RpcError::Message(format!("error getting fee history: {}", e.to_string())))
    }

    pub async fn get_base_fee_per_gas(&self) -> Result<u128, RpcError> {
        let fee_history = self
            .get_fee_history()
            .await
            .map_err(|e| RpcError::Message(format!("error getting base fee per gas: {}", e)))?;

        fee_history
            .latest_block_base_fee()
            .ok_or(RpcError::Message(format!("error getting latest base fee")))
    }

    pub async fn get_balance(&self, address: &Address) -> Result<Uint<256, 4>, RpcError> {
        self.provider.get_balance(address.clone()).await.map_err(|e| {
            RpcError::Message(format!("error getting account balance: {}", e.to_string()))
        })
    }

    pub async fn call(&self, tx: &TransactionRequest) -> Result<String, String> {
        let result = match self.provider.call(tx).block(BlockId::pending()).await {
            Ok(res) => res,
            Err(e) => {
                let err = format!("received error result from RPC API during eth_call: {}", e);
                if let Ok(trace) = self.trace_call(&tx).await {
                    return Err(format!("{}\ncall trace: {}", err, trace));
                } else {
                    return Err(format!("{}", err));
                }
            }
        };

        Ok(hex::encode(result))
    }

    pub async fn get_code(&self, address: &Address) -> Result<Bytes, RpcError> {
        self.provider.get_code_at(address.clone()).await.map_err(|e| {
            RpcError::Message(format!(
                "error getting code at address {}: {}",
                address.to_string(),
                e.to_string()
            ))
        })
    }

    pub async fn trace_transaction(&self, tx_hash: &Vec<u8>) -> Result<String, String> {
        let result = self
            .provider
            .debug_trace_transaction(
                FixedBytes::from_slice(&tx_hash),
                GethDebugTracingOptions::default(),
            )
            .await
            .map_err(|e| {
                format!("received error result from RPC API during debug_trace_transaction: {}", e)
            })?;

        let result = serde_json::to_string(&result)
            .map_err(|e| format!("failed to serialize debug_trace_transaction response: {}", e))?;
        Ok(result)
    }

    pub async fn trace_call(&self, tx: &TransactionRequest) -> Result<String, String> {
        let result = self
            .provider
            .debug_trace_call(tx.clone(), Latest, GethDebugTracingCallOptions::default())
            .await
            .map_err(|e| format!("received error result from RPC API during trace_call: {}", e))?;

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
        self.provider.get_block_number().await.map_err(|e| {
            RpcError::Message(format!("error getting transaction receipt: {}", e.to_string()))
        })
    }

    pub async fn get_block_by_hash(&self, block_hash: &str) -> Result<Option<Block>, RpcError> {
        let block_hash = BlockHash::from_str(&block_hash).map_err(|e| {
            RpcError::Message(format!("error parsing block hash: {}", e.to_string()))
        })?;
        self.provider.get_block_by_hash(block_hash, BlockTransactionsKind::Hashes).await.map_err(
            |e| RpcError::Message(format!("error getting block by hash: {}", e.to_string())),
        )
    }

    pub async fn get_latest_block(&self) -> Result<Option<Block>, RpcError> {
        self.provider
            .get_block(BlockId::latest(), BlockTransactionsKind::Hashes)
            .await
            .map_err(|e| RpcError::Message(format!("error getting block: {}", e.to_string())))
    }
}
