use alloy::hex;
use alloy::primitives::{Address, FixedBytes, Uint};
use alloy::providers::utils::Eip1559Estimation;
use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::rpc::types::{TransactionReceipt, TransactionRequest};
use alloy::transports::http::Http;

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

#[derive(Clone, Debug)]
pub struct EVMRpc {
    pub url: Url,
    pub provider: RootProvider<Http<Client>>,
}

impl EVMRpc {
    pub fn new(url: &str) -> Result<Self, String> {
        let url = Url::try_from(url).map_err(|e| format!("invalid rpc url {}: {}", url, e))?;
        let provider = ProviderBuilder::new().on_http(url.clone());
        Ok(Self { url, provider })
    }

    pub async fn get_nonce(&self, address: &Address) -> Result<u64, RpcError> {
        self.provider
            .get_transaction_count(address.clone())
            .await
            .map_err(|e| {
                RpcError::Message(format!(
                    "error getting transaction count: {}",
                    e.to_string()
                ))
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
        self.provider
            .estimate_eip1559_fees(None)
            .await
            .map_err(|e| {
                RpcError::Message(format!("error getting EIP 1559 fees: {}", e.to_string()))
            })
    }

    pub async fn get_balance(&self, address: &Address) -> Result<Uint<256, 4>, RpcError> {
        self.provider
            .get_balance(address.clone())
            .await
            .map_err(|e| {
                RpcError::Message(format!("error getting account balance: {}", e.to_string()))
            })
    }

    pub async fn call(&self, tx: &TransactionRequest) -> Result<String, String> {
        let result =
            self.provider.call(tx).await.map_err(|e| {
                format!("received error result from RPC API during eth_call: {}", e)
            })?;

        Ok(hex::encode(result))
    }

    pub async fn get_receipt(
        &self,
        tx_hash: &Vec<u8>,
    ) -> Result<Option<TransactionReceipt>, RpcError> {
        self.provider
            .get_transaction_receipt(FixedBytes::from_slice(&tx_hash))
            .await
            .map_err(|e| {
                RpcError::Message(format!(
                    "error getting transaction receipt: {}",
                    e.to_string()
                ))
            })
    }

    pub async fn get_block_number(&self) -> Result<u64, RpcError> {
        self.provider.get_block_number().await.map_err(|e| {
            RpcError::Message(format!(
                "error getting transaction receipt: {}",
                e.to_string()
            ))
        })
    }
}
