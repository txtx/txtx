use std::time::Duration;

use kit::{
    channel::{Receiver, Sender},
    types::{
        block_id::BlockId,
        frontend::{
            ActionPanelData, ModalPanelData, NormalizedActionItemRequestUpdate,
            ProvideSignedTransactionResponse,
        },
        AuthorizationContext, RunbookId,
    },
    Addon,
};
use txtx_addon_kit::{
    helpers::fs::FileLocation,
    types::{
        frontend::{
            ActionItemRequest, ActionItemRequestType, ActionItemResponse, ActionItemResponseType,
            ActionItemStatus, BlockEvent, ProvidePublicKeyResponse, ProvidedInputResponse,
            ReviewedInputResponse,
        },
        types::Value,
    },
};
use txtx_addon_network_stacks::StacksNetworkAddon;

use crate::{
    runbook::RunbookInputsMap,
    start_supervised_runbook_runloop,
    types::{Runbook, RunbookSources},
};

#[allow(unused)]
struct TestHarness {
    block_tx: Sender<BlockEvent>,
    block_rx: Receiver<BlockEvent>,
    action_item_updates_tx: Sender<ActionItemRequest>,
    action_item_events_tx: Sender<ActionItemResponse>,
    action_item_events_rx: Receiver<ActionItemResponse>,
}

#[allow(unused)]
impl TestHarness {
    fn send(&self, response: &ActionItemResponse) {
        let _ = self.action_item_events_tx.send(response.clone());
    }

    fn send_and_expect_action_item_update(
        &self,
        response: ActionItemResponse,
        expected_updates: Vec<(&BlockId, Option<ActionItemStatus>)>,
    ) -> Vec<NormalizedActionItemRequestUpdate> {
        self.send(&response);
        self.expect_action_item_update(Some(response), expected_updates)
    }

    fn expect_action_item_update(
        &self,
        response: Option<ActionItemResponse>,
        expected_updates: Vec<(&BlockId, Option<ActionItemStatus>)>,
    ) -> Vec<NormalizedActionItemRequestUpdate> {
        let Ok(event) = self.block_rx.recv_timeout(Duration::from_secs(5)) else {
            panic!(
                "unable to receive input block after sending action item response: {:?}",
                response
            );
        };

        let updates = event.expect_updated_action_items();
        let ctx = format!(
            "\n=> action item response: {:?}\n=> expected updates: {:?}\n=> actual updates: {:?}",
            response, expected_updates, updates
        );
        assert_eq!(updates.len(), expected_updates.len(), "{}", ctx);
        updates.iter().enumerate().for_each(|(i, u)| {
            assert_eq!(expected_updates[i].0.clone(), u.id, "{}", ctx);
            assert_eq!(expected_updates[i].1.clone(), u.action_status, "{}", ctx);
        });
        updates.clone()
    }

    fn send_and_expect_action_panel(
        &self,
        response: ActionItemResponse,
        expected_title: &str,
        expected_group_lengths: Vec<Vec<usize>>,
    ) -> ActionPanelData {
        self.send(&response);
        self.expect_action_panel(Some(response), expected_title, expected_group_lengths)
    }

    fn send_and_expect_modal(
        &self,
        response: ActionItemResponse,
        expected_title: &str,
        expected_group_lengths: Vec<Vec<usize>>,
    ) -> ModalPanelData {
        self.send(&response);
        self.expect_modal(Some(response), expected_title, expected_group_lengths)
    }

    fn expect_action_panel(
        &self,
        response: Option<ActionItemResponse>,
        expected_title: &str,
        expected_group_lengths: Vec<Vec<usize>>,
    ) -> ActionPanelData {
        let Ok(event) = self.block_rx.recv_timeout(Duration::from_secs(5)) else {
            panic!(
                "unable to receive input block after sending action item response: {:?}",
                response
            );
        };

        let action_panel_data = event.expect_block().panel.expect_action_panel();

        assert_eq!(
            action_panel_data.title.to_uppercase(),
            expected_title.to_uppercase(),
            "unexpected panel title after sending action item response: {:?}",
            response
        );
        let ctx = format!(
            "=> response triggering panel: {:?}\n=> actual panel group: {:?}",
            response, action_panel_data.groups
        );
        assert_eq!(
            action_panel_data.groups.len(),
            expected_group_lengths.len(),
            "{}",
            ctx
        );
        action_panel_data
            .groups
            .iter()
            .enumerate()
            .for_each(|(i, g)| {
                let expected_sub_groups = &expected_group_lengths[i];
                assert_eq!(g.sub_groups.len(), expected_sub_groups.len(), "{}", ctx);
                g.sub_groups.iter().enumerate().for_each(|(j, s)| {
                    assert_eq!(s.action_items.len(), expected_sub_groups[j], "{}", ctx);
                })
            });
        action_panel_data.clone()
    }

    fn expect_modal(
        &self,
        response: Option<ActionItemResponse>,
        expected_title: &str,
        expected_group_lengths: Vec<Vec<usize>>,
    ) -> ModalPanelData {
        let Ok(event) = self.block_rx.recv_timeout(Duration::from_secs(5)) else {
            panic!(
                "unable to receive input block after sending action item response: {:?}",
                response
            );
        };

        let modal_panel_data = event.expect_modal().panel.as_modal_panel().unwrap();

        assert_eq!(
            modal_panel_data.title.to_uppercase(),
            expected_title.to_uppercase(),
            "unexpected panel title after sending action item response: {:?}",
            response
        );
        let ctx = format!(
            "=> response triggering panel: {:?}\n=> actual panel group: {:?}",
            response, modal_panel_data.groups
        );
        assert_eq!(
            modal_panel_data.groups.len(),
            expected_group_lengths.len(),
            "{}",
            ctx
        );
        modal_panel_data
            .groups
            .iter()
            .enumerate()
            .for_each(|(i, g)| {
                let expected_sub_groups = &expected_group_lengths[i];
                assert_eq!(g.sub_groups.len(), expected_sub_groups.len(), "{}", ctx);
                g.sub_groups.iter().enumerate().for_each(|(j, s)| {
                    assert_eq!(s.action_items.len(), expected_sub_groups[j], "{}", ctx);
                })
            });
        modal_panel_data.clone()
    }
    fn expect_noop(&self) {
        let Err(_) = self.block_rx.recv_timeout(Duration::from_secs(2)) else {
            panic!("unable to receive input block")
        };
    }

    fn expect_runbook_complete(&self) {
        let Ok(event) = self.block_rx.recv_timeout(Duration::from_secs(5)) else {
            assert!(false, "unable to receive input block");
            panic!()
        };

        event.expect_runbook_completed();
    }
}

fn setup_test(file_name: &str, fixture: &str) -> TestHarness {
    let mut runbook_sources = RunbookSources::new();
    runbook_sources.add_source(
        file_name.into(),
        FileLocation::from_path_string(".").unwrap(),
        fixture.into(),
    );
    let runbook_inputs = RunbookInputsMap::new();

    let runbook_id = RunbookId {
        org: None,
        workspace: None,
        name: "test".into(),
    };

    let available_addons: Vec<Box<dyn Addon>> = vec![Box::new(StacksNetworkAddon::new())];
    let mut runbook = Runbook::new(runbook_id, None);
    let authorization_context = AuthorizationContext::empty();
    runbook
        .build_contexts_from_sources(
            runbook_sources,
            runbook_inputs,
            authorization_context,
            available_addons,
        )
        .unwrap();

    let (block_tx, block_rx) = txtx_addon_kit::channel::unbounded::<BlockEvent>();
    let (action_item_updates_tx, _action_item_updates_rx) =
        txtx_addon_kit::channel::unbounded::<ActionItemRequest>();
    let (action_item_events_tx, action_item_events_rx) =
        txtx_addon_kit::channel::unbounded::<ActionItemResponse>();

    let harness = TestHarness {
        block_tx: block_tx.clone(),
        block_rx,
        action_item_updates_tx: action_item_updates_tx.clone(),
        action_item_events_tx: action_item_events_tx.clone(),
        action_item_events_rx: action_item_events_rx.clone(),
    };
    let _ = hiro_system_kit::thread_named("Runbook Runloop").spawn(move || {
        let runloop_future = start_supervised_runbook_runloop(
            &mut runbook,
            block_tx,
            action_item_updates_tx,
            action_item_events_rx,
        );
        if let Err(diags) = hiro_system_kit::nestable_block_on(runloop_future) {
            for diag in diags.iter() {
                println!("{}", diag);
            }
        }
    });
    harness
}

#[test]
fn test_ab_c_runbook_no_env() {
    // Load Runbook ab_c.tx
    let abc_tx = include_str!("./fixtures/ab_c.tx");

    let harness = setup_test("ab_c.tx", &abc_tx);

    // runbook checklist assertions
    {
        let action_panel_data =
            harness.expect_action_panel(None, "runbook checklist", vec![vec![1]]);

        let validate_button = &action_panel_data.groups[0].sub_groups[0].action_items[0];

        let start_runbook = &action_panel_data.groups[0].sub_groups[0].action_items[0];
        // assert_eq!(start_runbook.action_status, ActionItemStatus::Success(None));
        assert_eq!(start_runbook.title.to_uppercase(), "START RUNBOOK");

        // Complete start_runbook action
        harness.send_and_expect_action_item_update(
            ActionItemResponse {
                action_item_id: start_runbook.id.clone(),
                payload: ActionItemResponseType::ValidateBlock,
            },
            vec![(&validate_button.id, Some(ActionItemStatus::Success(None)))],
        );
    }
    // Review inputs assertions
    {
        let inputs_panel_data =
            harness.expect_action_panel(None, "inputs review", vec![vec![1, 1, 1]]);

        let input_b_action = &inputs_panel_data.groups[0].sub_groups[0].action_items[0];
        let input_a_action = &inputs_panel_data.groups[0].sub_groups[1].action_items[0];

        assert_eq!(&input_a_action.internal_key, "check_input");
        assert_eq!(&input_b_action.internal_key, "provide_input");

        // review input a and expect action item update
        harness.send_and_expect_action_item_update(
            ActionItemResponse {
                action_item_id: input_a_action.id.clone(),
                payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                    value_checked: true,
                    input_name: "value".into(),
                }),
            },
            vec![(&input_a_action.id, Some(ActionItemStatus::Success(None)))],
        );

        // Should be a no-op
        harness.expect_noop();

        // provide input b and expect no update
        harness.send(&ActionItemResponse {
            action_item_id: input_b_action.id.clone(),
            payload: ActionItemResponseType::ProvideInput(ProvidedInputResponse {
                updated_value: Value::uint(5),
                input_name: "default".into(),
            }),
        });
        // review input b and expect action item update
        harness.send_and_expect_action_item_update(
            ActionItemResponse {
                action_item_id: input_b_action.id.clone(),
                payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                    value_checked: true,
                    input_name: "default".into(),
                }),
            },
            vec![(&input_b_action.id, Some(ActionItemStatus::Success(None)))],
        );

        // our validate block button yields another action item update for input b, but it would be filtered
        // out from being propagated to the frontend... we should probably update tests to check this
        harness.send_and_expect_action_item_update(
            ActionItemResponse {
                action_item_id: BlockId::new(&vec![]),
                payload: ActionItemResponseType::ValidateBlock,
            },
            vec![(&input_b_action.id, Some(ActionItemStatus::Success(None)))],
        );
    }

    // assert output review
    {
        harness.expect_action_panel(None, "output review", vec![vec![1]]);
    }

    harness.expect_runbook_complete();
}

#[test]
fn test_wallet_runbook_no_env() {
    // Load Runbook wallet.tx
    let wallet_tx = include_str!("./fixtures/wallet.tx");

    let harness = setup_test("wallet.tx", wallet_tx);

    let action_panel_data =
        harness.expect_action_panel(None, "runbook checklist", vec![vec![3, 1]]);

    let get_public_key = &action_panel_data.groups[0].sub_groups[0].action_items[0];
    let confirm_address = &action_panel_data.groups[0].sub_groups[0].action_items[1];
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
    assert_eq!(start_runbook.title.to_uppercase(), "START RUNBOOK");

    // Complete start_runbook action
    let _ = harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: get_public_key.id.clone(),
            payload: ActionItemResponseType::ProvidePublicKey(ProvidePublicKeyResponse {
                public_key: "038665eaed5fc80bd01a1068f90f2e2de4c9c041f1865868169c848c0e770042e7"
                    .into(),
            }),
        },
        vec![
            (
                &confirm_address.id,
                Some(ActionItemStatus::Success(Some(
                    "ST12886CEM87N4TP9CGV91VWJ8FXVX57R6AG1AXS4".into(),
                ))),
            ),
            (
                &get_public_key.id,
                Some(ActionItemStatus::Success(Some(
                    "ST12886CEM87N4TP9CGV91VWJ8FXVX57R6AG1AXS4".into(),
                ))),
            ),
        ],
    );

    // Validate panel
    harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: start_runbook.id.clone(),
            payload: ActionItemResponseType::ValidateBlock,
        },
        vec![(&start_runbook.id, Some(ActionItemStatus::Success(None)))],
    );

    let action_panel_data =
        harness.expect_action_panel(None, "transaction signing", vec![vec![3, 1]]);

    let nonce_action = &action_panel_data.groups[0].sub_groups[0].action_items[0];
    let fee_action = &action_panel_data.groups[0].sub_groups[0].action_items[1];
    let provide_signature_action = &action_panel_data.groups[0].sub_groups[0].action_items[2];
    assert_eq!(
        Some("Check account nonce".to_string()),
        nonce_action.description
    );
    assert_eq!(
        Some("Check transaction fee".to_string()),
        fee_action.description
    );
    let signed_transaction_bytes = "808000000004004484198ea20f526ac9643690ef9243fbbe94f832000000000000000000000000000000c3000182509cd88a51120bde26719ce8299779eaed0047d2253ef4b5bff19ac1559818639fa00bff96b0178870bf5352c85f1c47d6ad011838a699623b0ca64f8dd100030200000000021a000000000000000000000000000000000000000003626e730d6e616d652d726567697374657200000004020000000474657374020000000474657374020000000474657374020000000474657374";
    // sign tx
    {
        let _ = harness.send_and_expect_action_item_update(
            ActionItemResponse {
                action_item_id: provide_signature_action.id.clone(),
                payload: ActionItemResponseType::ProvideSignedTransaction(
                    ProvideSignedTransactionResponse {
                        signed_transaction_bytes: signed_transaction_bytes.to_string(),
                        signer_uuid: provide_signature_action
                            .action_type
                            .as_provide_signed_tx()
                            .unwrap()
                            .signer_uuid
                            .clone(),
                    },
                ),
            },
            vec![(
                &provide_signature_action.id,
                Some(ActionItemStatus::Success(None)),
            )],
        );
    }
    // validate nonce input
    {
        let _ = harness.send_and_expect_action_item_update(
            ActionItemResponse {
                action_item_id: nonce_action.id.clone(),
                payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                    input_name: nonce_action
                        .action_type
                        .as_review_input()
                        .unwrap()
                        .input_name
                        .clone(),
                    value_checked: true,
                }),
            },
            vec![(&nonce_action.id, Some(ActionItemStatus::Success(None)))],
        );
    }
    // validate fee input
    {
        let _ = harness.send_and_expect_action_item_update(
            ActionItemResponse {
                action_item_id: fee_action.id.clone(),
                payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                    input_name: fee_action
                        .action_type
                        .as_review_input()
                        .unwrap()
                        .input_name
                        .clone(),
                    value_checked: true,
                }),
            },
            vec![(&fee_action.id, Some(ActionItemStatus::Success(None)))],
        );
    }

    let validate_signature = &action_panel_data.groups[0].sub_groups[1].action_items[0];

    harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: validate_signature.id.clone(),
            payload: ActionItemResponseType::ValidateBlock,
        },
        vec![(
            &validate_signature.id,
            Some(ActionItemStatus::Success(None)),
        )],
    );

    let outputs_panel_data = harness.expect_action_panel(None, "output review", vec![vec![1]]);

    assert_eq!(
        outputs_panel_data.groups[0].sub_groups[0].action_items[0]
            .action_type
            .as_display_output()
            .map(|v| &v.value),
        Some(&Value::string(signed_transaction_bytes.to_string()))
    );

    harness.expect_runbook_complete();
}

#[test]
fn test_multisig_runbook_no_env() {
    let multisig_tx = include_str!("./fixtures/multisig.tx");
    let harness = setup_test("wallet.tx", multisig_tx);

    let modal_panel_data = harness.expect_modal(
        None,
        "STACKS MULTISIG CONFIGURATION ASSISTANT",
        vec![vec![1], vec![1], vec![1]],
    );

    let action_panel_data =
        harness.expect_action_panel(None, "runbook checklist", vec![vec![1], vec![1, 2, 1]]);

    let get_public_key_alice = &modal_panel_data.groups[0].sub_groups[0].action_items[0];
    let get_public_key_bob = &modal_panel_data.groups[1].sub_groups[0].action_items[0];
    println!("modal_panel_data: {:?}", modal_panel_data);
    // validate some data about actions to provide pub key
    {
        assert_eq!(get_public_key_alice.action_status, ActionItemStatus::Todo);
        let ActionItemRequestType::ProvidePublicKey(_request) = &get_public_key_alice.action_type
        else {
            panic!("expected provide public key request");
        };
        assert_eq!(
            &get_public_key_alice.title.to_uppercase(),
            "CONNECT WALLET ALICE"
        );

        assert_eq!(get_public_key_bob.action_status, ActionItemStatus::Todo);
        let ActionItemRequestType::ProvidePublicKey(_request) = &get_public_key_bob.action_type
        else {
            panic!("expected provide public key request");
        };
        assert_eq!(
            &get_public_key_bob.title.to_uppercase(),
            "CONNECT WALLET BOB"
        );
    }
    println!("action panel data: {:?}", action_panel_data);
    let verify_address_alice = &action_panel_data.groups[1].sub_groups[0].action_items[0];
    let verify_address_bob = &action_panel_data.groups[0].sub_groups[0].action_items[0];
    let compute_multisig = &action_panel_data.groups[1].sub_groups[1].action_items[0];
    let verify_balance = &action_panel_data.groups[1].sub_groups[1].action_items[1];
    assert_eq!(compute_multisig.action_status, ActionItemStatus::Todo);
    assert_eq!(
        compute_multisig.title.to_uppercase(),
        "COMPUTE MULTISIG ADDRESS"
    );

    harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: get_public_key_alice.id.clone(),
            payload: ActionItemResponseType::ProvidePublicKey(ProvidePublicKeyResponse {
                public_key: "02c4b5eacb71a27be633ed970dcbc41c00440364bc04ba38ae4683ac24e708bf33"
                    .into(),
            }),
        },
        vec![
            (
                &verify_address_alice.id,
                Some(ActionItemStatus::Success(Some(
                    "ST2JHG361ZXG51QTKY2NQCVBPPRRE2KZB1HR05NNC".into(),
                ))),
            ),
            (
                &get_public_key_alice.id,
                Some(ActionItemStatus::Success(Some(
                    "ST2JHG361ZXG51QTKY2NQCVBPPRRE2KZB1HR05NNC".into(),
                ))),
            ),
        ],
    );

    harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: get_public_key_bob.id.clone(),
            payload: ActionItemResponseType::ProvidePublicKey(ProvidePublicKeyResponse {
                public_key: "03b3e0a76b292b2c83fc0ac14ae6160d0438ebe94e14bbb5b7755153628886e08e"
                    .into(),
            }),
        },
        vec![
            (
                &verify_address_bob.id,
                Some(ActionItemStatus::Success(Some(
                    "ST2NEB84ASENDXKYGJPQW86YXQCEFEX2ZQPG87ND".into(),
                ))),
            ),
            (
                &get_public_key_bob.id,
                Some(ActionItemStatus::Success(Some(
                    "ST2NEB84ASENDXKYGJPQW86YXQCEFEX2ZQPG87ND".into(),
                ))),
            ),
            (
                &verify_address_alice.id,
                Some(ActionItemStatus::Success(Some(
                    "ST2JHG361ZXG51QTKY2NQCVBPPRRE2KZB1HR05NNC".into(),
                ))),
            ),
            (
                &get_public_key_alice.id,
                Some(ActionItemStatus::Success(Some(
                    "ST2JHG361ZXG51QTKY2NQCVBPPRRE2KZB1HR05NNC".into(),
                ))),
            ),
            (&verify_balance.id, Some(ActionItemStatus::Success(None))),
            (
                &compute_multisig.id,
                Some(ActionItemStatus::Success(Some(
                    "SN263VV5AHS55QV94FB70W6DJNPET8SWF5WRK5S1K".into(),
                ))),
            ),
        ],
    );

    let sign_tx_modal = harness.send_and_expect_modal(
        ActionItemResponse {
            action_item_id: compute_multisig.id.clone(),
            payload: ActionItemResponseType::ValidateBlock,
        },
        "Stacks Multisig Signing Assistant",
        vec![vec![1, 1], vec![1]],
    );

    let sign_tx_alice = &sign_tx_modal.groups[0].sub_groups[0].action_items[0];
    let sign_tx_bob = &sign_tx_modal.groups[0].sub_groups[1].action_items[0];

    // I don't know why this update is sent here, this feels extraneous
    harness.expect_action_item_update(
        None,
        vec![(&compute_multisig.id, Some(ActionItemStatus::Success(None)))],
    );

    let action_panel_data =
        harness.expect_action_panel(None, "TRANSACTION SIGNING", vec![vec![2, 1, 1]]);

    let nonce_action = &action_panel_data.groups[0].sub_groups[0].action_items[0];
    let fee_action = &action_panel_data.groups[0].sub_groups[0].action_items[1];
    let compute_multisig = &action_panel_data.groups[0].sub_groups[1].action_items[0];
    let validate_signature = &action_panel_data.groups[0].sub_groups[2].action_items[0];

    // alice signature
    let signed_transaction_bytes = "808000000004018c3decaa8e4a5bed247ace0e19b2ad9da4678f2f000000000000000000000000000000c30000000102014511a3f97d09ec94db5f7ebee6f8fe62b5400ce1ba97c39e68acda4493e6c57572e752a2bcfacf3d2c6dc6cd18b5ede7c9913eeb6729e1a00b312f950b9e8f4a0002030100000000021a000000000000000000000000000000000000000003626e730d6e616d652d726567697374657200000004020000000474657374020000000474657374020000000474657374020000000474657374";
    harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: sign_tx_alice.id.clone(),
            payload: ActionItemResponseType::ProvideSignedTransaction(
                ProvideSignedTransactionResponse {
                    signed_transaction_bytes: signed_transaction_bytes.to_string(),
                    signer_uuid: sign_tx_alice
                        .action_type
                        .as_provide_signed_tx()
                        .unwrap()
                        .signer_uuid
                        .clone(),
                },
            ),
        },
        vec![
            (&sign_tx_alice.id, Some(ActionItemStatus::Success(None))),
            (&sign_tx_bob.id, Some(ActionItemStatus::Todo)),
        ],
    );

    // bob signature
    let signed_transaction_bytes = "808000000004018c3decaa8e4a5bed247ace0e19b2ad9da4678f2f000000000000000000000000000000c30000000202014511a3f97d09ec94db5f7ebee6f8fe62b5400ce1ba97c39e68acda4493e6c57572e752a2bcfacf3d2c6dc6cd18b5ede7c9913eeb6729e1a00b312f950b9e8f4a0201f9f471b80dc111b4e33632335002a5e26ac4369899da7545273c883d26bdd28356e29821259e4ced4d65f0d833a12860b4a0844858b14df6f39ece49c05f75f30002030100000000021a000000000000000000000000000000000000000003626e730d6e616d652d726567697374657200000004020000000474657374020000000474657374020000000474657374020000000474657374";
    harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: sign_tx_bob.id.clone(),
            payload: ActionItemResponseType::ProvideSignedTransaction(
                ProvideSignedTransactionResponse {
                    signed_transaction_bytes: signed_transaction_bytes.to_string(),
                    signer_uuid: sign_tx_bob
                        .action_type
                        .as_provide_signed_tx()
                        .unwrap()
                        .signer_uuid
                        .clone(),
                },
            ),
        },
        vec![
            (&sign_tx_alice.id, Some(ActionItemStatus::Success(None))),
            (&sign_tx_bob.id, Some(ActionItemStatus::Success(None))),
            (
                &compute_multisig.id,
                Some(ActionItemStatus::Success(Some(
                    "All signers participated".to_string(),
                ))),
            ),
        ],
    );

    // validate nonce
    harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: nonce_action.id.clone(),
            payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                input_name: nonce_action
                    .action_type
                    .as_review_input()
                    .unwrap()
                    .input_name
                    .clone(),
                value_checked: true,
            }),
        },
        vec![(&nonce_action.id, Some(ActionItemStatus::Success(None)))],
    );

    // validate fee
    harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: fee_action.id.clone(),
            payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                input_name: fee_action
                    .action_type
                    .as_review_input()
                    .unwrap()
                    .input_name
                    .clone(),
                value_checked: true,
            }),
        },
        vec![(&fee_action.id, Some(ActionItemStatus::Success(None)))],
    );

    // validate signature block
    harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: validate_signature.id.clone(),
            payload: ActionItemResponseType::ValidateBlock,
        },
        vec![(
            &validate_signature.id,
            Some(ActionItemStatus::Success(None)),
        )],
    );

    let outputs_panel_data = harness.expect_action_panel(None, "output review", vec![vec![1]]);

    assert_eq!(
        outputs_panel_data.groups[0].sub_groups[0].action_items[0]
            .action_type
            .as_display_output()
            .map(|v| &v.value),
        Some(&Value::string(signed_transaction_bytes.to_string()))
    );

    harness.expect_runbook_complete();
}

#[ignore]
#[test]
fn test_bns_runbook_no_env() {
    // Load Runbook abc.tx
    let wallet_tx = include_str!("./fixtures/wallet.tx");

    let mut runbook_sources = RunbookSources::new();
    runbook_sources.add_source(
        "bns.tx".into(),
        FileLocation::from_path_string(".").unwrap(),
        wallet_tx.into(),
    );
    let runbook_inputs = RunbookInputsMap::new();

    let runbook_id = RunbookId {
        org: None,
        workspace: None,
        name: "test".into(),
    };

    let mut runbook = Runbook::new(runbook_id, None);
    let available_addons: Vec<Box<dyn Addon>> = vec![Box::new(StacksNetworkAddon::new())];
    let authorization_context = AuthorizationContext::empty();
    runbook
        .build_contexts_from_sources(
            runbook_sources,
            runbook_inputs,
            authorization_context,
            available_addons,
        )
        .unwrap();

    let (block_tx, block_rx) = txtx_addon_kit::channel::unbounded::<BlockEvent>();
    let (action_item_updates_tx, _action_item_updates_rx) =
        txtx_addon_kit::channel::unbounded::<ActionItemRequest>();
    let (action_item_events_tx, action_item_events_rx) =
        txtx_addon_kit::channel::unbounded::<ActionItemResponse>();

    let _ = hiro_system_kit::thread_named("Runbook Runloop").spawn(move || {
        let runloop_future = start_supervised_runbook_runloop(
            &mut runbook,
            block_tx,
            action_item_updates_tx,
            action_item_events_rx,
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
        action_item_id: get_public_key.id.clone(),
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
        action_item_id: start_runbook.id.clone(),
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
        action_item_id: action_item_uuid.id.clone(),
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
