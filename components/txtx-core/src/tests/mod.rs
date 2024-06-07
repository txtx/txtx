use std::{collections::BTreeMap, time::Duration};

use kit::types::frontend::ProvideSignedTransactionResponse;
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

    // runbook checklist assertions
    {
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
        // assert_eq!(start_runbook.action_status, ActionItemStatus::Success(None));
        assert_eq!(start_runbook.title.to_uppercase(), "START RUNBOOK");

        // Complete start_runbook action
        let _ = action_item_events_tx.send(ActionItemResponse {
            action_item_uuid: start_runbook.uuid.clone(),
            payload: ActionItemResponseType::ValidateBlock,
        });
    }
    // Review inputs assertions
    {
        let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
            assert!(false, "unable to receive input block");
            panic!()
        };

        let inputs_panel_data = event.expect_block().panel.expect_action_panel();

        // assert_eq!(inputs_panel_data.title.to_uppercase(), "INPUT REVIEW");
        assert_eq!(inputs_panel_data.groups.len(), 1);
        assert_eq!(inputs_panel_data.groups[0].sub_groups.len(), 3);
        assert_eq!(
            inputs_panel_data.groups[0].sub_groups[0].action_items.len(),
            1
        );
        assert_eq!(
            inputs_panel_data.groups[0].sub_groups[1].action_items.len(),
            1
        );

        let input_b_action = &inputs_panel_data.groups[0].sub_groups[0].action_items[0];
        let input_a_action = &inputs_panel_data.groups[0].sub_groups[1].action_items[0];

        assert_eq!(&input_a_action.internal_key, "check_input");
        assert_eq!(&input_b_action.internal_key, "provide_input");

        let _ = action_item_events_tx.send(ActionItemResponse {
            action_item_uuid: input_a_action.uuid.clone(),
            payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                value_checked: true,
                input_name: "value".into(),
            }),
        });

        let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
            assert!(false, "unable to receive input block");
            panic!()
        };

        let BlockEvent::UpdateActionItems(updates) = event else {
            panic!("Sending ReviewedInputResponse did not trigger update")
        };
        assert_eq!(updates.len(), 1);
        assert_eq!(&updates[0].uuid, &input_a_action.uuid);

        // Should be a no-op
        let Err(_) = block_rx.recv_timeout(Duration::from_secs(2)) else {
            assert!(false, "unable to receive input block");
            panic!()
        };

        let _ = action_item_events_tx.send(ActionItemResponse {
            action_item_uuid: input_b_action.uuid.clone(),
            payload: ActionItemResponseType::ProvideInput(ProvidedInputResponse {
                updated_value: Value::uint(5),
                input_name: "default".into(),
            }),
        });

        let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
            assert!(false, "unable to receive input block");
            panic!()
        };

        let BlockEvent::UpdateActionItems(updates) = event else {
            panic!("Sending ProvidedInputResponse did not trigger update")
        };
        println!("updates after providing input {:?}", updates);
        assert_eq!(updates.len(), 1);
        assert_eq!(&updates[0].uuid, &input_b_action.uuid);

        let _ = action_item_events_tx.send(ActionItemResponse {
            action_item_uuid: input_b_action.uuid.clone(),
            payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                value_checked: true,
                input_name: "default".into(),
            }),
        });

        let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
            assert!(false, "unable to receive input block");
            panic!()
        };

        let BlockEvent::UpdateActionItems(updates) = event else {
            panic!("Sending ReviewedInputResponse did not trigger update")
        };
        assert_eq!(updates.len(), 1);
        assert_eq!(&updates[0].uuid, &input_b_action.uuid);

        let _ = action_item_events_tx.send(ActionItemResponse {
            action_item_uuid: Uuid::new_v4(),
            payload: ActionItemResponseType::ValidateBlock,
        });
    }

    // assert output review
    {
        let Ok(event) = block_rx.recv_timeout(Duration::from_secs(2)) else {
            assert!(false, "unable to receive input block");
            panic!()
        };

        let outputs_panel_data = event.expect_block().panel.expect_action_panel();
        assert_eq!(outputs_panel_data.title.to_uppercase(), "OUTPUT REVIEW");
        assert_eq!(outputs_panel_data.groups.len(), 1);
        assert_eq!(outputs_panel_data.groups[0].sub_groups.len(), 1);
        assert_eq!(
            outputs_panel_data.groups[0].sub_groups[0]
                .action_items
                .len(),
            1
        );
    }

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    event.expect_runbook_completed();
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

    let block = event.expect_block();
    println!("PANEL + {}", block);
    let action_panel_data = block.panel.expect_action_panel();

    assert_eq!(action_panel_data.title.to_uppercase(), "RUNBOOK CHECKLIST");
    assert_eq!(action_panel_data.groups.len(), 1);
    assert_eq!(action_panel_data.groups[0].sub_groups.len(), 2);
    assert_eq!(
        action_panel_data.groups[0].sub_groups[0].action_items.len(),
        3
    );
    assert_eq!(
        action_panel_data.groups[0].sub_groups[1].action_items.len(),
        1
    );

    let get_public_key = &action_panel_data.groups[0].sub_groups[0].action_items[0];
    assert_eq!(get_public_key.action_status, ActionItemStatus::Todo);
    let ActionItemRequestType::ProvidePublicKey(_request) = &get_public_key.action_type else {
        panic!("expected provide public key request");
    };

    let check_public_key = &action_panel_data.groups[0].sub_groups[0].action_items[1];
    assert_eq!(check_public_key.action_status, ActionItemStatus::Todo);
    let ActionItemRequestType::ReviewInput(_) = &check_public_key.action_type else {
        panic!("expected provide public key request");
    };

    let start_runbook = &action_panel_data.groups[0].sub_groups[1].action_items[0];
    // assert_eq!(start_runbook.action_status, ActionItemStatus::Success(None));
    assert_eq!(start_runbook.title.to_uppercase(), "START RUNBOOK");

    // Complete start_runbook action
    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: get_public_key.uuid.clone(),
        payload: ActionItemResponseType::ProvidePublicKey(ProvidePublicKeyResponse {
            public_key: "038665eaed5fc80bd01a1068f90f2e2de4c9c041f1865868169c848c0e770042e7".into(),
        }),
    });

    // Complete start_runbook action
    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let updates = event.expect_updated_action_items();
    println!("UPDATES = {:?}", updates);
    assert_eq!(updates.len(), 2);
    assert_eq!(
        updates[0].action_status.as_ref().unwrap(),
        &ActionItemStatus::Success(Some("ST12886CEM87N4TP9CGV91VWJ8FXVX57R6AG1AXS4".into()))
    );
    assert_eq!(
        updates[1].action_status.as_ref().unwrap(),
        &ActionItemStatus::Success(Some("ST12886CEM87N4TP9CGV91VWJ8FXVX57R6AG1AXS4".into()))
    );

    // Validate panel
    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: start_runbook.uuid.clone(),
        payload: ActionItemResponseType::ValidateBlock,
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let action_panel_data = event.expect_block().panel.expect_action_panel();
    // assert_eq!(action_panel_data.title, "Sign Stacks Transaction Review");
    assert_eq!(action_panel_data.groups.len(), 1);
    assert_eq!(action_panel_data.groups[0].sub_groups.len(), 2);
    assert_eq!(
        action_panel_data.groups[0].sub_groups[0].action_items.len(),
        1
    );
    let action_item_uuid = &action_panel_data.groups[0].sub_groups[0].action_items[0];

    // Validate panel
    let signed_transaction_bytes = "808000000004004484198ea20f526ac9643690ef9243fbbe94f832000000000000000000000000000000c3000182509cd88a51120bde26719ce8299779eaed0047d2253ef4b5bff19ac1559818639fa00bff96b0178870bf5352c85f1c47d6ad011838a699623b0ca64f8dd100030200000000021a000000000000000000000000000000000000000003626e730d6e616d652d726567697374657200000004020000000474657374020000000474657374020000000474657374020000000474657374";
    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: action_item_uuid.uuid.clone(),
        payload: ActionItemResponseType::ProvideSignedTransaction(
            ProvideSignedTransactionResponse {
                signed_transaction_bytes: signed_transaction_bytes.to_string(),
            },
        ),
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let updates = event.expect_updated_action_items();
    assert_eq!(updates.len(), 1);
    assert_eq!(
        updates[0].action_status.as_ref().unwrap(),
        &ActionItemStatus::Success(None)
    );

    let validate_signature = &action_panel_data.groups[0].sub_groups[1].action_items[0];

    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: validate_signature.uuid.clone(),
        payload: ActionItemResponseType::ValidateBlock,
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let outputs_panel_data = event.expect_block().panel.expect_action_panel();

    assert_eq!(outputs_panel_data.title.to_uppercase(), "OUTPUT REVIEW");
    assert_eq!(outputs_panel_data.groups.len(), 1);
    assert_eq!(outputs_panel_data.groups[0].sub_groups.len(), 1);
    assert_eq!(
        outputs_panel_data.groups[0].sub_groups[0]
            .action_items
            .len(),
        1
    );
    assert_eq!(
        outputs_panel_data.groups[0].sub_groups[0].action_items[0]
            .action_type
            .as_display_output()
            .map(|v| &v.value),
        Some(&Value::string(signed_transaction_bytes.to_string()))
    );
    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    event.expect_runbook_completed();
}

#[test]
fn test_multisig_runbook_no_env() {
    // Load Runbook abc.tx
    let wallet_tx = include_str!("./fixtures/multisig.tx");

    let mut source_tree = SourceTree::new();
    source_tree.add_source(
        "multisig.tx".into(),
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

    let modal_panel_data = event.expect_modal().panel.as_modal_panel().unwrap();

    assert_eq!(
        modal_panel_data.title.to_uppercase(),
        "STACKS MULTISIG CONFIGURATION ASSISTANT"
    );
    println!("");
    assert_eq!(modal_panel_data.groups.len(), 3);
    assert_eq!(modal_panel_data.groups[0].sub_groups.len(), 1);
    assert_eq!(modal_panel_data.groups[1].sub_groups.len(), 1);
    assert_eq!(modal_panel_data.groups[2].sub_groups.len(), 1);
    assert_eq!(
        modal_panel_data.groups[0].sub_groups[0].action_items.len(),
        1
    );
    assert_eq!(
        modal_panel_data.groups[1].sub_groups[0].action_items.len(),
        1
    );
    assert_eq!(
        modal_panel_data.groups[2].sub_groups[0].action_items.len(),
        1
    );

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive genesis block");
        panic!()
    };

    let action_panel_data = event.expect_block().panel.expect_action_panel();

    println!("action_panel_data  {:?}", action_panel_data);
    assert_eq!(action_panel_data.title.to_uppercase(), "RUNBOOK CHECKLIST");
    assert_eq!(action_panel_data.groups.len(), 1);
    assert_eq!(action_panel_data.groups[0].sub_groups.len(), 4);
    assert_eq!(
        action_panel_data.groups[0].sub_groups[0].action_items.len(),
        1
    );
    assert_eq!(
        action_panel_data.groups[0].sub_groups[1].action_items.len(),
        1
    );
    assert_eq!(
        action_panel_data.groups[0].sub_groups[2].action_items.len(),
        1
    );
    assert_eq!(
        action_panel_data.groups[0].sub_groups[3].action_items.len(),
        1
    );

    let get_public_key_alice = &modal_panel_data.groups[0].sub_groups[0].action_items[0];
    assert_eq!(get_public_key_alice.action_status, ActionItemStatus::Todo);
    let ActionItemRequestType::ProvidePublicKey(_request) = &get_public_key_alice.action_type
    else {
        panic!("expected provide public key request");
    };

    let get_public_key_bob = &modal_panel_data.groups[1].sub_groups[0].action_items[0];
    assert_eq!(get_public_key_bob.action_status, ActionItemStatus::Todo);
    let ActionItemRequestType::ProvidePublicKey(_request) = &get_public_key_bob.action_type else {
        panic!("expected provide public key request");
    };

    let start_runbook = &action_panel_data.groups[0].sub_groups[2].action_items[0];
    assert_eq!(start_runbook.action_status, ActionItemStatus::Todo);
    assert_eq!(
        start_runbook.title.to_uppercase(),
        "COMPUTE MULTISIG ADDRESS"
    );

    // Provide Alice public key
    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: get_public_key_alice.uuid.clone(),
        payload: ActionItemResponseType::ProvidePublicKey(ProvidePublicKeyResponse {
            public_key: "02c4b5eacb71a27be633ed970dcbc41c00440364bc04ba38ae4683ac24e708bf33".into(),
        }),
    });
    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };
    let updates = event.expect_updated_action_items();
    assert_eq!(updates.len(), 2);
    assert_eq!(
        updates[0].action_status.as_ref().unwrap(),
        &ActionItemStatus::Success(Some("ST2JHG361ZXG51QTKY2NQCVBPPRRE2KZB1HR05NNC".into()))
    );

    // Provide Bob public key
    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: get_public_key_bob.uuid.clone(),
        payload: ActionItemResponseType::ProvidePublicKey(ProvidePublicKeyResponse {
            public_key: "03b3e0a76b292b2c83fc0ac14ae6160d0438ebe94e14bbb5b7755153628886e08e".into(),
        }),
    });
    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };
    let updates = event.expect_updated_action_items();
    println!("{:?}", updates);
    assert_eq!(updates.len(), 2);
    assert_eq!(
        updates[0].action_status.as_ref().unwrap(),
        &ActionItemStatus::Success(Some("ST2NEB84ASENDXKYGJPQW86YXQCEFEX2ZQPG87ND".into()))
    );

    // Validate panel
    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: start_runbook.uuid.clone(),
        payload: ActionItemResponseType::ValidateBlock,
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let action_panel_data = event.expect_block().panel.expect_action_panel();
    assert_eq!(action_panel_data.title, "Sign Stacks Transaction Review");
    assert_eq!(action_panel_data.groups.len(), 1);
    assert_eq!(action_panel_data.groups[0].sub_groups.len(), 2);
    assert_eq!(
        action_panel_data.groups[0].sub_groups[0].action_items.len(),
        1
    );
    let action_item_uuid = &action_panel_data.groups[0].sub_groups[0].action_items[0];

    // Validate panel
    let signed_transaction_bytes = "808000000004004484198ea20f526ac9643690ef9243fbbe94f832000000000000000000000000000000c3000182509cd88a51120bde26719ce8299779eaed0047d2253ef4b5bff19ac1559818639fa00bff96b0178870bf5352c85f1c47d6ad011838a699623b0ca64f8dd100030200000000021a000000000000000000000000000000000000000003626e730d6e616d652d726567697374657200000004020000000474657374020000000474657374020000000474657374020000000474657374";
    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: action_item_uuid.uuid.clone(),
        payload: ActionItemResponseType::ProvideSignedTransaction(
            ProvideSignedTransactionResponse {
                signed_transaction_bytes: signed_transaction_bytes.to_string(),
            },
        ),
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let updates = event.expect_updated_action_items();
    assert_eq!(updates.len(), 1);
    assert_eq!(
        updates[0].action_status.as_ref().unwrap(),
        &ActionItemStatus::Success(None)
    );

    let validate_signature = &action_panel_data.groups[0].sub_groups[1].action_items[0];

    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: validate_signature.uuid.clone(),
        payload: ActionItemResponseType::ValidateBlock,
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let outputs_panel_data = event.expect_block().panel.expect_action_panel();

    assert_eq!(outputs_panel_data.title.to_uppercase(), "OUTPUT REVIEW");
    assert_eq!(outputs_panel_data.groups.len(), 1);
    assert_eq!(outputs_panel_data.groups[0].sub_groups.len(), 1);
    assert_eq!(
        outputs_panel_data.groups[0].sub_groups[0]
            .action_items
            .len(),
        1
    );
    assert_eq!(
        outputs_panel_data.groups[0].sub_groups[0].action_items[0]
            .action_type
            .as_display_output()
            .map(|v| &v.value),
        Some(&Value::string(signed_transaction_bytes.to_string()))
    );
}

#[test]
fn test_bns_runbook_no_env() {
    // Load Runbook abc.tx
    let wallet_tx = include_str!("./fixtures/wallet.tx");

    let mut source_tree = SourceTree::new();
    source_tree.add_source(
        "bns.tx".into(),
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

    println!("{}", event.expect_block());

    assert_eq!(action_panel_data.title.to_uppercase(), "RUNBOOK CHECKLIST");
    assert_eq!(action_panel_data.groups.len(), 2);
    assert_eq!(action_panel_data.groups[0].sub_groups.len(), 1);
    assert_eq!(
        action_panel_data.groups[0].sub_groups[0].action_items.len(),
        3
    );
    assert_eq!(action_panel_data.groups[1].sub_groups.len(), 1);
    assert_eq!(
        action_panel_data.groups[1].sub_groups[0].action_items.len(),
        1
    );

    let get_public_key = &action_panel_data.groups[0].sub_groups[0].action_items[0];
    assert_eq!(get_public_key.action_status, ActionItemStatus::Todo);
    let ActionItemRequestType::ProvidePublicKey(_request) = &get_public_key.action_type else {
        panic!("expected provide public key request");
    };

    let check_public_key = &action_panel_data.groups[0].sub_groups[0].action_items[1];
    assert_eq!(check_public_key.action_status, ActionItemStatus::Todo);
    let ActionItemRequestType::ReviewInput(_) = &check_public_key.action_type else {
        panic!("expected provide public key request");
    };

    let start_runbook = &action_panel_data.groups[1].sub_groups[0].action_items[0];
    assert_eq!(start_runbook.action_status, ActionItemStatus::Success(None));
    assert_eq!(start_runbook.title.to_uppercase(), "START RUNBOOK");

    // Complete start_runbook action
    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: get_public_key.uuid.clone(),
        payload: ActionItemResponseType::ProvidePublicKey(ProvidePublicKeyResponse {
            public_key: "038665eaed5fc80bd01a1068f90f2e2de4c9c041f1865868169c848c0e770042e7".into(),
        }),
    });

    // Complete start_runbook action
    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let updates = event.expect_updated_action_items();
    assert_eq!(updates.len(), 3);
    assert_eq!(
        updates[0].action_status.as_ref().unwrap(),
        &ActionItemStatus::Success(Some("ST12886CEM87N4TP9CGV91VWJ8FXVX57R6AG1AXS4".into()))
    );
    assert_eq!(
        updates[1].action_status.as_ref().unwrap(),
        &ActionItemStatus::Success(Some("ST12886CEM87N4TP9CGV91VWJ8FXVX57R6AG1AXS4".into()))
    );

    // Validate panel
    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: start_runbook.uuid.clone(),
        payload: ActionItemResponseType::ValidateBlock,
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let action_panel_data = event.expect_block().panel.expect_action_panel();
    println!("{:?}", action_panel_data);
    assert_eq!(action_panel_data.title, "Input Review");
    assert_eq!(action_panel_data.groups.len(), 1);
    assert_eq!(action_panel_data.groups[0].sub_groups.len(), 2);
    assert_eq!(
        action_panel_data.groups[0].sub_groups[0].action_items.len(),
        4
    );
    assert_eq!(
        action_panel_data.groups[0].sub_groups[1].action_items.len(),
        1
    );

    let action_item_uuid = &action_panel_data.groups[0].sub_groups[0].action_items[1];
    let _ = action_item_events_tx.send(ActionItemResponse {
        action_item_uuid: action_item_uuid.uuid.clone(),
        payload: ActionItemResponseType::ValidateBlock,
    });

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
        assert!(false, "unable to receive input block");
        panic!()
    };

    let action_panel_data = event.expect_block().panel.expect_action_panel();
    println!("{:?}", action_panel_data);
}

// sequenceDiagram
//     frontend->>+runloop:
//     runloop->>+wallet_evaluation: Process wallet
//     wallet_evaluation->>+alice_wallet: Compute ActionItemRequests
//     alice_wallet-->>-wallet_evaluation: ProvidePublicKey, InputReview[public_key], InputReview[balance], InputReview[costs]
//     wallet_evaluation->>+bob_wallet: Compute ActionItemRequests
//     bob_wallet-->>-wallet_evaluation: ProvidePublicKey, InputReview[public_key], InputReview[balance], InputReview[costs
//     wallet_evaluation->>+multisig_wallet: Compute ActionItemRequests
//     multisig_wallet-->>-alice_wallet: Is public key known?
//     alice_wallet->>+multisig_wallet: Hi Alice, I can hear you!
//     multisig_wallet-->>-bob_wallet: Is public key known?
//     bob_wallet->>+multisig_wallet: Hi Alice, I can hear you!
//     multisig_wallet-->>-wallet_evaluation: ProvidePublicKey, InputReview[public_key], InputReview[balance], InputReview[costs]
//     wallet_evaluation->>+runloop: Push collected ActionItemRequests
//     runloop->>+frontend:
