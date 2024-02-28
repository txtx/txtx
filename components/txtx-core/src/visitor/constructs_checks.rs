use std::sync::RwLock;

use crate::{types::Manual, AddonsContext};

pub fn run_constructs_checks(
    _manual: &RwLock<Manual>,
    _addons_ctx: &mut AddonsContext,
) -> Result<(), String> {
    // todo(lgalabru): re-implement this pass with the new approach
    Ok(())
}
