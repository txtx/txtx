pub const NAMESPACE: &str = "svm";
pub const SIGNERS: &str = "signers";
pub const SIGNER: &str = "signer";
pub const PAYER: &str = "payer";
pub const AUTHORITY: &str = "authority";
pub const OWNER: &str = "owner";

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/501'/0'/0/0";
pub const DEFAULT_ANCHOR_TARGET_PATH: &str = "target";
pub const DEFAULT_NATIVE_TARGET_PATH: &str = "target";
pub const DEFAULT_SHANK_IDL_PATH: &str = "idl";

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
pub const PROGRAM_IDL: &str = "program_idl";
pub const PROGRAM: &str = "program";
pub const ADDRESS: &str = "address";
pub const MULTISIG_ACCOUNT_PUBLIC_KEY: &str = "multisig_account_public_key";
pub const INSTRUCTION: &str = "instruction";
pub const PUBLIC_KEY: &str = "public_key";
pub const TRANSACTION_BYTES: &str = "transaction_bytes";
pub const PARTIALLY_SIGNED_TRANSACTION_BYTES: &str = "partially_signed_transaction_bytes";
pub const UPDATED_PARTIALLY_SIGNED_TRANSACTION: &str = "updated_partially_signed_transaction";
pub const PREVIOUSLY_SIGNED_BLOCKHASH: &str = "previously_signed_blockhash";
pub const NETWORK_ID: &str = "network_id";
pub const AUTO_EXTEND: &str = "auto_extend";
pub const COMMITMENT_LEVEL: &str = "commitment_level";
pub const DO_AWAIT_CONFIRMATION: &str = "do_await_confirmation";
pub const SIGNATURE: &str = "signature";
pub const SIGNATURES: &str = "signatures";
pub const IS_DEPLOYMENT: &str = "is_deployment";
pub const AMOUNT: &str = "amount";
pub const RECIPIENT: &str = "recipient";
pub const TOKEN: &str = "token";
pub const FUND_RECIPIENT: &str = "fund_recipient";
pub const AUTHORITY_ADDRESS: &str = "authority_address";
pub const RECIPIENT_ADDRESS: &str = "recipient_address";
pub const RECIPIENT_TOKEN_ADDRESS: &str = "recipient_token_address";
pub const SOURCE_TOKEN_ADDRESS: &str = "source_token_address";
pub const TOKEN_MINT_ADDRESS: &str = "token_mint_address";
pub const IS_FUNDING_RECIPIENT: &str = "is_funding_recipient";
pub const SET_ACCOUNT: &str = "set_account";
pub const RESET_ACCOUNT: &str = "reset_account";
pub const STREAM_ACCOUNT: &str = "stream_account";
pub const SET_TOKEN_ACCOUNT: &str = "set_token_account";
pub const CLONE_PROGRAM_ACCOUNT: &str = "clone_program_account";
pub const SET_PROGRAM_AUTHORITY: &str = "set_program_authority";
pub const DEPLOY_PROGRAM: &str = "deploy_program";
pub const INSTANT_SURFNET_DEPLOYMENT: &str = "instant_surfnet_deployment";
pub const IS_SURFNET: &str = "is_surfnet";
pub const DEPLOYMENT_TRANSACTION_TYPE: &str = "deployment_transaction_type";
pub const EPHEMERAL_AUTHORITY_SECRET_KEY: &str = "ephemeral_authority_secret_key";
pub const BUFFER_ACCOUNT_PUBKEY: &str = "buffer_account_pubkey";
pub const DEPLOYMENT_TRANSACTIONS: &str = "deployment_transactions";

// Subgraph keys
pub const BLOCK_HEIGHT: &str = "block_height";
pub const SLOT: &str = "slot";
pub const SUBGRAPH_NAME: &str = "subgraph_name";
pub const SUBGRAPH_REQUEST: &str = "subgraph_request";
pub const SUBGRAPH_URL: &str = "subgraph_url";
pub const SUBGRAPH_ENDPOINT_URL: &str = "subgraph_endpoint_url";
pub const DO_INCLUDE_TOKEN: &str = "do_include_token";

// Actions items keys

// Subgraph endpoints
pub const MAINNET_SUBGRAPH_ENDPOINT: &str = "https://svm-cloud-api.txtx.run/v1/subgraphs";
pub const DEVNET_SUBGRAPH_ENDPOINT: &str = "https://svm-cloud-api.txtx.run/v1/subgraphs";

// Squads keys
pub const VAULT_INDEX: &str = "vault_index";
pub const SQUADS_FRONTEND_URL: &str = "squads_frontend_url";
pub const CREATE_KEY: &str = "create_key";
pub const MULTISIG_ACCOUNT_ADDRESS: &str = "multisig_account_address";
pub const SQUADS_MULTISIG: &str = "squads_multisig";
pub const INITIATOR: &str = "initiator";
pub const VAULT_ADDRESS: &str = "vault_address";
pub const VAULT_PUBLIC_KEY: &str = "vault_public_key";
pub const IS_SQUADS_AUTHORITY: &str = "is_squads_authority";
