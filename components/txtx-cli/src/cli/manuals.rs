use std::sync::mpsc::channel;

use txtx_addon_network_stacks::StacksNetworkAddon;
use txtx_core::{simulate_manual, types::RuntimeContext, AddonsContext};
use txtx_gql::Context as GqlContext;

use crate::{
    manifest::{read_manifest_at_path, read_manuals_from_manifest},
    term_ui, web_ui,
};

use super::{CheckManuals, Context, InspectManual, RunManual};

pub async fn handle_check_command(cmd: &CheckManuals, _ctx: &Context) -> Result<(), String> {
    let manifest_file_path = match cmd.manifest_path {
        Some(ref path) => path.clone(),
        None => "protocol.json".to_string(),
    };
    let manifest = read_manifest_at_path(&manifest_file_path)?;
    let _ = read_manuals_from_manifest(&manifest, None)?;
    // let _ = txtx::check_plan(plan)?;
    Ok(())
}

pub async fn handle_inspect_command(cmd: &InspectManual, _ctx: &Context) -> Result<(), String> {
    let manifest_file_path = match cmd.manifest_path {
        Some(ref path) => path.clone(),
        None => "protocol.json".to_string(),
    };
    let manual_name = cmd.manual.clone().unwrap();
    let manifest = read_manifest_at_path(&manifest_file_path)?;
    let mut manual = read_manuals_from_manifest(&manifest, Some(&vec![manual_name.clone()]))
        .ok()
        .and_then(|mut m| m.remove(&manual_name))
        .ok_or(format!(
            "unable to find entry '{}' in manifest {}",
            manual_name, manifest_file_path
        ))?;
    let stacks_addon = StacksNetworkAddon::new();
    let mut addons_ctx = AddonsContext::new();
    addons_ctx.register(Box::new(stacks_addon));

    let mut runtime_context = RuntimeContext::new(addons_ctx);
    simulate_manual(&mut manual, &mut runtime_context)?;

    if cmd.no_tui {
        // manual.inspect_constructs();
    } else {
        let _ = term_ui::inspect::main(manual);
    }
    Ok(())
}

pub async fn handle_run_command(cmd: &RunManual, ctx: &Context) -> Result<(), String> {
    let manifest_file_path = match cmd.manifest_path {
        Some(ref path) => path.clone(),
        None => "protocol.json".to_string(),
    };
    // let manuals_names = vec![cmd.manual.clone().unwrap()];
    let manifest = read_manifest_at_path(&manifest_file_path)?;
    let mut manuals = read_manuals_from_manifest(&manifest, None)?;
    for (_, manual) in manuals.iter_mut() {
        let stacks_addon = StacksNetworkAddon::new();
        let mut addons_ctx = AddonsContext::new();
        addons_ctx.register(Box::new(stacks_addon));
        let mut runtime_context = RuntimeContext::new(addons_ctx);
        simulate_manual(manual, &mut runtime_context)?;
    }
    let (tx, rx) = channel();

    let gql_context = GqlContext { manuals };
    let _ = web_ui::http::start_server(gql_context, ctx).await;
    match rx.recv() {
        Ok(_) => {}
        Err(_) => {}
    };
    let _ = tx.send(true);
    Ok(())
}
