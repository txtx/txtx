pub const NAMESPACE: &str = "stacks";
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
pub const UNSIGNED_TRANSACTION_BYTES: &str = "unsigned_transaction_bytes";
pub const TRANSACTION_PAYLOAD_BYTES: &str = "transaction_payload_bytes";
pub const TRANSACTION_POST_CONDITIONS_BYTES: &str = "transaction_post_conditions_bytes";
pub const TRANSACTION_POST_CONDITION_MODE_BYTES: &str = "transaction_post_condition_mode_bytes";
pub const MESSAGE_BYTES: &str = "message_bytes";
pub const REQUIRED_SIGNATURE_COUNT: &str = "required_signer_count";
pub const SIGNER: &str = "signer";
pub const IS_SIGNABLE: &str = "is_signable";
pub const FORMATTED_TRANSACTION: &str = "formatted_transaction";

// Defaults keys
pub const NETWORK_ID: &str = "network_id";
pub const RPC_API_URL: &str = "rpc_api_url";
pub const RPC_API_AUTH_TOKEN: &str = "rpc_api_auth_token";

pub const DEFAULT_DEVNET_BACKOFF: u64 = 500;
pub const DEFAULT_MAINNET_BACKOFF: u64 = 15000;
pub const DEFAULT_CONFIRMATIONS_NUMBER: u64 = 1;
pub const DEFAULT_MESSAGE: &str =
    "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks.";
pub const DEFAULT_CLARINET_MANIFEST_PATH: &str = "Clarinet.toml";

// Actions items keys
pub const ACTION_OPEN_MODAL: &str = "open_modal";
