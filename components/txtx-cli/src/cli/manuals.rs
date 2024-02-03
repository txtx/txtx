use std::sync::mpsc::channel;

use txtx_ext_ethereum::EthereumExtension;
use txtx_gql::Context as GqlContext;
use txtx_vm::{simulate_manual, ExtensionManager};

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
        .unwrap();
    let ethereum_extension = EthereumExtension::new();
    let mut extension_manager = ExtensionManager::new();
    extension_manager.register(Box::new(ethereum_extension));
    simulate_manual(&mut manual, &mut extension_manager)?;

    if cmd.no_tui {
        manual.inspect_constructs(&extension_manager);
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
        let ethereum_extension = EthereumExtension::new();
        let mut extension_manager = ExtensionManager::new();
        extension_manager.register(Box::new(ethereum_extension));
        simulate_manual(manual, &mut extension_manager)?;
    }
    let (tx, rx) = channel();

    let gql_context = GqlContext { manuals };
    let _ = web_ui::http::start_server(gql_context, ctx).await;
    let _ = match rx.recv() {
        Ok(_) => {}
        Err(_) => {}
    };
    let _ = tx.send(true);
    Ok(())
}
