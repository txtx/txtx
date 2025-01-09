pub const NAMESPACE: &str = "svm";
pub const SIGNERS: &str = "signers";
pub const SIGNER: &str = "signer";

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
pub const SECRET_KEY: &str = "secret_key";
pub const MNEMONIC: &str = "mnemonic";
pub const DERIVATION_PATH: &str = "derivation_path";
pub const IS_ENCRYPTED: &str = "is_encrypted";
pub const PASSWORD: &str = "password";
pub const KEYPAIR_JSON: &str = "keypair_json";

// Defaults keys
pub const RPC_API_URL: &str = "rpc_api_url";
pub const PROGRAM_ID: &str = "program_id";
pub const ACCOUNT: &str = "account";
pub const ADDRESS: &str = "address";
pub const INSTRUCTION: &str = "instruction";
pub const PUBLIC_KEY: &str = "public_key";
pub const IS_SIGNER: &str = "is_signer";
pub const IS_WRITABLE: &str = "is_writable";
pub const TRANSACTION_BYTES: &str = "transaction_bytes";
pub const NETWORK_ID: &str = "network_id";
pub const AUTO_EXTEND: &str = "auto_extend";
pub const COMMITMENT_LEVEL: &str = "commitment_level";
pub const DO_AWAIT_CONFIRMATION: &str = "do_await_confirmation";
pub const SIGNATURE: &str = "signature";
pub const IS_DEPLOYMENT: &str = "is_deployment";
pub const AMOUNT: &str = "amount";
pub const RECIPIENT: &str = "recipient";
pub const AUTHORITY: &str = "authority";
pub const TOKEN: &str = "token";
pub const FUND_RECIPIENT: &str = "fund_recipient";
pub const AUTHORITY_ADDRESS: &str = "authority_address";
pub const RECIPIENT_ADDRESS: &str = "recipient_address";
pub const RECIPIENT_TOKEN_ADDRESS: &str = "recipient_token_address";
pub const SOURCE_TOKEN_ADDRESS: &str = "source_token_address";
pub const TOKEN_MINT_ADDRESS: &str = "token_mint_address";
pub const IS_FUNDING_RECIPIENT: &str = "is_funding_recipient";

// Actions items keys
pub const ACTION_ITEM_CHECK_BALANCE: &str = "check_balance";
pub const ACTION_ITEM_CHECK_ADDRESS: &str = "check_address";
pub const ACTION_ITEM_PROVIDE_PUBLIC_KEY: &str = "provide_public_key";
pub const ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION: &str = "provide_signed_transaction";
