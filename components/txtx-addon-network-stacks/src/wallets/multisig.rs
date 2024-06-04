use std::collections::{BTreeMap, HashMap, VecDeque};
use std::future::{self, Future};
use std::pin::Pin;
use std::str::FromStr;

use clarity::address::AddressHashMode;
use clarity::types::chainstate::StacksAddress;
use clarity::util::secp256k1::Secp256k1PublicKey;
use libsecp256k1::sign;
use txtx_addon_kit::types::commands::{
    CommandExecutionContext, CommandInputsEvaluationResult, CommandSpecification,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, Actions, BlockEvent, OpenModalData,
};
use txtx_addon_kit::types::wallets::{
    self, WalletActivabilityFutureResult, WalletActivateFutureResult, WalletImplementation,
    WalletInstance, WalletSignFutureResult, WalletSpecification, WalletsState,
};
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::{channel, AddonDefaults};

use crate::constants::{CHECKED_PUBLIC_KEY, NETWORK_ID, PUBLIC_KEYS, RPC_API_URL};
use crate::typing::CLARITY_BUFFER;

use super::get_addition_actions_for_address;

lazy_static! {
    pub static ref STACKS_MULTISIG: WalletSpecification = define_wallet! {
        StacksConnect => {
          name: "Stacks Multisig",
          matcher: "multisig",
          documentation: "Coming soon",
          inputs: [
            signers: {
              documentation: "Coming soon",
                typing: Type::array(Type::string()),
                optional: false,
                interpolable: true
            },
            exepcted_address: {
              documentation: "Coming soon",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            exepcted_public_key: {
                documentation: "Coming soon",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
              }
          ],
          outputs: [
              public_key: {
                documentation: "Coming soon",
                typing: Type::array(Type::buffer())
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
        // Coming soon
    "#},
      }
    };
}

pub struct StacksConnect;
impl WalletImplementation for StacksConnect {
    fn check_instantiability(
        _ctx: &WalletSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    // Loop over the signers
    // Ensuring that they are all correctly activable.
    // When they are, collect the public keys
    // and build the stacks address + Check the balance
    fn check_activability(
        uuid: &ConstructUuid,
        instance_name: &str,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut wallet_state: ValueStore,
        mut wallets: WalletsState,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        execution_context: &CommandExecutionContext,
        is_balance_check_required: bool,
        is_public_key_required: bool,
    ) -> WalletActivabilityFutureResult {
        let root_uuid = uuid.clone();
        let signers_uuid = args.get_expected_array("signers").unwrap();
        let mut signers = VecDeque::new();
        for signer_uuid in signers_uuid.iter() {
            let uuid = signer_uuid.as_string().unwrap();
            let uuid = ConstructUuid::from_uuid(&Uuid::from_str(uuid).unwrap());
            let wallet_spec = wallets_instances.get(&uuid).unwrap().clone();
            signers.push_back((uuid, wallet_spec));
        }
        let signers_count = signers.len();

        let args = args.clone();
        let wallets_instances = wallets_instances.clone();
        let defaults = defaults.clone();
        let execution_context = execution_context.clone();
        let instance_name = instance_name.to_string();
        let expected_address = None;
        let rpc_api_url = match args.get_defaulting_string(RPC_API_URL, &defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };
        let network_id = match args.get_defaulting_string(NETWORK_ID, &defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };

        let future = async move {
            let mut consolidated_actions = Actions::none();

            let modal =
                BlockEvent::new_modal("Stacks Multisig Configuration assistant", "", vec![]);
            let mut open_modal_action = vec![ActionItemRequest::new(
                &Uuid::new_v4(),
                &Some(root_uuid.value()),
                0,
                "Compute multisig address",
                "",
                ActionItemStatus::Todo,
                ActionItemRequestType::OpenModal(OpenModalData {
                    modal_uuid: modal.uuid.clone(),
                    title: "START ASSISTANT".into(),
                }),
            )];
            let mut additional_actions_res = get_addition_actions_for_address(
                &expected_address,
                &root_uuid,
                &instance_name,
                &network_id,
                &rpc_api_url,
                is_public_key_required,
                is_balance_check_required,
            )
            .await;
            match additional_actions_res {
                Ok(ref mut res) => {
                    open_modal_action.append(res);
                }
                Err(diag) => return Err((wallets, diag)),
            }

            consolidated_actions.push_sub_group(open_modal_action);
            consolidated_actions.push_modal(modal);

            // Modal configuration
            let mut checked_public_keys = HashMap::new();
            while let Some((wallet_uuid, wallet_instance)) = signers.pop_front() {
                let signer_wallet_state = wallets.pop_wallet_state(&wallet_uuid).unwrap();
                let future = (wallet_instance.specification.check_activability)(
                    &wallet_uuid,
                    &wallet_instance.name,
                    &wallet_instance.specification,
                    &args,
                    signer_wallet_state,
                    wallets,
                    &wallets_instances,
                    &defaults,
                    &execution_context,
                    false,
                    true,
                )?;
                let (updated_wallets, mut actions) = future.await?;
                wallets = updated_wallets;

                let signer_wallet_state = wallets.get_wallet_state(&wallet_uuid).unwrap();

                let Ok(checked_public_key) =
                    signer_wallet_state.get_expected_value(CHECKED_PUBLIC_KEY)
                else {
                    consolidated_actions.append(&mut actions);
                    continue;
                };

                checked_public_keys.insert(wallet_uuid, checked_public_key.clone());
            }

            if signers_count == checked_public_keys.len() {
                let mut ordered_public_keys = vec![];
                let mut ordered_parsed_public_keys = vec![];
                for (signer_uuid, _) in signers.iter() {
                    if let Some(public_key) = checked_public_keys.remove(signer_uuid) {
                        ordered_public_keys.push(public_key.clone());
                        let bytes = public_key.expect_buffer_bytes();
                        let public_key = match Secp256k1PublicKey::from_slice(&bytes) {
                            Ok(public_key) => public_key,
                            Err(e) => {
                                wallets.push_wallet_state(wallet_state);
                                return Err((wallets, diagnosed_error!("unable to parse public key {}", e.to_string())))
                            }
                        };
                        ordered_parsed_public_keys.push(public_key);
                    }
                }
                wallet_state.insert(CHECKED_PUBLIC_KEY, Value::array(ordered_public_keys));

                let version = if network_id.eq("mainnet") {
                    clarity_repl::clarity::address::C32_ADDRESS_VERSION_MAINNET_MULTISIG
                } else {
                    clarity_repl::clarity::address::C32_ADDRESS_VERSION_TESTNET_MULTISIG
                };
        
                let stx_address = StacksAddress::from_public_keys(
                    version,
                    &AddressHashMode::SerializeP2SH,
                    ordered_parsed_public_keys.len(),
                    &ordered_parsed_public_keys,
                ).map(|address| address.to_string());
                println!("===> {:?}", stx_address);

                let mut actions = Actions::none();
                actions
                    .push_status_update_construct_uuid(&root_uuid, ActionItemStatus::Success(stx_address));
                consolidated_actions = actions;
            } else {
                let validate_modal_action = ActionItemRequest::new(
                    &Uuid::new_v4(),
                    &Some(root_uuid.value()),
                    0,
                    "CONFIRM",
                    "",
                    ActionItemStatus::Todo,
                    ActionItemRequestType::ValidateModal,
                );
                consolidated_actions.push_sub_group(vec![validate_modal_action]);
            }

            wallets.push_wallet_state(wallet_state);
            Ok((wallets, consolidated_actions))
        };
        Ok(Box::pin(future))
    }

    fn activate(
        _uuid: &ConstructUuid,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        wallet_state: ValueStore,
        wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult {
        unimplemented!()
    }

    fn check_signability(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        wallet_state: ValueStore,
        wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)> {
        unimplemented!()
    }

    fn sign(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        wallet_state: ValueStore,
        wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
    ) -> WalletSignFutureResult {
        unimplemented!()
    }
}
