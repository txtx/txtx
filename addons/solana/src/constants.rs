pub const NAMESPACE: &str = "solana";
pub const SIGNERS: &str = "signers";

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/501'/0'/0/0";
pub const DEFAULT_ANCHOR_TARGET_PATH: &str = "target";

// Signer attached storage keys
pub const CHECKED_PUBLIC_KEY: &str = "checked_public_key";
pub const REQUESTED_STARTUP_DATA: &str = "requested_startup_data";
pub const CHECKED_ADDRESS: &str = "checked_address";
pub const EXPECTED_ADDRESS: &str = "expected_address";
pub const PROGRAM_DEPLOYMENT_KEYPAIR: &str = "program_deployment_keypair";

// Signers
pub const IS_SIGNABLE: &str = "is_signable";
pub const FORMATTED_TRANSACTION: &str = "formatted_transaction";

// Defaults keys
pub const RPC_API_URL: &str = "rpc_api_url";
pub const PROGRAM_ID: &str = "program_id";
pub const TRANSACTION_BYTES: &str = "transaction_bytes";
pub const NETWORK_ID: &str = "network_id";

// Actions items keys
pub const ACTION_ITEM_CHECK_BALANCE: &str = "check_balance";
pub const ACTION_ITEM_CHECK_ADDRESS: &str = "check_address";
pub const ACTION_ITEM_PROVIDE_PUBLIC_KEY: &str = "provide_public_key";
pub const ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION: &str = "provide_signed_transaction";
