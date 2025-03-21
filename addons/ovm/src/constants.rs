pub const NAMESPACE: &str = "ovm";

pub const WORKING_DIR: &str = "working_dir";
pub const L1_RPC_API_URL: &str = "l1_rpc_api_url";
pub const L1_RPC_KIND: &str = "l1_rpc_kind";
pub const SEQUENCER_SECRET_KEY: &str = "sequencer_secret_key";
pub const BATCHER_SECRET_KEY: &str = "batcher_secret_key";
pub const PROPOSER_SECRET_KEY: &str = "proposer_secret_key";
pub const JWT: &str = "jwt";
pub const DEPLOYMENT_CONFIG: &str = "deployment_config";
pub const ROLLUP_CONFIG: &str = "rollup_config";
pub const L1_DEPLOYMENT_ADDRESSES: &str = "l1_deployment_addresses";
pub const ROLLUP_CONTAINER_IDS: &str = "rollup_container_ids";

// Docker images
pub const DEFAULT_TAG: &str = "latest";
pub const OP_GETH_IMAGE: &str = "op-geth";
pub const OP_NODE_IMAGE: &str = "us-docker.pkg.dev/oplabs-tools-artifacts/images/op-node";
pub const OP_BATCHER_IMAGE: &str = "us-docker.pkg.dev/oplabs-tools-artifacts/images/op-batcher";
pub const OP_PROPOSER_IMAGE: &str = "us-docker.pkg.dev/oplabs-tools-artifacts/images/op-proposer";

pub const DEFAULT_OP_NODE_RPC_KIND: &str = "basic";
