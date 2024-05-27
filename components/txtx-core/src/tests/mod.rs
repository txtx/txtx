use std::{collections::BTreeMap, time::Duration};

use txtx_addon_kit::{
    helpers::fs::FileLocation,
    hiro_system_kit,
    types::frontend::{ActionItem, ActionItemEvent, BlockEvent},
};

use crate::{
    start_runbook_runloop,
    std::StdAddon,
    types::{Runbook, RuntimeContext, SourceTree},
    AddonsContext,
};

#[test]
fn test_abc_runbook_no_env() {
    // Load Runbook abc.tx
    let abc_tx = include_str!("./fixtures/abc.tx");

    let mut source_tree = SourceTree::new();
    source_tree.add_source(
        "abc.tx".into(),
        FileLocation::from_path_string("/").unwrap(),
        abc_tx.into(),
    );

    let environments = BTreeMap::new();
    let mut addons_ctx = AddonsContext::new();
    addons_ctx.register(Box::new(StdAddon::new()));
    // addons_ctx.register(Box::new(StacksNetworkAddon::new()));

    let mut runtime_context = RuntimeContext::new(addons_ctx, environments.clone());
    let mut runbook = Runbook::new(Some(source_tree), None);

    let (block_tx, block_rx) = crate::channel::unbounded::<BlockEvent>();
    let (action_item_updates_tx, action_item_updates_rx) =
        crate::channel::unbounded::<ActionItem>();
    let (action_item_events_tx, action_item_events_rx) =
        crate::channel::unbounded::<ActionItemEvent>();

    let interactive_by_default = false;

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

    let Ok(event) = block_rx.recv_timeout(Duration::from_secs(1)) else {
        assert!(false, "unable to receive genesis block");
        panic!()
    };

    eprintln!("{:?}", event);

    let action_panel_data = event.expect_block().panel.expect_action_panel();
    assert_eq!(action_panel_data.title.to_uppercase(), "RUNBOOK CHECKLIST");
}
