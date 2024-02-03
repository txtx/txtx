use std::sync::mpsc::channel;

use txtx_ext_bitcoin::BitcoinCodec;
use txtx_gql::Context as GqlContext;
use txtx_core::{simulate_manual, CodecManager};

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
    let bitcoin_codec = BitcoinCodec::new();
    let mut codec_manager = CodecManager::new();
    codec_manager.register(Box::new(bitcoin_codec));
    simulate_manual(&mut manual, &mut codec_manager)?;

    if cmd.no_tui {
        manual.inspect_constructs();
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
        let bitcoin_codec = BitcoinCodec::new();
        let mut codec_manager = CodecManager::new();
        codec_manager.register(Box::new(bitcoin_codec));
        simulate_manual(manual, &mut codec_manager)?;
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
