use txtx_addon_kit::types::{
    commands::{CommandSpecification, CompositeCommandImplementation, PreCommandSpecification},
    diagnostics::Diagnostic,
};

use crate::commands::actions::{
    broadcast_transaction::BROADCAST_STACKS_TRANSACTION,
    encode_contract_call::ENCODE_STACKS_CONTRACT_CALL, sign_transaction::SIGN_STACKS_TRANSACTION,
};

lazy_static! {
    pub static ref SEND_CONTRACT_CALL: PreCommandSpecification = define_multistep_command! {
        SendContractCall => {
            name: "Send Contract Call Transaction",
            matcher: "send_contract_call",
            documentation: "The `send_contract_call` action encodes a contract call transaction, signs the transaction using an in-browser wallet, and broadcasts the signed transaction to the network.",
            parts: [ENCODE_STACKS_CONTRACT_CALL.clone(), SIGN_STACKS_TRANSACTION.clone(), BROADCAST_STACKS_TRANSACTION.clone()],
            example: txtx_addon_kit::indoc! {r#"
              action "my_ref" "stacks::send_contract_call" {
                  description = "Encodes the contract call, sign, and broadcasts the set-token function."
                  contract_id = "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.pyth-oracle-v1"
                  function_name = "verify-and-update-price-feeds"
                  function_args = [
                      encode_buffer(output.bitcoin_price_feed),
                      encode_tuple({
                          "pyth-storage-contract": encode_principal("${env.pyth_deployer}.pyth-store-v1"),
                          "pyth-decoder-contract": encode_principal("${env.pyth_deployer}.pyth-pnau-decoder-v1"),
                          "wormhole-core-contract": encode_principal("${env.pyth_deployer}.wormhole-core-v1")
                      })
                  ]
              }            
              output "tx_id" {
                value = action.my_ref.tx_id
              }
              output "result" {
                value = action.my_ref.result
              }
              // > tx_id: 0x...
              // > result: success
          "#},
        }
    };
}

pub struct SendContractCall;
impl CompositeCommandImplementation for SendContractCall {
    fn router(
        first_input_body: &String,
        command_instance_name: &String,
        parts: &Vec<PreCommandSpecification>,
    ) -> Result<Vec<String>, Diagnostic> {
        let (
            Some(PreCommandSpecification::Atomic(encode_contract_call)),
            Some(PreCommandSpecification::Atomic(sign_tx)),
            Some(PreCommandSpecification::Atomic(broadcast_tx)),
        ) = (parts.get(0), parts.get(1), parts.get(2))
        else {
            panic!("send_contract_call should have three atomic command specifications");
        };
        let encoded_call_name = format!("encoded_{}", command_instance_name);
        let signed_call_name = format!("signed_{}", command_instance_name);
        let block_0 = format!(
            r#"action "{}" "stacks::{}" {{
              {}
            }}"#,
            encoded_call_name, encode_contract_call.matcher, first_input_body
        );

        let block_1 = format!(
            r#"action "{}" "stacks::{}" {{
                  transaction_payload_bytes = action.{}.bytes
                  network_id = action.{}.network_id
                  signer = action.{}.signer
            }}"#,
            signed_call_name,
            sign_tx.matcher,
            encoded_call_name,
            encoded_call_name,
            encoded_call_name
        );

        let block_2 = format!(
            r#"action "{}" "stacks::{}" {{
                  signed_transaction_bytes = action.{}.signed_transaction_bytes
                  network_id = action.{}.network_id
            }}"#,
            command_instance_name, broadcast_tx.matcher, signed_call_name, signed_call_name
        );
        Ok(vec![block_0, block_1, block_2])
    }
}
