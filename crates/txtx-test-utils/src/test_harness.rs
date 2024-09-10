use std::time::Duration;

use txtx_addon_kit::{
    channel::{Receiver, Sender},
    futures::executor::block_on,
    helpers::fs::FileLocation,
    types::{
        block_id::BlockId,
        frontend::{
            ActionItemRequest, ActionItemResponse, ActionItemStatus, ActionPanelData, BlockEvent,
            ModalPanelData, NormalizedActionItemRequestUpdate,
        },
        AuthorizationContext, RunbookId,
    },
    Addon,
};
use txtx_core::{
    runbook::RunbookInputsMap,
    start_supervised_runbook_runloop,
    types::{Runbook, RunbookSources},
};
#[allow(unused)]
pub struct TestHarness {
    block_tx: Sender<BlockEvent>,
    block_rx: Receiver<BlockEvent>,
    action_item_updates_tx: Sender<ActionItemRequest>,
    action_item_events_tx: Sender<ActionItemResponse>,
    action_item_events_rx: Receiver<ActionItemResponse>,
}

#[allow(unused)]
impl TestHarness {
    pub fn send(&self, response: &ActionItemResponse) {
        let _ = self.action_item_events_tx.send(response.clone());
    }

    pub fn receive_event(&self) -> BlockEvent {
        let Ok(event) = self.block_rx.recv_timeout(Duration::from_secs(5)) else {
            panic!("unable to receive input block");
        };
        event
    }

    pub fn send_and_expect_action_item_update(
        &self,
        response: ActionItemResponse,
        expected_updates: Vec<(&BlockId, Option<ActionItemStatus>)>,
    ) -> Vec<NormalizedActionItemRequestUpdate> {
        self.send(&response);
        self.expect_action_item_update(Some(response), expected_updates)
    }

    pub fn expect_action_item_update(
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

    pub fn send_and_expect_action_panel(
        &self,
        response: ActionItemResponse,
        expected_title: &str,
        expected_group_lengths: Vec<Vec<usize>>,
    ) -> ActionPanelData {
        self.send(&response);
        self.expect_action_panel(Some(response), expected_title, expected_group_lengths)
    }

    pub fn send_and_expect_modal(
        &self,
        response: ActionItemResponse,
        expected_title: &str,
        expected_group_lengths: Vec<Vec<usize>>,
    ) -> ModalPanelData {
        self.send(&response);
        self.expect_modal(Some(response), expected_title, expected_group_lengths)
    }

    pub fn expect_action_panel(
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
            "unexpected panel title after sending action item response: {:?}\n=>full panel: {:?}",
            response,
            action_panel_data
        );
        let ctx = format!(
            "\n=> response triggering panel: {:?}\n=> actual panel group: {:?}",
            response, action_panel_data.groups
        );
        assert_eq!(action_panel_data.groups.len(), expected_group_lengths.len(), "{}", ctx);
        action_panel_data.groups.iter().enumerate().for_each(|(i, g)| {
            let expected_sub_groups = &expected_group_lengths[i];
            assert_eq!(g.sub_groups.len(), expected_sub_groups.len(), "{}", ctx);
            g.sub_groups.iter().enumerate().for_each(|(j, s)| {
                assert_eq!(s.action_items.len(), expected_sub_groups[j], "{}", ctx);
            })
        });
        action_panel_data.clone()
    }

    pub fn expect_modal(
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
        assert_eq!(modal_panel_data.groups.len(), expected_group_lengths.len(), "{}", ctx);
        modal_panel_data.groups.iter().enumerate().for_each(|(i, g)| {
            let expected_sub_groups = &expected_group_lengths[i];
            assert_eq!(g.sub_groups.len(), expected_sub_groups.len(), "{}", ctx);
            g.sub_groups.iter().enumerate().for_each(|(j, s)| {
                assert_eq!(s.action_items.len(), expected_sub_groups[j], "{}", ctx);
            })
        });
        modal_panel_data.clone()
    }

    pub fn expect_noop(&self) {
        let Err(_) = self.block_rx.recv_timeout(Duration::from_secs(2)) else {
            panic!("unable to receive input block")
        };
    }

    pub fn expect_runbook_complete(&self) {
        let Ok(event) = self.block_rx.recv_timeout(Duration::from_secs(5)) else {
            assert!(false, "unable to receive input block");
            panic!()
        };

        event.expect_runbook_completed();
    }

    pub fn assert_provide_signature_formatted_payload(
        &self,
        action: &ActionItemRequest,
        formatted_payload: Option<String>,
    ) {
        let Some(action) = action.action_type.as_provide_signed_tx() else {
            panic!("expected sign transaction payload, found {:?}", action);
        };
        assert_eq!(
            action.formatted_payload, formatted_payload,
            "mismatching payload for action {:?}",
            action
        );
    }
}

pub fn setup_test(
    file_name: &str,
    fixture: &str,
    available_addons: Vec<Box<dyn Addon>>,
) -> TestHarness {
    let mut runbook_sources = RunbookSources::new();
    runbook_sources.add_source(
        file_name.into(),
        FileLocation::from_path_string(".").unwrap(),
        fixture.into(),
    );
    let runbook_inputs = RunbookInputsMap::new();

    let runbook_id = RunbookId { org: None, workspace: None, name: "test".into() };

    let mut runbook = Runbook::new(runbook_id, None);
    let authorization_context = AuthorizationContext::empty();
    let future = runbook.build_contexts_from_sources(
        runbook_sources,
        runbook_inputs,
        authorization_context,
        available_addons,
    );
    let _ = block_on(future);

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
