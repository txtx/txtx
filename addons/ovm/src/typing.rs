use txtx_addon_kit::types::types::Type;

pub struct OvmValue {}

impl OvmValue {}

lazy_static! {
    pub static ref ROLLUP_CONFIG_TYPE: Type = define_map_type! {
        // required fields
        l1_chain_id: {
            documentation: "The L1 chain id.",
            typing: Type::integer(),
            optional: false,
            tainting: true
        },
        l2_chain_id: {
            documentation: "The L2 chain id.",
            typing: Type::integer(),
            optional: false,
            tainting: true
        },
        p2p_sequencer_address: {
            documentation: "The P2P sequencer address.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        batch_sender_address: {
            documentation: "The batch sender address.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        l2_output_oracle_proposer: {
            documentation: "The L2 output oracle proposer.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        proxy_admin_owner: {
            documentation: "The address of the proxy admin owner.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        // optional fields
        l1_block_time: {
            documentation: "The L1 block time.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        l2_block_time: {
            documentation: "The L2 block time.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        max_sequencer_drift: {
            documentation: "The max sequencer drift.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        sequencer_window_size: {
            documentation: "The sequencer window size.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        channel_timeout: {
            documentation: "The channel timeout.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        batch_inbox_address: {
            documentation: "The batch inbox address.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        batch_sender_address: {
            documentation: "The batch sender address.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        l2_output_oracle_submission_interval: {
            documentation: "The L2 output oracle submission interval.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        l2_output_oracle_starting_block_number: {
            documentation: "The L2 output oracle starting block number.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        l2_output_oracle_starting_timestamp: {
            documentation: "The L2 output oracle starting timestamp.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        l2_output_oracle_challenger: {
            documentation: "The L2 output oracle challenger.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        finalization_period_seconds: {
            documentation: "The finalization period in seconds.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        base_fee_vault_recipient: {
            documentation: "The base fee vault recipient.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        l1_fee_vault_recipient: {
            documentation: "The L1 fee vault recipient.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        sequencer_fee_vault_recipient: {
            documentation: "The sequencer fee vault recipient.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        final_system_owner: {
            documentation: "The final system owner.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        superchain_config_guardian: {
            documentation: "The superchain config guardian.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        base_fee_vault_minimum_withdrawal_amount: {
            documentation: "The base fee vault minimum withdrawal amount.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        l1_fee_vault_minimum_withdrawal_amount: {
            documentation: "The L1 fee vault minimum withdrawal amount.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        sequencer_fee_vault_minimum_withdrawal_amount: {
            documentation: "The sequencer fee vault minimum withdrawal amount.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        base_fee_vault_withdrawal_network: {
            documentation: "The base fee vault withdrawal network.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        l1_fee_vault_withdrawal_network: {
            documentation: "The L1 fee vault withdrawal network.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        sequencer_fee_vault_withdrawal_network: {
            documentation: "The sequencer fee vault withdrawal network.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        gas_price_oracle_overhead: {
            documentation: "The gas price oracle overhead.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        gas_price_oracle_scalar: {
            documentation: "The gas price oracle scalar.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        enable_governance: {
            documentation: "Enable governance.",
            typing: Type::bool(),
            optional: true,
            tainting: true
        },
        governance_token_symbol: {
            documentation: "The governance token symbol.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        governance_token_name: {
            documentation: "The governance token name.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        governance_token_owner: {
            documentation: "The governance token owner.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        l2_genesis_block_gas_limit: {
            documentation: "The L2 genesis block gas limit (as a hex string).",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        l2_genesis_block_base_fee_per_gas: {
            documentation: "The L2 genesis block base fee per gas (as a hex string).",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        l2_genesis_regolith_time_offset: {
            documentation: "The L2 genesis regolith time offset (as a hex string).",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        eip1559_denominator: {
            documentation: "The EIP-1559 denominator.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        eip1559_denominator_canyon: {
            documentation: "The EIP-1559 denominator canyon.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        eip1559_elasticity: {
            documentation: "The EIP-1559 elasticity.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        l2_genesis_delta_time_offset: {
            documentation: "The L2 genesis delta time offset.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        l2_genesis_canyon_time_offset: {
            documentation: "The L2 genesis canyon time offset (as a hex string).",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        system_config_start_block: {
            documentation: "The system config start block.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        required_protocol_version: {
            documentation: "The required protocol version.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        recommended_protocol_version: {
            documentation: "The recommended protocol version.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        fault_game_absolute_prestate: {
            documentation: "The fault game absolute prestate.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        fault_game_max_depth: {
            documentation: "The fault game max depth.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        fault_game_max_duration: {
            documentation: "The fault game max duration.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        fault_game_genesis_block: {
            documentation: "The fault game genesis block.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        fault_game_genesis_output_root: {
            documentation: "The fault game genesis output root.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        fault_game_split_depth: {
            documentation: "The fault game split depth.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        }
    };

    pub static ref ROLLUP_CONTAINER_IDS_TYPE: Type = define_object_type! {
        op_geth_container_id: {
            documentation: "The op-geth container id.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        op_node_container_id: {
            documentation: "The op-node container id.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        op_batcher_container_id: {
            documentation: "The op-batcher container id.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        op_proposer_container_id: {
            documentation: "The op-proposer container id.",
            typing: Type::string(),
            optional: false,
            tainting: true
        }
    };

}
