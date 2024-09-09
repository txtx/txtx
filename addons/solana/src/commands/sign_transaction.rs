use serde::de::value;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::CompiledInstruction;
use solana_sdk::message::{Message, MessageHeader};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use txtx_addon_kit::types::stores::ValueStore;
use std::collections::HashMap;
use txtx_addon_kit::constants::SIGNED_TRANSACTION_BYTES;
use txtx_addon_kit::types::commands::{
    CommandExecutionResult, CommandImplementation, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::signers::{
    return_synchronous_ok, SignerActionsFutureResult, SignerInstance, SignerSignFutureResult,
    SignersState,
};
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::{commands::CommandSpecification, diagnostics::Diagnostic, types::Type};
use txtx_addon_kit::types::ConstructDid;

use crate::constants::{TRANSACTION_MESSAGE_BYTES, UNSIGNED_TRANSACTION_BYTES};

use super::get_signer_did;

lazy_static! {
    pub static ref SIGN_SOLANA_TRANSACTION: PreCommandSpecification = define_command! {
      SignSolanaTransaction => {
          name: "Sign Solana Transaction",
          matcher: "sign_transaction",
          documentation: "The `solana::send_transaction` is coming soon.",
          implements_signing_capability: true,
          implements_background_task_capability: false,
          inputs: [
            description: {
                documentation: "Description of the transaction",
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false
            },
            transaction_payload_bytes: {
                documentation: "The transaction payload bytes, encoded as a clarity buffer.",
                typing: Type::string(),
                optional: true,
                tainting: true,
                internal: false
            },
            chain_id: {
                documentation: indoc!{r#"Coming soon"#},
                typing: Type::string(),
                optional: true,
                tainting: true,
                internal: false
            },
            signer: {
                documentation: "A reference to a signer construct, which will be used to sign the transaction payload.",
                typing: Type::string(),
                optional: false,
                tainting: true,
                internal: false
            },
            nonce: {
                documentation: "The account nonce of the signer. This value will be retrieved from the network if omitted.",
                typing: Type::integer(),
                optional: true,
                tainting: false,
                internal: false
            }
          ],
          outputs: [
              signed_transaction_bytes: {
                  documentation: "The signed transaction bytes.",
                  typing: Type::string()
              },
              chain_id: {
                  documentation: "Coming soon.",
                  typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
          // Coming soon
      "#},
      }
    };
}

pub struct SignSolanaTransaction;
impl CommandImplementation for SignSolanaTransaction {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    #[cfg(not(feature = "wasm"))]
    fn check_signed_executability(
        construct_did: &ConstructDid,
        instance_name: &str,
        spec: &CommandSpecification,
        values: &ValueStore,
        supervision_context: &RunbookSupervisionContext,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerActionsFutureResult {
        use txtx_addon_kit::{constants::SIGNATURE_APPROVED, types::types::Value};

        use crate::{constants::CHECKED_PUBLIC_KEY, typing::SolanaValue};

        let signer_did = get_signer_did(values).unwrap();
        let signer = signers_instances.get(&signer_did).unwrap().clone();
        let construct_did = construct_did.clone();
        let instance_name = instance_name.to_string();
        let spec = spec.clone();
        let args = values.clone();
        let supervision_context = supervision_context.clone();
        let signers_instances = signers_instances.clone();

        let future = async move {
            let mut actions = Actions::none();
            let signer_state = signers.pop_signer_state(&signer_did).unwrap();
            if signer_state
                .get_scoped_value(&construct_did.to_string(), SIGNED_TRANSACTION_BYTES)
                .is_some()
                || signer_state
                    .get_scoped_value(&construct_did.to_string(), SIGNATURE_APPROVED)
                    .is_some()
            {
                return Ok((signers, signer_state, Actions::none()));
            }

            let transaction = build_transaction().unwrap();
            let mut instructions = vec![];
            for instruction in transaction.message.instructions.iter() {
                let instruction_bytes = bincode::serialize(instruction).unwrap();
                instructions.push(SolanaValue::instruction(instruction_bytes));
            }
            let payload = Value::array(instructions);

            let description =
                args.get_expected_string("description").ok().and_then(|d| Some(d.to_string()));

            let (signers, signer_state, mut signer_actions) =
                (signer.specification.check_signability)(
                    &construct_did,
                    &instance_name,
                    &description,
                    &payload,
                    &signer.specification,
                    &args,
                    signer_state,
                    signers,
                    &signers_instances,
                    &supervision_context,
                )?;
            actions.append(&mut signer_actions);
            return Ok((signers, signer_state, actions));
        };
        Ok(Box::pin(future))
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerSignFutureResult {
        let signer_did = get_signer_did(values).unwrap();
        let signer_state = signers.pop_signer_state(&signer_did).unwrap();

        if let Ok(signed_transaction_bytes) = values.get_expected_value(SIGNED_TRANSACTION_BYTES) {
            let mut result = CommandExecutionResult::new();
            result
                .outputs
                .insert(SIGNED_TRANSACTION_BYTES.into(), signed_transaction_bytes.clone());
            return return_synchronous_ok(signers, signer_state, result);
        }

        let signer = signers_instances.get(&signer_did).unwrap();

        let payload = signer_state
            .get_scoped_value(&construct_did.to_string(), TRANSACTION_MESSAGE_BYTES)
            .unwrap()
            .clone();

        let title = values.get_expected_string("description").unwrap_or("New Transaction".into());

        let res = (signer.specification.sign)(
            construct_did,
            title,
            &payload,
            &signer.specification,
            &values,
            signer_state,
            signers,
            signers_instances,
        );
        res
    }
}

pub fn build_transaction() -> Result<Transaction, Diagnostic> {
    let tx = Transaction {
        signatures: vec![],
        message: Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            account_keys: vec![],
            recent_blockhash: Hash::new_unique(),
            instructions: vec![CompiledInstruction {
                program_id_index: 0,
                accounts: vec![],
                data: vec![],
            }],
        },
    };

    Ok(tx)
}
