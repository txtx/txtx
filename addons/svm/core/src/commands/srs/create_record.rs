use std::collections::HashMap;

use kaigan::types::RemainderStr;
use solana_client::rpc_client::RpcClient;
use solana_record_service_sdk::instructions::{CreateRecord, CreateRecordInstructionArgs};
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
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type, Value};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::uuid::Uuid;

use crate::codec::send_transaction::send_transaction_background_task;
use crate::commands::get_custom_signer_did;
use crate::commands::sign_transaction::SignTransaction;
use crate::constants::{
    AUTHORITY, CHECKED_PUBLIC_KEY, OWNER, RPC_API_URL, SIGNERS, TRANSACTION_BYTES,
};
use crate::typing::SvmValue;

use super::to_u8_prefix_string;

const RECORD_PREFIX: &[u8] = b"record";

lazy_static! {
    pub static ref CREATE_RECORD: PreCommandSpecification = {
        let mut command = define_command! {
            ProcessInstructions => {
                name: "Create a record with the Solana Record Service program.",
                matcher: "create_record",
                documentation: "The `svm::create_record` action is coming soon.",
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
                    owner: {
                        documentation: "A reference to a signer construct, which will be used to sign the transaction and will be the owner of the record.",
                        typing: Type::array(Type::string()),
                        optional: false,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    authority: {
                        documentation: "An optional reference to a signer construct, which will be used to sign the transaction and will be the authority on the record.",
                        typing: Type::array(Type::string()),
                        optional: false,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    class: {
                        documentation: "The pubkey of the class to which the record belongs.",
                        typing: Type::string(),
                        optional: false,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    name: {
                        documentation: "The name of the record. This must be a valid UTF-8 string that is less than 256 bytes long.",
                        typing: Type::string(),
                        optional: false,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    data: {
                        documentation: "The data of the record. This must be a valid UTF-8 string.",
                        typing: Type::string(),
                        optional: false,
                        tainting: true,
                        internal: false,
                        sensitive: false
                    },
                    expiration: {
                        documentation: "The expiration time of the record, in seconds since the Unix epoch. If not provided, the record will not expire.",
                        typing: Type::integer(),
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
                        documentation: "The name of the record.",
                        typing: Type::string()
                    },
                    data: {
                        documentation: "The data of the record.",
                        typing: Type::string()
                    },
                    class: {
                        documentation: "The pubkey of the associated class.",
                        typing: Type::string()
                    },
                    pubkey: {
                        documentation: "The pubkey of the created record.",
                        typing: Type::string()
                    }
                ],
                example: txtx_addon_kit::indoc! {r#"
                    action "my_record" "svm::create_record" {
                        name = "my_record"
                        data = "data string"
                        class = action.create_my_class.pubkey
                        owner = signer.owner
                    }
                "#},
            }
        };
        if let PreCommandSpecification::Atomic(ref mut spec) = command {
            spec.create_critical_output = Some("pubkey".to_string());
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
        // used to track all of the signers that need to sign the transaction
        let mut signer_dids = vec![];
        let owner_signer_did = get_custom_signer_did(&args, OWNER).unwrap();
        let mut owner_signer_state = signers.pop_signer_state(&owner_signer_did).unwrap().clone();
        signer_dids.push(Value::string(owner_signer_did.to_string()));

        let owner_pubkey = owner_signer_state
            .get_expected_value(CHECKED_PUBLIC_KEY)
            .and_then(|key| SvmValue::to_pubkey(key).map_err(Into::into))
            .map_err(|diag| (signers.clone(), owner_signer_state.clone(), diag))?;

        let authority_pubkey = if let Ok(authority_did) = get_custom_signer_did(&args, AUTHORITY) {
            if owner_signer_did.ne(&authority_did) {
                // if the authority is the same as the owner, it's already been tracked above,
                // and we've already popped the owner signer state so we can't get it again
                signer_dids.push(Value::string(authority_did.to_string()));
                let authority_signer_state =
                    signers.get_signer_state(&authority_did).unwrap().clone();

                Some(
                    authority_signer_state
                        .get_expected_value(CHECKED_PUBLIC_KEY)
                        .and_then(|key| SvmValue::to_pubkey(key).map_err(Into::into))
                        .map_err(|diag| (signers.clone(), owner_signer_state.clone(), diag))?,
                )
            } else {
                Some(owner_pubkey)
            }
        } else {
            None
        };

        let rpc_api_url = args
            .get_expected_string(RPC_API_URL)
            .map_err(|diag| (signers.clone(), owner_signer_state.clone(), diag))?
            .to_string();

        let class = args
            .get_expected_value("class")
            .and_then(|key| SvmValue::to_pubkey(key).map_err(Into::into))
            .map_err(|diag| (signers.clone(), owner_signer_state.clone(), diag))?;

        let name_str = args
            .get_expected_string("name")
            .map_err(|diag| (signers.clone(), owner_signer_state.clone(), diag))?;

        let expiration = args
            .get_i64("expiration")
            .map_err(|diag| (signers.clone(), owner_signer_state.clone(), diag))?
            .unwrap_or(0);

        let name = to_u8_prefix_string(name_str).map_err(|diag| {
            (
                signers.clone(),
                owner_signer_state.clone(),
                format!("invalid name '{}' for class: {}", name_str, diag).into(),
            )
        })?;

        let data = args
            .get_expected_string("data")
            .map_err(|diag| (signers.clone(), owner_signer_state.clone(), diag))?;

        let record_seeds = [RECORD_PREFIX, class.as_ref(), name_str.as_bytes()];
        let (record, _) =
            Pubkey::find_program_address(&record_seeds[..], &SOLANA_RECORD_SERVICE_ID);

        // store the signer state so we can store it in our outputs later
        owner_signer_state.insert_scoped_value(
            &construct_did.to_string(),
            "record",
            SvmValue::pubkey(record.to_bytes().to_vec()),
        );

        let instruction = CreateRecord {
            owner: owner_pubkey,
            class,
            record,
            system_program: system_program::ID,
            authority: authority_pubkey,
        }
        .instruction(CreateRecordInstructionArgs {
            expiration,
            name,
            data: RemainderStr::from(data.to_string()),
        });

        let mut message = Message::new(&[instruction], None);
        let client = RpcClient::new(rpc_api_url);
        message.recent_blockhash = client.get_latest_blockhash().map_err(|e| {
            (
                signers.clone(),
                owner_signer_state.clone(),
                diagnosed_error!("failed to get latest blockhash: {e}"),
            )
        })?;

        let transaction = SvmValue::transaction(&Transaction::new_unsigned(message))
            .map_err(|e| (signers.clone(), owner_signer_state.clone(), e))?;

        let mut args = args.clone();
        args.insert(TRANSACTION_BYTES, transaction);
        args.insert(SIGNERS, Value::array(signer_dids));

        signers.push_signer_state(owner_signer_state);
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
        let mut args = args.clone();
        let signers_instances = signers_instances.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();

        let future = async move {
            let mut signer_dids = vec![];
            let owner_signer_did = get_custom_signer_did(&args, OWNER).unwrap();
            signer_dids.push(Value::string(owner_signer_did.to_string()));

            if let Ok(authority_did) = get_custom_signer_did(&args, AUTHORITY) {
                signer_dids.push(Value::string(authority_did.to_string()));
            };
            args.insert(SIGNERS, Value::array(signer_dids));
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
                "record",
                signer_state
                    .get_scoped_value(&construct_did.to_string(), "record")
                    .unwrap()
                    .clone(),
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
            let data = inputs.get_value("data").unwrap();
            let class = inputs.get_value("class").unwrap();
            let expiration = inputs.get_value("expiration");
            let record = outputs.get_value("record").unwrap();

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
                "Record Created",
                &format!("Record {} created", name.as_string().unwrap()),
            );

            result.insert("name", name.clone());
            result.insert("data", data.clone());
            result.insert("class", class.clone());
            if let Some(expiration) = expiration {
                result.insert("expiration", expiration.clone());
            }
            result.insert("pubkey", record.clone());
            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
