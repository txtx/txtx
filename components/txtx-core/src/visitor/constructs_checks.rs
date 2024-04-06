use std::sync::{Arc, RwLock};

use crate::{types::Manual, AddonsContext};

pub fn run_constructs_checks(
    _manual: &Arc<RwLock<Manual>>,
    _addons_ctx: &mut AddonsContext,
) -> Result<(), String> {
    // todo(lgalabru): re-implement this pass with the new approach
    Ok(())
}
