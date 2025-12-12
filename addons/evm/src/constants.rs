use alloy::{contract::Interface, json_abi::JsonAbi, primitives::Address};
use txtx_addon_kit::{
    helpers::fs::get_path_from_components, hex, indexmap::IndexMap, types::types::Value,
};

use crate::codec::foundry::{BytecodeData, FoundryCompiledOutputJson};

pub const OUT_DIR: &str = env!("OUT_DIR");

pub const NAMESPACE: &str = "evm";

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/60'/0'/0/0";

// Signer attached storage keys
pub const CHECKED_PUBLIC_KEY: &str = "checked_public_key";
pub const REQUESTED_STARTUP_DATA: &str = "requested_startup_data";
pub const CHECKED_ADDRESS: &str = "checked_address";
pub const EXPECTED_ADDRESS: &str = "expected_address";
pub const CHECKED_COST_PROVISION: &str = "checked_costs";
pub const FETCHED_BALANCE: &str = "fetched_balance";
pub const FETCHED_NONCE: &str = "fetched_nonce";

// Signers
pub const PUBLIC_KEYS: &str = "public_keys";
pub const SECRET_KEY_WALLET_UNSIGNED_TRANSACTION_BYTES: &str =
    "secret_key_wallet_unsigned_transaction_bytes";
pub const WEB_WALLET_UNSIGNED_TRANSACTION_BYTES: &str = "web_wallet_unsigned_transaction_bytes";
pub const TRANSACTION_PAYLOAD_BYTES: &str = "transaction_payload_bytes";
pub const SIGNED_MESSAGE_BYTES: &str = "signed_message_bytes";
pub const MESSAGE_BYTES: &str = "message_bytes";
pub const FORMATTED_TRANSACTION: &str = "formatted_transaction";

// Defaults keys
pub const CHAIN_ID: &str = "chain_id";
pub const NETWORK_ID: &str = "network_id";
pub const RPC_API_URL: &str = "rpc_api_url";
pub const BLOCK_EXPLORER_API_KEY: &str = "block_explorer_api_key";
pub const TRANSACTION_TO: &str = "to";
pub const SIGNER: &str = "signer";
pub const TRANSACTION_AMOUNT: &str = "amount";
pub const TRANSACTION_TYPE: &str = "type";
pub const NONCE: &str = "nonce";
pub const GAS_LIMIT: &str = "gas_limit";
pub const GAS_PRICE: &str = "gas_price";
pub const MAX_FEE_PER_GAS: &str = "max_fee_per_gas";
pub const MAX_PRIORITY_FEE_PER_GAS: &str = "max_priority_fee_per_gas";
pub const CONTRACT_ADDRESS: &str = "contract_address";
pub const IMPL_CONTRACT_ADDRESS: &str = "impl_contract_address";
pub const PROXY_CONTRACT_ADDRESS: &str = "proxy_contract_address";
pub const CONTRACT_ABI: &str = "contract_abi";
pub const CONTRACT_FUNCTION_NAME: &str = "function_name";
pub const CONTRACT_FUNCTION_ARGS: &str = "function_args";
pub const CONTRACT_CONSTRUCTOR_ARGS: &str = "constructor_args";
pub const TX_HASH: &str = "tx_hash";
pub const FACTORY_ADDRESS: &str = "factory_address";
pub const FACTORY_ABI: &str = "factory_abi";
pub const FACTORY_FUNCTION_NAME: &str = "factory_function_name";
pub const FACTORY_FUNCTION_ARGS: &str = "factory_function_args";
pub const EXPECTED_CONTRACT_ADDRESS: &str = "expected_contract_address";
pub const DO_VERIFY_CONTRACT: &str = "verify";
pub const CONTRACT_VERIFICATION_OPTS: &str = "verifier";
pub const CONTRACT: &str = "contract";
pub const SALT: &str = "salt";
pub const ALREADY_DEPLOYED: &str = "already_deployed";
pub const TRANSACTION_COST: &str = "transaction_cost";
pub const ADDRESS_ABI_MAP: &str = "address_abi_map";
pub const IS_PROXIED: &str = "is_proxied";
pub const RESULT: &str = "result";
pub const ABI_ENCODED_RESULT: &str = "abi_encoded_result";
pub const LOGS: &str = "logs";
pub const RAW_LOGS: &str = "raw_logs";
pub const VERIFICATION_RESULTS: &str = "verification_results";
pub const LINKED_LIBRARIES: &str = "linked_libraries";

// Default values
pub const DEFAULT_CONFIRMATIONS_NUMBER: u64 = 1;
pub const DEFAULT_MESSAGE: &str =
    "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks.";
pub const DEFAULT_HARDHAT_ARTIFACTS_DIR: &str = "artifacts";
pub const DEFAULT_HARDHAT_SOURCE_DIR: &str = "contracts";
pub const DEFAULT_FOUNDRY_MANIFEST_PATH: &str = "foundry.toml";
pub const DEFAULT_FOUNDRY_PROFILE: &str = "default";
pub const DEFAULT_FOUNDRY_OUT_DIR: &str = "out";
pub const DEFAULT_FOUNDRY_SRC_DIR: &str = "src";
pub const DEFAULT_CREATE2_SALT: &str =
    "0x7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f";
pub const EMPTY_CREATE2_SALT: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000000";


// Default contracts
pub const DEFAULT_CREATE2_FACTORY_ADDRESS: &str = "0x4e59b44847b379578588920cA78FbF26c0B4956C";

// API Responses
pub const EXPLORER_NO_CONTRACT: &str = "Unable to locate ContractCode at";

lazy_static! {
    pub static ref DEFAULT_PROXY_CONTRACT: IndexMap<String, Value> = IndexMap::new();
    pub static ref PROXY_FACTORY_ADDRESS: Address =
        Address::from_slice(&hex::decode(&"0x13c8b8e8e671386f2e2d39e7da50284faaa3fe12"[2..]).unwrap()); // created from salt 0x0000000000000000000000000000000000000000000000000000000000007f35

    pub static ref CONTRACTS_BUILD_DIR: String = get_path_from_components(vec![OUT_DIR, "contracts"]);
    static ref PROXY_OUTPUT_PATH: String = get_path_from_components(vec![&CONTRACTS_BUILD_DIR, "out", "ERC1967Proxy.sol", "ERC1967Proxy.json"]);
    static ref PROXY_FACTORY_PATH: String = get_path_from_components(vec![&CONTRACTS_BUILD_DIR, "out", "AtomicProxyDeploymentFactory.sol", "AtomicProxyDeploymentFactory.json"]);
    pub static ref ERC1967_PROXY_COMPILED_OUTPUT: FoundryCompiledOutputJson = serde_json::from_str(&std::fs::read_to_string(PROXY_OUTPUT_PATH.to_string()).unwrap()).unwrap();
    pub static ref ERC1967_PROXY_BYTECODE: BytecodeData = ERC1967_PROXY_COMPILED_OUTPUT.bytecode.clone();
    pub static ref ERC1967_PROXY_ABI: JsonAbi = ERC1967_PROXY_COMPILED_OUTPUT.abi.clone();
    pub static ref ERC_1967_PROXY_ABI_VALUE: Value = Value::string(serde_json::to_string(&ERC1967_PROXY_COMPILED_OUTPUT.abi).unwrap());
    pub static ref ERC1967_PROXY_ABI_INTERFACE: Interface = Interface::new(ERC1967_PROXY_ABI.clone());

    pub static ref PROXY_FACTORY_COMPILED_OUTPUT: FoundryCompiledOutputJson = serde_json::from_str(&std::fs::read_to_string(PROXY_FACTORY_PATH.to_string()).unwrap()).unwrap();
    pub static ref PROXY_FACTORY_ABI: JsonAbi = PROXY_FACTORY_COMPILED_OUTPUT.abi.clone();
    pub static ref PROXY_FACTORY_ABI_VALUE: Value = Value::string(serde_json::to_string(&PROXY_FACTORY_COMPILED_OUTPUT.abi).unwrap());
    pub static ref PROXY_FACTORY_ABI_INTERFACE: Interface = Interface::new(PROXY_FACTORY_ABI.clone());

    pub static ref EMPTY_CREATE2_RAW_SALT: Vec<u8> = hex::decode(&EMPTY_CREATE2_SALT[2..]).unwrap();
}
