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
            documentation: "Send an encoded transaction payload",
            parts: [ENCODE_STACKS_CONTRACT_CALL.clone(), SIGN_STACKS_TRANSACTION.clone(), BROADCAST_STACKS_TRANSACTION.clone()],
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
            r#"prompt "{}" "stacks::{}" {{
                  transaction_payload_bytes = action.{}.bytes
                  network_id = action.{}.network_id
            }}"#,
            signed_call_name, sign_tx.matcher, encoded_call_name, encoded_call_name
        );

        let block_2 = format!(
            r#"action "{}" "stacks::{}" {{
                  signed_transaction_bytes = prompt.{}.signed_transaction_bytes
                  network_id = prompt.{}.network_id
            }}"#,
            command_instance_name, broadcast_tx.matcher, signed_call_name, signed_call_name
        );
        Ok(vec![block_0, block_1, block_2])
    }
}
