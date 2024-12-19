use txtx_addon_kit::{
    types::{
        frontend::{
            ActionItemRequestType, ActionItemResponse, ActionItemResponseType, ActionItemStatus,
            ProvidePublicKeyResponse, ProvideSignedTransactionResponse, ReviewedInputResponse,
        },
        types::Value,
    },
    Addon,
};
use txtx_test_utils::{test_harness::setup_test, StdAddon};

use crate::StacksNetworkAddon;

pub fn get_addon_by_namespace(namespace: &str) -> Option<Box<dyn Addon>> {
    let available_addons: Vec<Box<dyn Addon>> =
        vec![Box::new(StdAddon::new()), Box::new(StacksNetworkAddon::new())];
    for addon in available_addons.into_iter() {
        if namespace.starts_with(&format!("{}", addon.get_namespace())) {
            return Some(addon);
        }
    }
    None
}

#[test]
fn test_signer_runbook_no_env() {
    // Load Runbook signer.tx
    let var_name = include_str!("./fixtures/signer.tx");
    let signer_tx = var_name;
    let harness = setup_test("signer.tx", signer_tx, get_addon_by_namespace);

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

    harness.assert_provide_signature_formatted_payload(provide_signature_action, Some("{\n  \"version\": \"testnet\",\n  \"chain_id\": \"2147483648\",\n  \"payload\": {\n    \"type\": \"Contract Call\",\n    \"contract_address\": \"ST000000000000000000002AMW42H\",\n    \"contract_name\": \"bns\",\n    \"function_name\": \"name-register\",\n    \"function_args\": [\n      \"0x74657374\",\n      \"0x74657374\",\n      \"0x74657374\",\n      \"0x74657374\"\n    ]\n  },\n  \"post_condition_mode\": \"Deny\",\n  \"post_conditions\": [],\n  \"auth\": {\n    \"spending_condition\": \"singlesig\",\n    \"signer\": \"4484198ea20f526ac9643690ef9243fbbe94f832\",\n    \"nonce\": 0,\n    \"tx_fee\": 195\n  }\n}".into()));

    assert_eq!(Some("Check account nonce".to_string()), nonce_action.description);
    assert_eq!(Some("Check transaction fee".to_string()), fee_action.description);
    let signed_transaction_bytes = "808000000004004484198ea20f526ac9643690ef9243fbbe94f832000000000000000000000000000000c3000182509cd88a51120bde26719ce8299779eaed0047d2253ef4b5bff19ac1559818639fa00bff96b0178870bf5352c85f1c47d6ad011838a699623b0ca64f8dd100030200000000021a000000000000000000000000000000000000000003626e730d6e616d652d726567697374657200000004020000000474657374020000000474657374020000000474657374020000000474657374";
    // sign tx
    {
        let _ = harness.send_and_expect_action_item_update(
            ActionItemResponse {
                action_item_id: provide_signature_action.id.clone(),
                payload: ActionItemResponseType::ProvideSignedTransaction(
                    ProvideSignedTransactionResponse {
                        signed_transaction_bytes: Some(signed_transaction_bytes.to_string()),
                        signer_uuid: provide_signature_action
                            .action_type
                            .as_provide_signed_tx()
                            .unwrap()
                            .signer_uuid
                            .clone(),
                        signature_approved: None,
                    },
                ),
            },
            vec![(&provide_signature_action.id, Some(ActionItemStatus::Success(None)))],
        );
    }
    // validate nonce input
    {
        let nonce_review_input = nonce_action.action_type.as_review_input().unwrap();
        let _ = harness.send_and_expect_action_item_update(
            ActionItemResponse {
                action_item_id: nonce_action.id.clone(),
                payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                    input_name: nonce_review_input.input_name.clone(),
                    value_checked: true,
                    force_execution: nonce_review_input.force_execution,
                }),
            },
            vec![(&nonce_action.id, Some(ActionItemStatus::Success(None)))],
        );
    }
    // validate fee input
    {
        let fee_review_input = fee_action.action_type.as_review_input().unwrap();
        let _ = harness.send_and_expect_action_item_update(
            ActionItemResponse {
                action_item_id: fee_action.id.clone(),
                payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                    input_name: fee_review_input.input_name.clone(),
                    value_checked: true,
                    force_execution: fee_review_input.force_execution,
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
        vec![(&validate_signature.id, Some(ActionItemStatus::Success(None)))],
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
    let harness = setup_test("multisig.tx", multisig_tx, get_addon_by_namespace);

    let modal_panel_data = harness.expect_modal(
        None,
        "STACKS MULTISIG CONFIGURATION ASSISTANT",
        vec![vec![1], vec![1], vec![1]],
    );

    let action_panel_data =
        harness.expect_action_panel(None, "runbook checklist", vec![vec![1], vec![1, 2, 1]]);

    let get_public_key_alice = &modal_panel_data.groups[0].sub_groups[0].action_items[0];
    let get_public_key_bob = &modal_panel_data.groups[1].sub_groups[0].action_items[0];

    // validate some data about actions to provide pub key
    {
        assert_eq!(get_public_key_alice.action_status, ActionItemStatus::Todo);
        let ActionItemRequestType::ProvidePublicKey(_request) = &get_public_key_alice.action_type
        else {
            panic!("expected provide public key request");
        };
        assert_eq!(&get_public_key_alice.title.to_uppercase(), "CONNECT WALLET ALICE");

        assert_eq!(get_public_key_bob.action_status, ActionItemStatus::Todo);
        let ActionItemRequestType::ProvidePublicKey(_request) = &get_public_key_bob.action_type
        else {
            panic!("expected provide public key request");
        };
        assert_eq!(&get_public_key_bob.title.to_uppercase(), "CONNECT WALLET BOB");
    }

    let verify_address_alice = &action_panel_data.groups[1].sub_groups[0].action_items[0];
    let verify_address_bob = &action_panel_data.groups[0].sub_groups[0].action_items[0];
    let compute_multisig = &action_panel_data.groups[1].sub_groups[1].action_items[0];
    let verify_balance = &action_panel_data.groups[1].sub_groups[1].action_items[1];
    assert_eq!(compute_multisig.action_status, ActionItemStatus::Todo);
    assert_eq!(compute_multisig.title.to_uppercase(), "COMPUTED MULTISIG ADDRESS");

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
    harness.assert_provide_signature_formatted_payload(sign_tx_alice, Some("{\n  \"version\": \"testnet\",\n  \"chain_id\": \"2147483648\",\n  \"payload\": {\n    \"type\": \"Contract Call\",\n    \"contract_address\": \"ST000000000000000000002AMW42H\",\n    \"contract_name\": \"bns\",\n    \"function_name\": \"name-register\",\n    \"function_args\": [\n      \"0x74657374\",\n      \"0x74657374\",\n      \"0x74657374\",\n      \"0x74657374\"\n    ]\n  },\n  \"post_condition_mode\": \"Deny\",\n  \"post_conditions\": [],\n  \"auth\": {\n    \"spending_condition\": \"multisig\",\n    \"signer\": \"8c3decaa8e4a5bed247ace0e19b2ad9da4678f2f\",\n    \"nonce\": 0,\n    \"tx_fee\": 195,\n    \"signatures_required\": 2,\n    \"fields\": []\n  }\n}".into()));

    let sign_tx_bob = &sign_tx_modal.groups[0].sub_groups[1].action_items[0];
    harness.assert_provide_signature_formatted_payload(sign_tx_bob, Some("{\n  \"version\": \"testnet\",\n  \"chain_id\": \"2147483648\",\n  \"payload\": {\n    \"type\": \"Contract Call\",\n    \"contract_address\": \"ST000000000000000000002AMW42H\",\n    \"contract_name\": \"bns\",\n    \"function_name\": \"name-register\",\n    \"function_args\": [\n      \"0x74657374\",\n      \"0x74657374\",\n      \"0x74657374\",\n      \"0x74657374\"\n    ]\n  },\n  \"post_condition_mode\": \"Deny\",\n  \"post_conditions\": [],\n  \"auth\": {\n    \"spending_condition\": \"multisig\",\n    \"signer\": \"8c3decaa8e4a5bed247ace0e19b2ad9da4678f2f\",\n    \"nonce\": 0,\n    \"tx_fee\": 195,\n    \"signatures_required\": 2,\n    \"fields\": [\n      \"02c4b5eacb71a27be633ed970dcbc41c00440364bc04ba38ae4683ac24e708bf33\"\n    ]\n  }\n}".into()));

    // I don't know why this update is sent here, this feels extraneous
    harness.expect_action_item_update(
        None,
        vec![(&compute_multisig.id, Some(ActionItemStatus::Success(None)))],
    );

    let action_panel_data =
        harness.expect_action_panel(None, "TRANSACTION SIGNING", vec![vec![3, 1]]);

    assert_eq!(
        action_panel_data.groups[0].title,
        "Review and sign the transactions from the list below".to_string()
    );
    let nonce_action = &action_panel_data.groups[0].sub_groups[0].action_items[0];
    let fee_action = &action_panel_data.groups[0].sub_groups[0].action_items[1];
    let compute_multisig_signature = &action_panel_data.groups[0].sub_groups[0].action_items[2];
    let validate_signature = &action_panel_data.groups[0].sub_groups[1].action_items[0];

    // alice signature
    let signed_transaction_bytes = "808000000004018c3decaa8e4a5bed247ace0e19b2ad9da4678f2f000000000000000000000000000000c300000001020037489e7cde9f22a6dd9ba1012b3f98ef983ace0cf111628de2ee314206330dc32aa52a2f0389246c0839370a12c6843c8ffffca637bef78d63f45fe4a5a59fbc0002030200000000021a000000000000000000000000000000000000000003626e730d6e616d652d726567697374657200000004020000000474657374020000000474657374020000000474657374020000000474657374";
    harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: sign_tx_alice.id.clone(),
            payload: ActionItemResponseType::ProvideSignedTransaction(
                ProvideSignedTransactionResponse {
                    signed_transaction_bytes: Some(signed_transaction_bytes.to_string()),
                    signer_uuid: sign_tx_alice
                        .action_type
                        .as_provide_signed_tx()
                        .unwrap()
                        .signer_uuid
                        .clone(),
                    signature_approved: None,
                },
            ),
        },
        vec![
            (&sign_tx_alice.id, Some(ActionItemStatus::Success(None))),
            (&sign_tx_bob.id, Some(ActionItemStatus::Todo)),
        ],
    );

    // bob signature
    let signed_transaction_bytes = "808000000004018c3decaa8e4a5bed247ace0e19b2ad9da4678f2f000000000000000000000000000000c300000002020037489e7cde9f22a6dd9ba1012b3f98ef983ace0cf111628de2ee314206330dc32aa52a2f0389246c0839370a12c6843c8ffffca637bef78d63f45fe4a5a59fbc02003b4784204e1a01ea1e359862bd42d3654f3b4d72a938a1fe511f7acc91fb1e89740c469a7e39d344c5ffe7f52099517d6dedd701b152c7d804cffe49eafe10390002030200000000021a000000000000000000000000000000000000000003626e730d6e616d652d726567697374657200000004020000000474657374020000000474657374020000000474657374020000000474657374";
    harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: sign_tx_bob.id.clone(),
            payload: ActionItemResponseType::ProvideSignedTransaction(
                ProvideSignedTransactionResponse {
                    signed_transaction_bytes: Some(signed_transaction_bytes.to_string()),
                    signer_uuid: sign_tx_bob
                        .action_type
                        .as_provide_signed_tx()
                        .unwrap()
                        .signer_uuid
                        .clone(),
                    signature_approved: None,
                },
            ),
        },
        vec![
            (&sign_tx_alice.id, Some(ActionItemStatus::Success(None))),
            (&sign_tx_bob.id, Some(ActionItemStatus::Success(None))),
            (
                &compute_multisig_signature.id,
                Some(ActionItemStatus::Success(Some("All signers participated".to_string()))),
            ),
        ],
    );

    // validate nonce
    let nonce_review_input = nonce_action.action_type.as_review_input().unwrap();
    harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: nonce_action.id.clone(),
            payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                input_name: nonce_review_input.input_name.clone(),
                value_checked: true,
                force_execution: nonce_review_input.force_execution,
            }),
        },
        vec![(&nonce_action.id, Some(ActionItemStatus::Success(None)))],
    );

    // validate fee
    let fee_review_input = fee_action.action_type.as_review_input().unwrap();
    harness.send_and_expect_action_item_update(
        ActionItemResponse {
            action_item_id: fee_action.id.clone(),
            payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                input_name: fee_review_input.input_name.clone(),
                value_checked: true,
                force_execution: fee_review_input.force_execution,
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
        vec![(&validate_signature.id, Some(ActionItemStatus::Success(None)))],
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

// #[ignore]
// #[test]
// fn test_bns_runbook_no_env() {
//     // Load Runbook abc.tx
//     let signer_tx = include_str!("./fixtures/signer.tx");
//     let test_harness = setup_test(file_name, fixture, available_addons)

//     let mut runbook_sources = RunbookSources::new();
//     runbook_sources.add_source(
//         "bns.tx".into(),
//         FileLocation::from_path_string(".").unwrap(),
//         signer_tx.into(),
//     );
//     let runbook_inputs = RunbookInputsMap::new();

//     let runbook_id = RunbookId { org: None, workspace: None, name: "test".into() };

//     let mut runbook = Runbook::new(runbook_id, None);
//     let available_addons: Vec<Box<dyn Addon>> = vec![Box::new(StacksNetworkAddon::new())];
//     let authorization_context = AuthorizationContext::empty();
//     runbook
//         .build_contexts_from_sources(
//             runbook_sources,
//             runbook_inputs,
//             authorization_context,
//             available_addons,
//         )
//         .unwrap();

//     let (block_tx, block_rx) = txtx_addon_kit::channel::unbounded::<BlockEvent>();
//     let (action_item_updates_tx, _action_item_updates_rx) =
//         txtx_addon_kit::channel::unbounded::<ActionItemRequest>();
//     let (action_item_events_tx, action_item_events_rx) =
//         txtx_addon_kit::channel::unbounded::<ActionItemResponse>();

//     let _ = hiro_system_kit::thread_named("Runbook Runloop").spawn(move || {
//         let runloop_future = start_supervised_runbook_runloop(
//             &mut runbook,
//             block_tx,
//             action_item_updates_tx,
//             action_item_events_rx,
//         );
//         if let Err(diags) = hiro_system_kit::nestable_block_on(runloop_future) {
//             for diag in diags.iter() {
//                 println!("{}", diag);
//             }
//         }
//     });

//     let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
//         assert!(false, "unable to receive genesis block");
//         panic!()
//     };

//     let action_panel_data = event.expect_block().panel.expect_action_panel();

//     println!("{}", event.expect_block());

//     assert_eq!(action_panel_data.title.to_uppercase(), "RUNBOOK CHECKLIST");
//     assert_eq!(action_panel_data.groups.len(), 2);
//     assert_eq!(action_panel_data.groups[0].sub_groups.len(), 1);
//     assert_eq!(action_panel_data.groups[0].sub_groups[0].action_items.len(), 3);
//     assert_eq!(action_panel_data.groups[1].sub_groups.len(), 1);
//     assert_eq!(action_panel_data.groups[1].sub_groups[0].action_items.len(), 1);

//     let get_public_key = &action_panel_data.groups[0].sub_groups[0].action_items[0];
//     assert_eq!(get_public_key.action_status, ActionItemStatus::Todo);
//     let ActionItemRequestType::ProvidePublicKey(_request) = &get_public_key.action_type else {
//         panic!("expected provide public key request");
//     };

//     let check_public_key = &action_panel_data.groups[0].sub_groups[0].action_items[1];
//     assert_eq!(check_public_key.action_status, ActionItemStatus::Todo);
//     let ActionItemRequestType::ReviewInput(_) = &check_public_key.action_type else {
//         panic!("expected provide public key request");
//     };

//     let start_runbook = &action_panel_data.groups[1].sub_groups[0].action_items[0];
//     assert_eq!(start_runbook.action_status, ActionItemStatus::Success(None));
//     assert_eq!(start_runbook.title.to_uppercase(), "START RUNBOOK");

//     // Complete start_runbook action
//     let _ = action_item_events_tx.send(ActionItemResponse {
//         action_item_id: get_public_key.id.clone(),
//         payload: ActionItemResponseType::ProvidePublicKey(ProvidePublicKeyResponse {
//             public_key: "038665eaed5fc80bd01a1068f90f2e2de4c9c041f1865868169c848c0e770042e7".into(),
//         }),
//     });

//     // Complete start_runbook action
//     let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
//         assert!(false, "unable to receive input block");
//         panic!()
//     };

//     let updates = event.expect_updated_action_items();
//     assert_eq!(updates.len(), 3);
//     assert_eq!(
//         updates[0].action_status.as_ref().unwrap(),
//         &ActionItemStatus::Success(Some("ST12886CEM87N4TP9CGV91VWJ8FXVX57R6AG1AXS4".into()))
//     );
//     assert_eq!(
//         updates[1].action_status.as_ref().unwrap(),
//         &ActionItemStatus::Success(Some("ST12886CEM87N4TP9CGV91VWJ8FXVX57R6AG1AXS4".into()))
//     );

//     // Validate panel
//     let _ = action_item_events_tx.send(ActionItemResponse {
//         action_item_id: start_runbook.id.clone(),
//         payload: ActionItemResponseType::ValidateBlock,
//     });

//     let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
//         assert!(false, "unable to receive input block");
//         panic!()
//     };

//     let action_panel_data = event.expect_block().panel.expect_action_panel();
//     println!("{:?}", action_panel_data);
//     assert_eq!(action_panel_data.title, "Input Review");
//     assert_eq!(action_panel_data.groups.len(), 1);
//     assert_eq!(action_panel_data.groups[0].sub_groups.len(), 2);
//     assert_eq!(action_panel_data.groups[0].sub_groups[0].action_items.len(), 4);
//     assert_eq!(action_panel_data.groups[0].sub_groups[1].action_items.len(), 1);

//     let action_item_uuid = &action_panel_data.groups[0].sub_groups[0].action_items[1];
//     let _ = action_item_events_tx.send(ActionItemResponse {
//         action_item_id: action_item_uuid.id.clone(),
//         payload: ActionItemResponseType::ValidateBlock,
//     });

//     let Ok(event) = block_rx.recv_timeout(Duration::from_secs(5)) else {
//         assert!(false, "unable to receive input block");
//         panic!()
//     };

//     let action_panel_data = event.expect_block().panel.expect_action_panel();
//     println!("{:?}", action_panel_data);
// }

// sequenceDiagram
//     frontend->>+runloop:
//     runloop->>+signer_evaluation: Process signer
//     signer_evaluation->>+alice_signer: Compute ActionItemRequests
//     alice_signer-->>-signer_evaluation: ProvidePublicKey, InputReview[public_key], InputReview[balance], InputReview[costs]
//     signer_evaluation->>+bob_signer: Compute ActionItemRequests
//     bob_signer-->>-signer_evaluation: ProvidePublicKey, InputReview[public_key], InputReview[balance], InputReview[costs
//     signer_evaluation->>+multisig_signer: Compute ActionItemRequests
//     multisig_signer-->>-alice_signer: Is public key known?
//     alice_signer->>+multisig_signer: Hi Alice, I can hear you!
//     multisig_signer-->>-bob_signer: Is public key known?
//     bob_signer->>+multisig_signer: Hi Alice, I can hear you!
//     multisig_signer-->>-signer_evaluation: ProvidePublicKey, InputReview[public_key], InputReview[balance], InputReview[costs]
//     signer_evaluation->>+runloop: Push collected ActionItemRequests
//     runloop->>+frontend:
