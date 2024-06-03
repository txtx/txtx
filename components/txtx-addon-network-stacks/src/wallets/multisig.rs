use std::collections::{BTreeMap, HashMap, VecDeque};
use std::future::{self, Future};
use std::pin::Pin;
use std::str::FromStr;

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

    fn check_activability(
        uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &WalletSpecification,
        args: &ValueStore,
        wallet_state: ValueStore,
        mut wallets: WalletsState,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        execution_context: &CommandExecutionContext,
        is_balance_check_required: bool,
        is_public_key_required: bool,
    ) -> WalletActivabilityFutureResult {
        // Loop over the signers
        // Ensuring that they are all correctly activable.
        // When they are, collect the public keys
        // and build the stacks address + Check the balance

        println!("1=> {:?}", _spec.inputs);
        println!("2=> {:?}", args);
        println!("3=> {:?}", wallets);

        let root_uuid = uuid.clone();
        let signers_uuid = args.get_expected_array("signers").unwrap();
        let mut signers = VecDeque::new();
        for signer_uuid in signers_uuid.iter() {
            let uuid = signer_uuid.as_string().unwrap();
            let uuid = ConstructUuid::from_uuid(&Uuid::from_str(uuid).unwrap());
            let wallet_spec = wallets_instances.get(&uuid).unwrap().clone();
            signers.push_back((uuid, wallet_spec));
        }

        let args = args.clone();
        let wallets_instances = wallets_instances.clone();
        let defaults = defaults.clone();
        let execution_context = execution_context.clone();

        let future = async move {
            let mut consolidated_actions = Actions::none();

            let modal =
                BlockEvent::new_modal("Stacks Multisig Configuration assistant", "", vec![]);
            let open_modal_action = ActionItemRequest::new(
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
            );
            consolidated_actions.push_sub_group(vec![open_modal_action]);
            consolidated_actions.push_modal(modal);

            // Modal configuration
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
                consolidated_actions.append(&mut actions);
            }

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
