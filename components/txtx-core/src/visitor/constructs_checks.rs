use std::sync::{Arc, RwLock};

use crate::{types::Runbook, AddonsContext};

pub fn run_constructs_checks(
    _runbook: &Arc<RwLock<Runbook>>,
    _addons_ctx: &mut AddonsContext,
) -> Result<(), String> {
    // todo(lgalabru): re-implement this pass with the new approach
    Ok(())
}
