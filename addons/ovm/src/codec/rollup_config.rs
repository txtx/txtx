use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{diagnostics::Diagnostic, types::Value},
};
use txtx_addon_network_evm::rpc::EvmRpc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollupConfig {
    pub l1_starting_block_tag: String,
    pub l1_chain_id: u64,
    pub l2_chain_id: u64,
    pub l2_block_time: u64,
    pub l1_block_time: u64,
    pub max_sequencer_drift: u64,
    pub sequencer_window_size: u64,
    pub channel_timeout: u64,
    pub p2p_sequencer_address: String,
    pub batch_inbox_address: String,
    pub batch_sender_address: String,
    pub l2_output_oracle_submission_interval: u64,
    pub l2_output_oracle_starting_block_number: u64,
    pub l2_output_oracle_starting_timestamp: u64,
    pub l2_output_oracle_proposer: String,
    pub l2_output_oracle_challenger: String,
    pub finalization_period_seconds: u64,
    pub proxy_admin_owner: String,
    pub base_fee_vault_recipient: String,
    pub l1_fee_vault_recipient: String,
    pub sequencer_fee_vault_recipient: String,
    pub final_system_owner: String,
    pub superchain_config_guardian: String,
    pub base_fee_vault_minimum_withdrawal_amount: String,
    pub l1_fee_vault_minimum_withdrawal_amount: String,
    pub sequencer_fee_vault_minimum_withdrawal_amount: String,
    pub base_fee_vault_withdrawal_network: u64,
    pub l1_fee_vault_withdrawal_network: u64,
    pub sequencer_fee_vault_withdrawal_network: u64,
    pub gas_price_oracle_overhead: u64,
    pub gas_price_oracle_scalar: u64,
    pub enable_governance: bool,
    pub governance_token_symbol: String,
    pub governance_token_name: String,
    pub governance_token_owner: String,
    pub l2_genesis_block_gas_limit: String,
    pub l2_genesis_block_base_fee_per_gas: String,
    pub l2_genesis_regolith_time_offset: String,
    pub eip1559_denominator: u64,
    pub eip1559_denominator_canyon: u64,
    pub eip1559_elasticity: u64,
    pub l2_genesis_delta_time_offset: Option<String>,
    pub l2_genesis_canyon_time_offset: String,
    pub system_config_start_block: u64,
    pub required_protocol_version: String,
    pub recommended_protocol_version: String,
    pub fault_game_absolute_prestate: String,
    pub fault_game_max_depth: u64,
    pub fault_game_max_duration: u64,
    pub fault_game_genesis_block: u64,
    pub fault_game_genesis_output_root: String,
    pub fault_game_split_depth: u64,
}

impl Default for RollupConfig {
    fn default() -> Self {
        RollupConfig {
            l2_output_oracle_starting_timestamp: 0,
            l1_starting_block_tag: "".into(),
            p2p_sequencer_address: "".into(),
            batch_sender_address: "".into(),
            l2_output_oracle_proposer: "".into(),
            l2_output_oracle_challenger: "".into(),
            proxy_admin_owner: "".into(),
            base_fee_vault_recipient: "".into(),
            l1_fee_vault_recipient: "".into(),
            sequencer_fee_vault_recipient: "".into(),
            final_system_owner: "".into(),
            superchain_config_guardian: "".into(),
            governance_token_owner: "".into(),
            l1_chain_id: 11155111,
            l2_chain_id: 42069,
            l2_block_time: 2,
            l1_block_time: 12,
            max_sequencer_drift: 600,
            sequencer_window_size: 3600,
            channel_timeout: 300,
            batch_inbox_address: "0xff00000000000000000000000000000000042069".into(),
            l2_output_oracle_submission_interval: 120,
            l2_output_oracle_starting_block_number: 0,
            finalization_period_seconds: 12,
            base_fee_vault_minimum_withdrawal_amount: "0x8ac7230489e80000".into(),
            l1_fee_vault_minimum_withdrawal_amount: "0x8ac7230489e80000".into(),
            sequencer_fee_vault_minimum_withdrawal_amount: "0x8ac7230489e80000".into(),
            base_fee_vault_withdrawal_network: 0,
            l1_fee_vault_withdrawal_network: 0,
            sequencer_fee_vault_withdrawal_network: 0,
            gas_price_oracle_overhead: 2100,
            gas_price_oracle_scalar: 1000000,
            enable_governance: true,
            governance_token_symbol: "OP".into(),
            governance_token_name: "Optimism".into(),
            l2_genesis_block_gas_limit: "0x1c9c380".into(),
            l2_genesis_block_base_fee_per_gas: "0x3b9aca00".into(),
            l2_genesis_regolith_time_offset: "0x0".into(),
            eip1559_denominator: 50,
            eip1559_denominator_canyon: 250,
            eip1559_elasticity: 6,
            l2_genesis_delta_time_offset: None,
            l2_genesis_canyon_time_offset: "0x0".into(),
            system_config_start_block: 0,
            required_protocol_version:
                "0x0000000000000000000000000000000000000000000000000000000000000000".into(),
            recommended_protocol_version:
                "0x0000000000000000000000000000000000000000000000000000000000000000".into(),
            fault_game_absolute_prestate:
                "0x03c7ae758795765c6664a5d39bf63841c71ff191e9189522bad8ebff5d4eca98".into(),
            fault_game_max_depth: 44,
            fault_game_max_duration: 1200,
            fault_game_genesis_block: 0,
            fault_game_genesis_output_root:
                "0x0000000000000000000000000000000000000000000000000000000000000000".into(),
            fault_game_split_depth: 14,
        }
    }
}

impl RollupConfig {
    pub async fn new(values: &Vec<Value>, l1_rpc_api_url: &str) -> Result<Self, Diagnostic> {
        if values.len() != 1 {
            return Err(diagnosed_error!("Rollup configuration must contain exactly one entry"));
        }
        let values = values
            .first()
            .unwrap()
            .as_object()
            .ok_or(diagnosed_error!("Rollup configuration must be an object"))?;

        let mut default = RollupConfig::default();

        // first, fetch any data from the rpc and set those fields
        default.set_block_fields(&values, l1_rpc_api_url).await.map_err(|e| {
            diagnosed_error!("failed to fetch l1 block data for rollup config: {}", e.message)
        })?;

        let l1_chain_id = values
            .get("l1_chain_id")
            .and_then(|v| v.as_uint())
            .transpose()
            .map_err(|e| {
                diagnosed_error!("invalid chain id for field 'l1_chain_id' for rollup config: {e}")
            })?
            .ok_or(diagnosed_error!("field 'l1_chain_id' is required for rollup config"))?;

        let l2_chain_id = values
            .get("l2_chain_id")
            .and_then(|v| v.as_uint())
            .transpose()
            .map_err(|e| {
                diagnosed_error!("invalid chain id for field 'l2_chain_id' for rollup config: {e}")
            })?
            .ok_or(diagnosed_error!("field 'l2_chain_id' is required for rollup config"))?;

        let p2p_sequencer_address =
            values.get("p2p_sequencer_address").and_then(|v| v.as_string()).ok_or(
                diagnosed_error!("field 'p2p_sequencer_address' is required for rollup config"),
            )?;

        let batch_sender_address =
            values.get("batch_sender_address").and_then(|v| v.as_string()).ok_or(
                diagnosed_error!("field 'batch_sender_address' is required for rollup config"),
            )?;

        let l2_output_oracle_proposer =
            values.get("l2_output_oracle_proposer").and_then(|v| v.as_string()).ok_or(
                diagnosed_error!("field 'l2_output_oracle_proposer' is required for rollup config"),
            )?;

        let proxy_admin_owner = values
            .get("proxy_admin_owner")
            .and_then(|v| v.as_string())
            .ok_or(diagnosed_error!("field 'proxy_admin_owner' is required for rollup config"))?;

        // set all of the required fields
        default.l1_chain_id = l1_chain_id;
        default.l2_chain_id = l2_chain_id;
        default.set_admin_address_fields(proxy_admin_owner);
        default.set_sequencer_address_fields(p2p_sequencer_address);
        default.set_batcher_address_fields(batch_sender_address);
        default.set_proposer_address_fields(l2_output_oracle_proposer);

        // override with optional fields if provided
        if let Some(l1_block_time) = RollupConfig::try_get_uint(values, "l1_block_time")? {
            default.l1_block_time = l1_block_time;
        }

        if let Some(l2_block_time) = RollupConfig::try_get_uint(values, "l2_block_time")? {
            default.l2_block_time = l2_block_time;
        }

        if let Some(max_sequencer_drift) =
            RollupConfig::try_get_uint(values, "max_sequencer_drift")?
        {
            default.max_sequencer_drift = max_sequencer_drift;
        }

        if let Some(sequencer_window_size) =
            RollupConfig::try_get_uint(values, "sequencer_window_size")?
        {
            default.sequencer_window_size = sequencer_window_size;
        }

        if let Some(channel_timeout) = RollupConfig::try_get_uint(values, "channel_timeout")? {
            default.channel_timeout = channel_timeout;
        }

        if let Some(batch_inbox_address) =
            values.get("batch_inbox_address").and_then(|v| v.as_string())
        {
            default.batch_inbox_address = batch_inbox_address.to_string();
        }

        if let Some(l2_output_oracle_submission_interval) =
            RollupConfig::try_get_uint(values, "l2_output_oracle_submission_interval")?
        {
            default.l2_output_oracle_submission_interval = l2_output_oracle_submission_interval;
        }

        if let Some(l2_output_oracle_challenger) =
            values.get("l2_output_oracle_challenger").and_then(|v| v.as_string())
        {
            default.l2_output_oracle_challenger = l2_output_oracle_challenger.to_string();
        }

        if let Some(finalization_period_seconds) =
            RollupConfig::try_get_uint(values, "finalization_period_seconds")?
        {
            default.finalization_period_seconds = finalization_period_seconds;
        }

        if let Some(base_fee_vault_recipient) =
            values.get("base_fee_vault_recipient").and_then(|v| v.as_string())
        {
            default.base_fee_vault_recipient = base_fee_vault_recipient.to_string();
        }

        if let Some(l1_fee_vault_recipient) =
            values.get("l1_fee_vault_recipient").and_then(|v| v.as_string())
        {
            default.l1_fee_vault_recipient = l1_fee_vault_recipient.to_string();
        }

        if let Some(sequencer_fee_vault_recipient) =
            values.get("sequencer_fee_vault_recipient").and_then(|v| v.as_string())
        {
            default.sequencer_fee_vault_recipient = sequencer_fee_vault_recipient.to_string();
        }

        if let Some(final_system_owner) =
            values.get("final_system_owner").and_then(|v| v.as_string())
        {
            default.final_system_owner = final_system_owner.to_string();
        }

        if let Some(superchain_config_guardian) =
            values.get("superchain_config_guardian").and_then(|v| v.as_string())
        {
            default.superchain_config_guardian = superchain_config_guardian.to_string();
        }

        if let Some(base_fee_vault_minimum_withdrawal_amount) =
            values.get("base_fee_vault_minimum_withdrawal_amount").and_then(|v| v.as_string())
        {
            default.base_fee_vault_minimum_withdrawal_amount =
                base_fee_vault_minimum_withdrawal_amount.to_string();
        }

        if let Some(l1_fee_vault_minimum_withdrawal_amount) =
            values.get("l1_fee_vault_minimum_withdrawal_amount").and_then(|v| v.as_string())
        {
            default.l1_fee_vault_minimum_withdrawal_amount =
                l1_fee_vault_minimum_withdrawal_amount.to_string();
        }

        if let Some(sequencer_fee_vault_minimum_withdrawal_amount) =
            values.get("sequencer_fee_vault_minimum_withdrawal_amount").and_then(|v| v.as_string())
        {
            default.sequencer_fee_vault_minimum_withdrawal_amount =
                sequencer_fee_vault_minimum_withdrawal_amount.to_string();
        }

        if let Some(base_fee_vault_withdrawal_network) =
            RollupConfig::try_get_uint(values, "base_fee_vault_withdrawal_network")?
        {
            default.base_fee_vault_withdrawal_network = base_fee_vault_withdrawal_network;
        }

        if let Some(l1_fee_vault_withdrawal_network) =
            RollupConfig::try_get_uint(values, "l1_fee_vault_withdrawal_network")?
        {
            default.l1_fee_vault_withdrawal_network = l1_fee_vault_withdrawal_network;
        }

        if let Some(sequencer_fee_vault_withdrawal_network) =
            RollupConfig::try_get_uint(values, "sequencer_fee_vault_withdrawal_network")?
        {
            default.sequencer_fee_vault_withdrawal_network = sequencer_fee_vault_withdrawal_network;
        }

        if let Some(gas_price_oracle_overhead) =
            RollupConfig::try_get_uint(values, "gas_price_oracle_overhead")?
        {
            default.gas_price_oracle_overhead = gas_price_oracle_overhead;
        }

        if let Some(gas_price_oracle_scalar) =
            RollupConfig::try_get_uint(values, "gas_price_oracle_scalar")?
        {
            default.gas_price_oracle_scalar = gas_price_oracle_scalar;
        }

        if let Some(enable_governance) = values.get("enable_governance").and_then(|v| v.as_bool()) {
            default.enable_governance = enable_governance;
        }

        if let Some(governance_token_symbol) =
            values.get("governance_token_symbol").and_then(|v| v.as_string())
        {
            default.governance_token_symbol = governance_token_symbol.to_string();
        }

        if let Some(governance_token_name) =
            values.get("governance_token_name").and_then(|v| v.as_string())
        {
            default.governance_token_name = governance_token_name.to_string();
        }

        if let Some(governance_token_owner) =
            values.get("governance_token_owner").and_then(|v| v.as_string())
        {
            default.governance_token_owner = governance_token_owner.to_string();
        }

        if let Some(l2_genesis_block_gas_limit) =
            values.get("l2_genesis_block_gas_limit").and_then(|v| v.as_string())
        {
            default.l2_genesis_block_gas_limit = l2_genesis_block_gas_limit.to_string();
        }

        if let Some(l2_genesis_block_base_fee_per_gas) =
            values.get("l2_genesis_block_base_fee_per_gas").and_then(|v| v.as_string())
        {
            default.l2_genesis_block_base_fee_per_gas =
                l2_genesis_block_base_fee_per_gas.to_string();
        }

        if let Some(l2_genesis_regolith_time_offset) =
            values.get("l2_genesis_regolith_time_offset").and_then(|v| v.as_string())
        {
            default.l2_genesis_regolith_time_offset = l2_genesis_regolith_time_offset.to_string();
        }

        if let Some(eip1559_denominator) =
            RollupConfig::try_get_uint(values, "eip1559_denominator")?
        {
            default.eip1559_denominator = eip1559_denominator;
        }

        if let Some(eip1559_denominator_canyon) =
            RollupConfig::try_get_uint(values, "eip1559_denominator_canyon")?
        {
            default.eip1559_denominator_canyon = eip1559_denominator_canyon;
        }

        if let Some(eip1559_elasticity) = RollupConfig::try_get_uint(values, "eip1559_elasticity")?
        {
            default.eip1559_elasticity = eip1559_elasticity;
        }

        if let Some(l2_genesis_delta_time_offset) =
            values.get("l2_genesis_delta_time_offset").and_then(|v| v.as_string())
        {
            default.l2_genesis_delta_time_offset = Some(l2_genesis_delta_time_offset.to_string());
        }

        if let Some(l2_genesis_canyon_time_offset) =
            values.get("l2_genesis_canyon_time_offset").and_then(|v| v.as_string())
        {
            default.l2_genesis_canyon_time_offset = l2_genesis_canyon_time_offset.to_string();
        }

        if let Some(system_config_start_block) =
            RollupConfig::try_get_uint(values, "system_config_start_block")?
        {
            default.system_config_start_block = system_config_start_block;
        }

        if let Some(required_protocol_version) =
            values.get("required_protocol_version").and_then(|v| v.as_string())
        {
            default.required_protocol_version = required_protocol_version.to_string();
        }

        if let Some(recommended_protocol_version) =
            values.get("recommended_protocol_version").and_then(|v| v.as_string())
        {
            default.recommended_protocol_version = recommended_protocol_version.to_string();
        }

        if let Some(fault_game_absolute_prestate) =
            values.get("fault_game_absolute_prestate").and_then(|v| v.as_string())
        {
            default.fault_game_absolute_prestate = fault_game_absolute_prestate.to_string();
        }

        if let Some(fault_game_max_depth) =
            RollupConfig::try_get_uint(values, "fault_game_max_depth")?
        {
            default.fault_game_max_depth = fault_game_max_depth;
        }

        if let Some(fault_game_max_duration) =
            RollupConfig::try_get_uint(values, "fault_game_max_duration")?
        {
            default.fault_game_max_duration = fault_game_max_duration;
        }

        if let Some(fault_game_genesis_block) =
            RollupConfig::try_get_uint(values, "fault_game_genesis_block")?
        {
            default.fault_game_genesis_block = fault_game_genesis_block;
        }

        if let Some(fault_game_genesis_output_root) =
            values.get("fault_game_genesis_output_root").and_then(|v| v.as_string())
        {
            default.fault_game_genesis_output_root = fault_game_genesis_output_root.to_string();
        }

        if let Some(fault_game_split_depth) =
            RollupConfig::try_get_uint(values, "fault_game_split_depth")?
        {
            default.fault_game_split_depth = fault_game_split_depth;
        }

        Ok(default)
    }

    async fn set_block_fields(
        &mut self,
        values: &IndexMap<String, Value>,
        l1_rpc_api_url: &str,
    ) -> Result<(), Diagnostic> {
        let l1_starting_block_tag = values.get("l1_starting_block_tag").and_then(|v| v.as_string());

        let l2_output_oracle_starting_timestamp = values
            .get("l2_output_oracle_starting_timestamp")
            .and_then(|v| v.as_uint())
            .transpose()
            .map_err(|e| {
                diagnosed_error!("invalid timestamp for field 'l2_output_oracle_starting_timestamp' for rollup config: {e}")
            })?;

        if let Some(starting_block_tag) = l1_starting_block_tag {
            self.l1_starting_block_tag = starting_block_tag.to_string();
            if let Some(starting_timestamp) = l2_output_oracle_starting_timestamp {
                self.l2_output_oracle_starting_timestamp = starting_timestamp;
            } else {
                // fetch the timestamp from the rpc
                let rpc = EvmRpc::new(l1_rpc_api_url).map_err(|e| {
                    diagnosed_error!("failed to fetch block timestamp from rpc: {e}")
                })?;
                let block = rpc
                    .get_block_by_hash(starting_block_tag)
                    .await
                    .map_err(|e| diagnosed_error!("failed to fetch block timestamp from rpc: {e}"))?
                    .ok_or(diagnosed_error!("block {} not found", starting_block_tag))?;
                self.l2_output_oracle_starting_timestamp = block.header.timestamp;
            }
        } else {
            // fetch the block tag from the rpc
            let rpc = EvmRpc::new(l1_rpc_api_url)
                .map_err(|e| diagnosed_error!("failed to fetch latest block from rpc: {e}"))?;
            let block = rpc
                .get_latest_block()
                .await
                .map_err(|e| diagnosed_error!("failed to fetch block latest tag from rpc: {e}"))?
                .unwrap();
            self.l1_starting_block_tag = block.header.hash.unwrap().to_string();
            self.l2_output_oracle_starting_timestamp = block.header.timestamp;
        }

        Ok(())
    }

    fn set_admin_address_fields(&mut self, admin_address: &str) {
        self.proxy_admin_owner = admin_address.into();
        self.base_fee_vault_recipient = admin_address.into();
        self.l1_fee_vault_recipient = admin_address.into();
        self.sequencer_fee_vault_recipient = admin_address.into();
        self.final_system_owner = admin_address.into();
        self.superchain_config_guardian = admin_address.into();
        self.governance_token_owner = admin_address.into();
        self.l2_output_oracle_challenger = admin_address.into();
    }

    fn set_sequencer_address_fields(&mut self, sequencer_address: &str) {
        self.p2p_sequencer_address = sequencer_address.into();
    }

    fn set_batcher_address_fields(&mut self, batcher_address: &str) {
        self.batch_sender_address = batcher_address.into();
    }

    fn set_proposer_address_fields(&mut self, proposer_address: &str) {
        self.l2_output_oracle_proposer = proposer_address.into();
    }

    fn try_get_uint(
        values: &IndexMap<String, Value>,
        key: &str,
    ) -> Result<Option<u64>, Diagnostic> {
        values.get(key).and_then(|v| v.as_uint()).transpose().map_err(|e| {
            diagnosed_error!("invalid value for field '{}' for rollup config: {e}", key)
        })
    }
}
