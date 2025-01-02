use kit::types::block_id::BlockId;
use txtx_addon_kit::types::{
    frontend::{
        ActionItemResponse, ActionItemResponseType, ActionItemStatus, ProvidedInputResponse,
        ReviewedInputResponse,
    },
    types::Value,
};
use txtx_test_utils::test_harness::setup_test;

#[test]
fn test_ab_c_runbook_no_env() {
    // Load Runbook ab_c.tx
    let abc_tx = include_str!("./fixtures/ab_c.tx");

    let harness = setup_test("ab_c.tx", &abc_tx, vec![]);

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
            harness.expect_action_panel(None, "variables review", vec![vec![2, 1]]);

        let input_a_action = &inputs_panel_data.groups[0].sub_groups[0].action_items[0];
        let input_b_action = &inputs_panel_data.groups[0].sub_groups[0].action_items[1];

        assert_eq!(&input_a_action.internal_key, "check_input");
        assert_eq!(&input_b_action.internal_key, "provide_input");

        // review input a and expect action item update
        harness.send_and_expect_action_item_update(
            ActionItemResponse {
                action_item_id: input_a_action.id.clone(),
                payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                    value_checked: true,
                    input_name: "value".into(),
                    force_execution: false,
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
                updated_value: Value::integer(5),
                input_name: "value".into(),
            }),
        });
        // review input b and expect action item update
        harness.send_and_expect_action_item_update(
            ActionItemResponse {
                action_item_id: input_b_action.id.clone(),
                payload: ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                    value_checked: true,
                    input_name: "value".into(),
                    force_execution: false,
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
