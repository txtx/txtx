mod construct;
mod package;
mod runbook;

pub use construct::PreConstructData;
pub use package::Package;
pub use runbook::{Runbook, SourceTree};
use std::collections::HashMap;
pub use txtx_addon_kit::types::commands::CommandInstance;

use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::functions::FunctionSpecification;
use txtx_addon_kit::types::types::Value;
pub use txtx_addon_kit::types::ConstructUuid;

use crate::std::functions::operators::OPERATORS_FUNCTIONS;
use crate::AddonsContext;

pub struct RuntimeContext {
    pub functions: HashMap<String, FunctionSpecification>,
    pub addons_ctx: AddonsContext,
}

impl RuntimeContext {
    pub fn new(addons_ctx: AddonsContext) -> RuntimeContext {
        let mut functions = HashMap::new();
        for function in OPERATORS_FUNCTIONS.iter() {
            functions.insert(function.name.clone(), function.clone());
        }

        for (_, addon) in addons_ctx.addons.iter() {
            for function in addon.get_functions().iter() {
                functions.insert(function.name.clone(), function.clone());
            }
        }

        RuntimeContext {
            functions,
            addons_ctx,
        }
    }

    pub fn execute_function(&self, name: &str, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let function = match self.functions.get(name) {
            Some(function) => function,
            None => {
                todo!("return diagnostic");
            }
        };
        (function.runner)(function, args)
    }
}
