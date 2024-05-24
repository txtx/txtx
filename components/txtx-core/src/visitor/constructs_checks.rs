use txtx_addon_kit::types::diagnostics::Diagnostic;

use crate::{types::Runbook, AddonsContext};

pub fn run_constructs_checks(
    _runbook: &Runbook,
    _addons_ctx: &mut AddonsContext,
) -> Result<(), Vec<Diagnostic>> {
    // todo(lgalabru): re-implement this pass with the new approach
    Ok(())
}
