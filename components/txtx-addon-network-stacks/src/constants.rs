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
pub const NONCE: &str = "nonce";
pub const SIGNED_TRANSACTION_BYTES: &str = "signed_transaction_bytes";
pub const TRANSACTION_PAYLOAD_BYTES: &str = "transaction_payload_bytes";

// Defaults keys
pub const NETWORK_ID: &str = "network_id";
pub const RPC_API_URL: &str = "rpc_api_url";

pub const DEFAULT_CONFIRMATIONS_NUMBER: u64 = 3;
pub const DEFAULT_MESSAGE: &str =
    "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks.";

// Actions items keys
pub const ACTION_ITEM_CHECK_BALANCE: &str = "check_balance";
pub const ACTION_ITEM_CHECK_NONCE: &str = "check_nonce";
pub const ACTION_ITEM_CHECK_ADDRESS: &str = "check_address";
pub const ACTION_ITEM_PROVIDE_PUBLIC_KEY: &str = "provide_public_key";
pub const ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION: &str = "provide_signed_transaction";
