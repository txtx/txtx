use std::{collections::BTreeMap, time::Duration};

use txtx_addon_kit::{
    helpers::fs::FileLocation,
    hiro_system_kit,
    types::{
        frontend::{
            ActionItemRequest, ActionItemRequestType, ActionItemResponse, ActionItemResponseType,
            ActionItemStatus, BlockEvent, ProvidePublicKeyResponse, ProvidedInputResponse,
            ReviewedInputResponse,
        },
        types::Value,
    },
    uuid::Uuid,
};
use txtx_addon_network_stacks::StacksNetworkAddon;

use crate::{
    pre_compute_runbook, start_runbook_runloop,
    std::StdAddon,
    types::{Runbook, RuntimeContext, SourceTree},
    AddonsContext,
};

#[test]
fn test_ab_c_runbook_no_env() {
    // Load Runbook abc.tx
    let abc_tx = include_str!("./fixtures/ab_c.tx");

    let mut source_tree = SourceTree::new();
    source_tree.add_source(
        "ab_c.tx".into(),
        FileLocation::from_path_string(".").unwrap(),
        abc_tx.into(),
    );

    let environments = BTreeMap::new();
    let mut addons_ctx = AddonsContext::new();
    addons_ctx.register(Box::new(StdAddon::new()));
    // addons_ctx.register(Box::new(StacksNetworkAddon::new()));

    let mut runtime_context = RuntimeContext::new(addons_ctx, environments.clone());
    let mut runbook = Runbook::new(Some(source_tree), None);

    let _ = pre_compute_runbook(&mut runbook, &mut runtime_context)
        .expect("unable to pre-compute runbook");

    let (block_tx, block_rx) = txtx_addon_kit::channel::unbounded::<BlockEvent>();
    let (action_item_updates_tx, _action_item_updates_rx) =
        txtx_addon_kit::channel::unbounded::<ActionItemRequest>();
    let (action_item_events_tx, action_item_events_rx) =
        txtx_addon_kit::channel::unbounded::<ActionItemResponse>();

    let interactive_by_default = true;

    let _ = hiro_system_kit::thread_named("Runbook Runloop").spawn(move || {
        let runloop_future = start_runbook_runloop(
            &mut runbook,
            &mut runtime_context,
            block_tx,
            action_item_updates_tx,
            action_item_events_rx,
            environments,
            interactive_by_default,
        );
        if let Err(diags) = hiro_system_kit::nestable_block_on(runloop_future) {
            for diag in diags.iter() {
                println!("{}", diag);
            }
        }
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(2)) else {
        assert!(false, "unable to receive genesis block");
        panic!()
    };

    let action_panel_data = event.expect_block().panel.expect_action_panel();
    assert_eq!(action_panel_data.title.to_uppercase(), "RUNBOOK CHECKLIST");
    assert_eq!(action_panel_data.groups.len(), 1);
    assert_eq!(action_panel_data.groups[0].sub_groups.len(), 1);
    assert_eq!(
        action_panel_data.groups[0].sub_groups[0].action_items.len(),
        1
    );

    let start_runbook = &action_panel_data.groups[0].sub_groups[0].action_items[0];
    assert_eq!(start_runbook.action_status, ActionItemStatus::Success);
    assert_eq!(start_runbook.title.to_uppercase(), "START RUNBOOK");

    // Complete start_runbook action
    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: start_runbook.uuid.clone(),
        payload: ActionItemResponseType::ValidatePanel,
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let inputs_panel_data = event.expect_block().panel.expect_action_panel();
    assert_eq!(inputs_panel_data.title.to_uppercase(), "INPUTS REVIEW");
    assert_eq!(inputs_panel_data.groups.len(), 1);
    assert_eq!(inputs_panel_data.groups[0].sub_groups.len(), 2);
    assert_eq!(
        inputs_panel_data.groups[0].sub_groups[0].action_items.len(),
        2
    );
    let input_a_uuid = &inputs_panel_data.groups[0].sub_groups[0].action_items[0];
    let input_b_uuid = &inputs_panel_data.groups[0].sub_groups[0].action_items[1];

    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: input_a_uuid.uuid.clone(),
        payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
            value_checked: true,
            input_name: "value".into(),
        }),
    });

    // Should be a no-op
    let Err(_) = block_rx.recv_timeout(Duration::from_secs(2)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: input_b_uuid.uuid.clone(),
        payload: ActionItemResponseType::ProvideInput(ProvidedInputResponse {
            updated_value: Value::uint(5),
            input_name: "value".into(),
        }),
    });

    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: Uuid::new_v4(),
        payload: ActionItemResponseType::ValidatePanel,
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(2)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let outputs_panel_data = event.expect_block().panel.expect_action_panel();
    assert_eq!(outputs_panel_data.title.to_uppercase(), "OUTPUTS REVIEW");
    assert_eq!(outputs_panel_data.groups.len(), 1);
    assert_eq!(outputs_panel_data.groups[0].sub_groups.len(), 1);
    assert_eq!(
        outputs_panel_data.groups[0].sub_groups[0]
            .action_items
            .len(),
        1
    );
}

#[test]
fn test_wallet_runbook_no_env() {
    // Load Runbook abc.tx
    let wallet_tx = include_str!("./fixtures/wallet.tx");

    let mut source_tree = SourceTree::new();
    source_tree.add_source(
        "wallet.tx".into(),
        FileLocation::from_path_string(".").unwrap(),
        wallet_tx.into(),
    );

    let environments = BTreeMap::new();
    let mut addons_ctx = AddonsContext::new();
    addons_ctx.register(Box::new(StdAddon::new()));
    addons_ctx.register(Box::new(StacksNetworkAddon::new()));

    let mut runtime_context = RuntimeContext::new(addons_ctx, environments.clone());
    let mut runbook = Runbook::new(Some(source_tree), None);

    let _ = pre_compute_runbook(&mut runbook, &mut runtime_context)
        .expect("unable to pre-compute runbook");

    let (block_tx, block_rx) = txtx_addon_kit::channel::unbounded::<BlockEvent>();
    let (action_item_updates_tx, _action_item_updates_rx) =
        txtx_addon_kit::channel::unbounded::<ActionItemRequest>();
    let (action_item_events_tx, action_item_events_rx) =
        txtx_addon_kit::channel::unbounded::<ActionItemResponse>();

    let interactive_by_default = true;

    let _ = hiro_system_kit::thread_named("Runbook Runloop").spawn(move || {
        let runloop_future = start_runbook_runloop(
            &mut runbook,
            &mut runtime_context,
            block_tx,
            action_item_updates_tx,
            action_item_events_rx,
            environments,
            interactive_by_default,
        );
        if let Err(diags) = hiro_system_kit::nestable_block_on(runloop_future) {
            for diag in diags.iter() {
                println!("{}", diag);
            }
        }
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive genesis block");
        panic!()
    };

    let action_panel_data = event.expect_block().panel.expect_action_panel();
    assert_eq!(action_panel_data.title.to_uppercase(), "RUNBOOK CHECKLIST");
    assert_eq!(action_panel_data.groups.len(), 2);
    assert_eq!(action_panel_data.groups[0].sub_groups.len(), 1);
    assert_eq!(
        action_panel_data.groups[0].sub_groups[0].action_items.len(),
        2
    );
    assert_eq!(action_panel_data.groups[1].sub_groups.len(), 1);
    assert_eq!(
        action_panel_data.groups[1].sub_groups[0].action_items.len(),
        1
    );

    let get_public_key = &action_panel_data.groups[0].sub_groups[0].action_items[0];
    assert_eq!(get_public_key.action_status, ActionItemStatus::Todo);
    let ActionItemRequestType::ProvidePublicKey(request) = &get_public_key.action_type else {
        panic!("expected provide public key request");
    };

    let check_public_key = &action_panel_data.groups[0].sub_groups[0].action_items[1];
    assert_eq!(check_public_key.action_status, ActionItemStatus::Todo);
    let ActionItemRequestType::ReviewInput = &check_public_key.action_type else {
        panic!("expected provide public key request");
    };


    // Complete start_runbook action
    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: get_public_key.uuid.clone(),
        payload: ActionItemResponseType::ProvidePublicKey(ProvidePublicKeyResponse {
            public_key: "test".into(),
        }),
    });

    let start_runbook = &action_panel_data.groups[1].sub_groups[0].action_items[0];
    assert_eq!(start_runbook.action_status, ActionItemStatus::Success);
    assert_eq!(start_runbook.title.to_uppercase(), "START RUNBOOK");

    // Complete start_runbook action
    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: start_runbook.uuid.clone(),
        payload: ActionItemResponseType::ValidatePanel,
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let inputs_panel_data = event.expect_block().panel.expect_action_panel();
    assert_eq!(inputs_panel_data.title.to_uppercase(), "INPUTS REVIEW");
    assert_eq!(inputs_panel_data.groups.len(), 1);
    assert_eq!(inputs_panel_data.groups[0].sub_groups.len(), 2);
    assert_eq!(
        inputs_panel_data.groups[0].sub_groups[0].action_items.len(),
        2
    );
    let input_a_uuid = &inputs_panel_data.groups[0].sub_groups[0].action_items[0];
    let input_b_uuid = &inputs_panel_data.groups[0].sub_groups[0].action_items[1];

    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: input_a_uuid.uuid.clone(),
        payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
            value_checked: true,
            input_name: "value".into(),
        }),
    });

    // Should be a no-op
    let Err(_) = block_rx.recv_timeout(Duration::from_secs(2)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: input_b_uuid.uuid.clone(),
        payload: ActionItemResponseType::ProvideInput(ProvidedInputResponse {
            updated_value: Value::uint(5),
            input_name: "value".into(),
        }),
    });

    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: Uuid::new_v4(),
        payload: ActionItemResponseType::ValidatePanel,
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(2)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let outputs_panel_data = event.expect_block().panel.expect_action_panel();
    assert_eq!(outputs_panel_data.title.to_uppercase(), "OUTPUTS REVIEW");
    assert_eq!(outputs_panel_data.groups.len(), 1);
    assert_eq!(outputs_panel_data.groups[0].sub_groups.len(), 1);
    assert_eq!(
        outputs_panel_data.groups[0].sub_groups[0]
            .action_items
            .len(),
        1
    );
}
