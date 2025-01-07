use std::collections::{HashMap, VecDeque};
use std::str::FromStr;

use solana_client::rpc_client::RpcClient;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::SIGNED_TRANSACTION_BYTES;
use txtx_addon_kit::helpers::build_diag_context_fn;
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

use crate::commands::send_transaction::SendTransaction;
use crate::constants::{
    AMOUNT, AUTHORITY, AUTHORITY_ADDRESS, CHECKED_PUBLIC_KEY, FUND_RECIPIENT, IS_FUNDING_RECIPIENT,
    NAMESPACE, RECIPIENT, RECIPIENT_ADDRESS, RECIPIENT_TOKEN_ADDRESS, RPC_API_URL,
    SOURCE_TOKEN_ADDRESS, TOKEN, TOKEN_MINT_ADDRESS, TRANSACTION_BYTES,
};
use crate::typing::{SvmValue, SVM_PUBKEY};

use super::get_signers_did;
use super::sign_transaction::SignTransaction;

lazy_static! {
    pub static ref SEND_TOKEN: PreCommandSpecification = define_command! {
        SendToken => {
            name: "Send Token",
            matcher: "send_token",
            documentation: "The `svm::send_token` action encodes a transaction which sends the specified token, signs it, and broadcasts it to the network.",
            implements_signing_capability: true,
            implements_background_task_capability: true,
            inputs: [
                description: {
                    documentation: "A description of the transaction.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                amount: {
                    documentation: "The amount, in lamports, of the token to send.",
                    typing: Type::integer(),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                token: {
                    documentation: "The program address for the token being sent. This is also known as the 'token mint account'.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                recipient: {
                    documentation: "The address of the recipient. The associated token account will be computed from this address and the token address.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                authority: {
                    documentation: "The pubkey of the authority account for the token source. If omitted, the first signer will be used.",
                    typing: Type::string(),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                fund_recipient: {
                    documentation: "If set to `true` and the recipient token account does not exist, the action will create the account and fund it, using the signer to fund the account. The default is `false`.",
                    typing: Type::bool(),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                signers: {
                    documentation: "A set of references to a signer construct, which will be used to sign the transaction.",
                    typing: Type::array(Type::string()),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                commitment_level: {
                    documentation: "The commitment level expected for considering this action as done ('processed', 'confirmed', 'finalized'). The default is 'confirmed'.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                rpc_api_url: {
                    documentation: "The URL to use when making API requests.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                rpc_api_auth_token: {
                    documentation: "The HTTP authentication token to include in the headers when making API requests.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                }
            ],
            outputs: [
                signature: {
                    documentation: "The transaction computed signature.",
                    typing: Type::string()
                },
                recipient_token_address: {
                    documentation: "The recipient token account address.",
                    typing: Type::addon(SVM_PUBKEY)
                },
                source_token_address: {
                    documentation: "The source token account address.",
                    typing: Type::addon(SVM_PUBKEY)
                },
                token_mint_address: {
                    documentation: "The token mint address.",
                    typing: Type::addon(SVM_PUBKEY)
                }
            ],
            example: txtx_addon_kit::indoc! {
                r#"action "send_sol" "svm::send_token" {
                    description = "Send some SOL"
                    amount = evm::sol_to_lamports(1)
                    signers = [signer.caller]
                    recipient = "zbBjhHwuqyKMmz8ber5oUtJJ3ZV4B6ePmANfGyKzVGV"
                    token = "3bv3j4GvMPjvvBX9QdoX27pVoWhDSXpwKZipFF1QiVr6"
                    fund_recipient = true
                }"#
            },
      }
    };
}

pub struct SendToken;
impl CommandImplementation for SendToken {
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
        let to_diag_with_ctx = build_diag_context_fn(
            instance_name.to_string(),
            format!("{}::{}", NAMESPACE, spec.matcher),
        );

        let signers_did = get_signers_did(args).unwrap();
        let signers_states = signers_did
            .iter()
            .map(|did| signers.get_signer_state(did).unwrap().clone())
            .collect::<Vec<_>>();
        let mut signer_state = signers.pop_signer_state(signers_did.first().unwrap()).unwrap();

        let amount = args
            .get_expected_uint(AMOUNT)
            .map_err(|e| (signers.clone(), signer_state.clone(), to_diag_with_ctx(e.message)))?;

        let token_mint_address =
            Pubkey::from_str(args.get_expected_string(TOKEN).map_err(|e| {
                (signers.clone(), signer_state.clone(), to_diag_with_ctx(e.message))
            })?)
            .map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    to_diag_with_ctx(format!("invalid token pubkey: {}", e.to_string())),
                )
            })?;

        let recipient =
            Pubkey::from_str(args.get_expected_string(RECIPIENT).map_err(|e| {
                (signers.clone(), signer_state.clone(), to_diag_with_ctx(e.message))
            })?)
            .map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    to_diag_with_ctx(format!("invalid recipient: {}", e.to_string())),
                )
            })?;

        let rpc_api_url = args
            .get_expected_string(RPC_API_URL)
            .map_err(|e| (signers.clone(), signer_state.clone(), to_diag_with_ctx(e.message)))?
            .to_string();

        let mut signer_pubkeys = vec![];
        for signer_state in signers_states.iter() {
            let signer_pubkey =
                signer_state.get_expected_string(CHECKED_PUBLIC_KEY).map_err(|e| {
                    (signers.clone(), signer_state.clone(), to_diag_with_ctx(e.to_string()))
                })?;
            let signer_pubkey = Pubkey::from_str(signer_pubkey).map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    to_diag_with_ctx(format!("invalid signer pubkey: {}", e.to_string())),
                )
            })?;
            signer_pubkeys.push(signer_pubkey);
        }

        // if the user has specified the authority pubkey, use it, otherwise use the first signer
        let authority_pubkey = if let Some(authority_pubkey) = args.get_string(AUTHORITY) {
            Pubkey::from_str(authority_pubkey).map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    to_diag_with_ctx(format!("invalid authority pubkey: {}", e.to_string())),
                )
            })?
        } else {
            signer_pubkeys[0].clone()
        };

        let source_token_address = spl_associated_token_account::get_associated_token_address(
            &authority_pubkey,
            &token_mint_address,
        );
        let recipient_token_address = spl_associated_token_account::get_associated_token_address(
            &recipient,
            &token_mint_address,
        );

        let mut instructions = VecDeque::from([spl_token::instruction::transfer(
            &spl_token::id(),
            &source_token_address,
            &recipient_token_address,
            &authority_pubkey,
            &signer_pubkeys.iter().map(|s| s).collect::<Vec<_>>(),
            amount,
        )
        .map_err(|e| {
            (
                signers.clone(),
                signer_state.clone(),
                to_diag_with_ctx(format!(
                    "failed to create token transfer instruction: {}",
                    e.to_string()
                )),
            )
        })?]);

        let client = RpcClient::new(rpc_api_url);

        let do_create_account = match client.get_account(&recipient_token_address) {
            Ok(recipient_account) => recipient_account.lamports == 0,
            Err(e) => {
                if e.to_string().contains("AccountNotFound") {
                    true
                } else {
                    return Err((
                        signers.clone(),
                        signer_state.clone(),
                        to_diag_with_ctx(format!(
                            "failed to get token recipient account: {}",
                            e.to_string()
                        )),
                    ));
                }
            }
        };

        let mut is_funding_recipient = false;
        if do_create_account {
            if args.get_bool(FUND_RECIPIENT).unwrap_or(false) {
                is_funding_recipient = true;
                instructions.push_front(
                    spl_associated_token_account::instruction::create_associated_token_account(
                        &authority_pubkey,
                        &recipient,
                        &token_mint_address,
                        &spl_token::id(),
                    ),
                );
            } else {
                return Err(
                    (
                        signers.clone(),
                        signer_state.clone(),
                        to_diag_with_ctx(
                            format!("cannot transfer token because recipient is unfunded; fund the recipient account or use the `fund_recipient = true` option")
                        )
                    )
                );
            }
        }

        let mut message =
            Message::new(&instructions.into_iter().collect::<Vec<_>>(), Some(&authority_pubkey));

        message.recent_blockhash = client.get_latest_blockhash().map_err(|e| {
            (
                signers.clone(),
                signer_state.clone(),
                to_diag_with_ctx(format!("failed to retrieve latest blockhash: {}", e.to_string())),
            )
        })?;
        let transaction = Transaction::new_unsigned(message);

        let transaction_bytes = serde_json::to_vec(&transaction).unwrap();

        let mut args = args.clone();
        args.insert(TRANSACTION_BYTES, SvmValue::message(transaction_bytes));

        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            RECIPIENT_TOKEN_ADDRESS,
            SvmValue::pubkey(recipient_token_address.to_bytes().to_vec()),
        );
        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            RECIPIENT_ADDRESS,
            SvmValue::pubkey(recipient.to_bytes().to_vec()),
        );
        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            SOURCE_TOKEN_ADDRESS,
            SvmValue::pubkey(source_token_address.to_bytes().to_vec()),
        );
        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            AUTHORITY_ADDRESS,
            SvmValue::pubkey(authority_pubkey.to_bytes().to_vec()),
        );
        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            TOKEN_MINT_ADDRESS,
            SvmValue::pubkey(token_mint_address.to_bytes().to_vec()),
        );
        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            IS_FUNDING_RECIPIENT,
            Value::bool(is_funding_recipient),
        );

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

        let mut args = args.clone();
        let future = async move {
            let run_signing_future = SignTransaction::run_signed_execution(
                &construct_did,
                &spec,
                &args,
                &progress_tx,
                &signers_instances,
                signers,
            );
            let (signers, signer_state, mut res_signing) = match run_signing_future {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(err) => return Err(err),
                },
                Err(err) => return Err(err),
            };

            let recipient_token_address = signer_state
                .get_scoped_value(&construct_did.to_string(), RECIPIENT_TOKEN_ADDRESS)
                .unwrap();
            let recipient_address = signer_state
                .get_scoped_value(&construct_did.to_string(), RECIPIENT_ADDRESS)
                .unwrap();
            let source_token_address = signer_state
                .get_scoped_value(&construct_did.to_string(), SOURCE_TOKEN_ADDRESS)
                .unwrap();
            let authority_address = signer_state
                .get_scoped_value(&construct_did.to_string(), AUTHORITY_ADDRESS)
                .unwrap();
            let token_mint_address = signer_state
                .get_scoped_value(&construct_did.to_string(), TOKEN_MINT_ADDRESS)
                .unwrap();
            let is_funding_recipient = signer_state
                .get_scoped_value(&construct_did.to_string(), IS_FUNDING_RECIPIENT)
                .unwrap();

            res_signing
                .outputs
                .insert(RECIPIENT_TOKEN_ADDRESS.into(), recipient_token_address.clone());
            res_signing.outputs.insert(RECIPIENT_ADDRESS.into(), recipient_address.clone());
            res_signing.outputs.insert(SOURCE_TOKEN_ADDRESS.into(), source_token_address.clone());
            res_signing.outputs.insert(AUTHORITY_ADDRESS.into(), authority_address.clone());
            res_signing.outputs.insert(TOKEN_MINT_ADDRESS.into(), token_mint_address.clone());
            res_signing.outputs.insert(IS_FUNDING_RECIPIENT.into(), is_funding_recipient.clone());

            let transaction_bytes = res_signing.outputs.get(SIGNED_TRANSACTION_BYTES).unwrap();
            args.insert(SIGNED_TRANSACTION_BYTES, transaction_bytes.clone());
            let transaction_bytes = transaction_bytes
                .expect_buffer_bytes_result()
                .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e)))?;
            let transaction: Transaction =
                serde_json::from_slice(&transaction_bytes).map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("failed to serialize transaction bytes: {}", e),
                    )
                })?;

            let _ = transaction.verify_and_hash_message().map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("failed to verify transaction message: {}", e),
                )
            })?;
            Ok((signers, signer_state, res_signing))
        };
        Ok(Box::pin(future))
    }

    fn build_background_task(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        values: &ValueStore,
        outputs: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        let mut status_updater =
            StatusUpdater::new(&background_tasks_uuid, &construct_did, &progress_tx);
        let recipient_token_address =
            SvmValue::expect_pubkey(outputs.get_expected_value(RECIPIENT_TOKEN_ADDRESS).unwrap());
        let recipient_address =
            SvmValue::expect_pubkey(outputs.get_expected_value(RECIPIENT_ADDRESS).unwrap());
        let source_token_address =
            SvmValue::expect_pubkey(outputs.get_expected_value(SOURCE_TOKEN_ADDRESS).unwrap());
        let authority_address =
            SvmValue::expect_pubkey(outputs.get_expected_value(AUTHORITY_ADDRESS).unwrap());
        let token_mint_address =
            SvmValue::expect_pubkey(outputs.get_expected_value(TOKEN_MINT_ADDRESS).unwrap());
        let is_funding_recipient = outputs.get_bool(IS_FUNDING_RECIPIENT).unwrap_or(false);

        status_updater.propagate_info(&format!("Transferring token {}", token_mint_address));
        status_updater.propagate_info(&format!(
            "Authority {} generated source token account {}",
            authority_address, source_token_address
        ));
        status_updater.propagate_info(&format!(
            "Recipient {} generated recipient token account {}",
            recipient_address, recipient_token_address
        ));
        if is_funding_recipient {
            status_updater.propagate_info(&format!(
                "Authority {} will fund recipient token account {}",
                authority_address, recipient_token_address
            ));
        }

        SendTransaction::build_background_task(
            &construct_did,
            &spec,
            &values,
            &outputs,
            &progress_tx,
            &background_tasks_uuid,
            &supervision_context,
        )
    }
}