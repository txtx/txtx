// Wallet attached storage keys
pub const CHECKED_PUBLIC_KEY: &str = "checked_public_key";
pub const REQUESTED_STARTUP_DATA: &str = "requested_startup_data";
pub const CHECKED_ADDRESS: &str = "checked_address";
pub const EXPECTED_ADDRESS: &str = "expected_address";
pub const CHECKED_COST_PROVISION: &str = "checked_costs";
pub const FETCHED_BALANCE: &str = "fetched_balance";
pub const FETCHED_NONCE: &str = "fetched_nonce";

// Wallets
pub const PUBLIC_KEYS: &str = "public_keys";
pub const SIGNED_TRANSACTION_BYTES: &str = "signed_transaction_bytes";
pub const UNSIGNED_TRANSACTION_BYTES: &str = "unsigned_transaction_bytes";
pub const TRANSACTION_PAYLOAD_BYTES: &str = "transaction_payload_bytes";
pub const SIGNED_MESSAGE_BYTES: &str = "signed_message_bytes";
pub const MESSAGE_BYTES: &str = "message_bytes";

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
pub const CONTRACT_ABI: &str = "contract_abi";
pub const CONTRACT_FUNCTION_NAME: &str = "function_name";
pub const CONTRACT_FUNCTION_ARGS: &str = "function_args";
pub const CONTRACT_CONSTRUCTOR_ARGS: &str = "constructor_args";
pub const ARTIFACTS: &str = "artifacts";
pub const TX_HASH: &str = "tx_hash";
pub const CREATE2_FACTORY_ADDRESS: &str = "create2_factory_address";
pub const CREATE2_FACTORY_ABI: &str = "create2_factory_abi";
pub const CREATE2_FUNCTION_NAME: &str = "create2_factory_function_name";
pub const CREATE2_FUNCTION_ARGS: &str = "create2_factory_function_args";
pub const EXPECTED_CONTRACT_ADDRESS: &str = "expected_contract_address";
pub const DO_VERIFY_CONTRACT: &str = "verify";
pub const CONTRACT: &str = "contract";
pub const SALT: &str = "salt";
pub const ALREADY_DEPLOYED: &str = "already_deployed";

// Default values
pub const DEFAULT_CONFIRMATIONS_NUMBER: u64 = 1;
pub const DEFAULT_MESSAGE: &str =
    "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks.";

// Actions items keys
pub const ACTION_ITEM_CHECK_BALANCE: &str = "check_balance";
pub const ACTION_ITEM_CHECK_ADDRESS: &str = "check_address";
pub const ACTION_ITEM_CHECK_NONCE: &str = "check_nonce";
pub const ACTION_ITEM_CHECK_FEE: &str = "check_fee";
pub const ACTION_ITEM_PROVIDE_PUBLIC_KEY: &str = "provide_public_key";
pub const ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION: &str = "provide_signed_transaction";
pub const ACTION_OPEN_MODAL: &str = "open_modal";

// Default contracts
pub const DEFAULT_CREATE2_FACTORY_ADDRESS: &str = "0x4e59b44847b379578588920cA78FbF26c0B4956C";

// API Responses
pub const EXPLORER_NO_CONTRACT: &str = "Unable to locate ContractCode at";
