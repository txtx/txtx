use std::collections::HashMap;

use kaigan::types::RemainderStr;
use solana_client::rpc_client::RpcClient;
use solana_record_service_sdk::instructions::{CreateClass, CreateClassInstructionArgs};
use solana_record_service_sdk::programs::SOLANA_RECORD_SERVICE_ID;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::system_program;
use solana_sdk::transaction::Transaction;
use txtx_addon_kit::channel;
use txtx_addon_kit::types::cloud_interface::CloudServiceContext;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandImplementation, CommandSpecification,
    PreCommandSpecification,
};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::{BlockEvent, StatusUpdater};
use txtx_addon_kit::types::signers::{
    SignerActionsFutureResult, SignerInstance, SignerSignFutureResult, SignersState,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::uuid::Uuid;

use crate::codec::send_transaction::send_transaction_background_task;
use crate::commands::get_signer_did;
use crate::commands::sign_transaction::SignTransaction;
use crate::constants::{CHECKED_PUBLIC_KEY, RPC_API_URL, TRANSACTION_BYTES};
use crate::typing::SvmValue;

use super::to_u8_prefix_string;

const CLASS_PREFIX: &[u8] = b"class";

lazy_static! {
    pub static ref CREATE_CLASS: PreCommandSpecification = {
        let mut command = define_command! {
            ProcessInstructions => {
                name: "Create a Class with the Solana Record Service program",
                matcher: "create_class",
                documentation: "The `svm::create_class` action is coming soon.",
                implements_signing_capability: true,
                implements_background_task_capability: true,
                inputs: [
                    description: {
                        documentation: "A description of the record.",
                        typing: Type::string(),
                        optional: true,
                        tainting: false,
                        internal: false,
                        sensitive: false
                    },
                    signer: {
                        documentation: "A reference to a signer construct, which will be used to sign the transaction.",
                        typing: Type::array(Type::string()),
                        optional: false,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    name: {
                        documentation: "The name of the class. This must be a valid UTF-8 string that is less than 256 bytes long.",
                        typing: Type::string(),
                        optional: false,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    metadata: {
                        documentation: "The metadata of the class. This must be a valid UTF-8 string.",
                        typing: Type::string(),
                        optional: false,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    is_permissioned: {
                        documentation: "Whether the class is permissioned. The default is true.",
                        typing: Type::bool(),
                        optional: true,
                        tainting: false,
                        internal: false,
                        sensitive: false
                    },
                    is_frozen: {
                        documentation: "Whether the class is frozen. The default is false.",
                        typing: Type::bool(),
                        optional: true,
                        tainting: false,
                        internal: false,
                        sensitive: false
                    },
                    rpc_api_url: {
                        documentation: "The URL to use when making API requests.",
                        typing: Type::string(),
                        optional: false,
                        tainting: false,
                        internal: false,
                        sensitive: false
                    },
                    rpc_api_auth_token: {
                        documentation: "The HTTP authentication token to include in the headers when making API requests.",
                        typing: Type::string(),
                        optional: true,
                        tainting: false,
                        internal: false,
                        sensitive: true
                    }
                ],
                outputs: [
                    signature: {
                        documentation: "The transaction computed signature.",
                        typing: Type::string()
                    },
                    name: {
                        documentation: "The name of the class.",
                        typing: Type::string()
                    },
                    metadata: {
                        documentation: "The metadata of the class.",
                        typing: Type::string()
                    },
                    public_key: {
                        documentation: "The public key of the created class.",
                        typing: Type::string()
                    }
                ],
                example: txtx_addon_kit::indoc! {r#"
                    action "my_class" "svm::create_class" {
                        name = "my_class"
                        metadata = "metadata string"
                        is_permissioned = true
                        is_frozen = false
                        signer = signer.creator
                    }
                "#},
            }
        };
        if let PreCommandSpecification::Atomic(ref mut spec) = command {
            spec.create_critical_output = Some("public_key".to_string());
        }

        command
    };
}

pub struct ProcessInstructions;
impl CommandImplementation for ProcessInstructions {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_signed_executability(
        construct_did: &ConstructDid,
        instance_name: &str,
        spec: &CommandSpecification,
        args: &ValueStore,
        supervision_context: &RunbookSupervisionContext,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerActionsFutureResult {
        let signer_did = get_signer_did(args).unwrap();
        let mut signer_state = signers.pop_signer_state(&signer_did).unwrap();

        let rpc_api_url = args
            .get_expected_string(RPC_API_URL)
            .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?
            .to_string();

        let authority = signer_state
            .get_expected_value(CHECKED_PUBLIC_KEY)
            .and_then(|key| SvmValue::to_pubkey(key).map_err(Into::into))
            .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;

        let name_str = args
            .get_expected_string("name")
            .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;

        let name = to_u8_prefix_string(name_str).map_err(|diag| {
            (
                signers.clone(),
                signer_state.clone(),
                format!("invalid name '{}' for class: {}", name_str, diag).into(),
            )
        })?;

        let metadata = args
            .get_expected_string("metadata")
            .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;

        let is_permissioned = args.get_bool("is_permissioned").unwrap_or(true);

        let is_frozen = args.get_bool("is_frozen").unwrap_or(false);

        let class_seeds = [CLASS_PREFIX, authority.as_ref(), name_str.as_bytes()];
        let (class, _) = Pubkey::find_program_address(&class_seeds[..], &SOLANA_RECORD_SERVICE_ID);

        // store the signer state so we can store it in our outputs later
        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            "class",
            SvmValue::pubkey(class.to_bytes().to_vec()),
        );

        let instruction = CreateClass { authority, class, system_program: system_program::ID }
            .instruction(CreateClassInstructionArgs {
                is_permissioned,
                is_frozen,
                name,
                metadata: RemainderStr::from(metadata.to_string()),
            });

        let mut message = Message::new(&[instruction], None);
        let client = RpcClient::new(rpc_api_url);
        message.recent_blockhash = client.get_latest_blockhash().map_err(|e| {
            (
                signers.clone(),
                signer_state.clone(),
                diagnosed_error!("failed to get latest blockhash: {e}"),
            )
        })?;
        let transaction = SvmValue::transaction(&Transaction::new_unsigned(message))
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

        let mut args = args.clone();
        args.insert(TRANSACTION_BYTES, transaction);

        signers.push_signer_state(signer_state);
        SignTransaction::check_signed_executability(
            construct_did,
            instance_name,
            spec,
            &args,
            supervision_context,
            signers_instances,
            signers,
        )
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        args: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        signers: SignersState,
    ) -> SignerSignFutureResult {
        let progress_tx = progress_tx.clone();
        let args = args.clone();
        let signers_instances = signers_instances.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();

        let args = args.clone();
        let future = async move {
            let run_signing_future = SignTransaction::run_signed_execution(
                &construct_did,
                &spec,
                &args,
                &progress_tx,
                &signers_instances,
                signers,
            );
            let (signers, signer_state, mut result) = match run_signing_future {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(err) => return Err(err),
                },
                Err(err) => return Err(err),
            };

            result.insert(
                "class",
                signer_state.get_scoped_value(&construct_did.to_string(), "class").unwrap().clone(),
            );

            Ok((signers, signer_state, result))
        };
        Ok(Box::pin(future))
    }

    fn build_background_task(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        inputs: &ValueStore,
        outputs: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
        _cloud_service_context: &Option<CloudServiceContext>,
    ) -> CommandExecutionFutureResult {
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let inputs = inputs.clone();
        let outputs = outputs.clone();
        let progress_tx = progress_tx.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();
        let supervision_context = supervision_context.clone();

        let future = async move {
            let name = inputs.get_value("name").unwrap();
            let metadata = inputs.get_value("metadata").unwrap();
            let class = outputs.get_value("class").unwrap();

            let mut status_updater =
                StatusUpdater::new(&background_tasks_uuid, &construct_did, &progress_tx);

            let mut result = match send_transaction_background_task(
                &construct_did,
                &spec,
                &inputs,
                &outputs,
                &progress_tx,
                &background_tasks_uuid,
                &supervision_context,
            ) {
                Ok(res) => match res.await {
                    Ok(res) => res,
                    Err(e) => return Err(e),
                },
                Err(e) => return Err(e),
            };

            status_updater.propagate_success_status(
                "Class Created",
                &format!("Class {} created", name.as_string().unwrap()),
            );

            result.insert("name", name.clone());
            result.insert("metadata", metadata.clone());
            result.insert("public_key", class.clone());
            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
