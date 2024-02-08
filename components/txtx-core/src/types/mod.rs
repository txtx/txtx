mod construct;
mod manual;
mod package;

pub use construct::import::ImportConstruct;
pub use construct::module::ModuleConstruct;
pub use construct::output::OutputConstruct;
pub use construct::variable::VariableConstruct;
pub use construct::{Construct, ConstructData, PreConstruct, PreConstructData};
pub use manual::{Manual, SourceTree};
pub use package::{Package, PackageUuid};
use std::collections::HashMap;

use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::functions::FunctionSpecification;
use txtx_addon_kit::types::typing::Value;
pub use txtx_addon_kit::types::ConstructUuid;

use crate::std::functions::operators::OPERATORS_FUNCTIONS;
use crate::AddonsContext;

pub struct RuntimeContext {
    pub functions: HashMap<String, FunctionSpecification>,
    pub addons: AddonsContext,
}

impl RuntimeContext {
    pub fn new(addons: AddonsContext) -> RuntimeContext {
        let mut functions = HashMap::new();
        for function in OPERATORS_FUNCTIONS.iter() {
            functions.insert(function.name.clone(), function.clone());
        }

        RuntimeContext { functions, addons }
    }

    pub fn execute_function(&self, name: &str, args: &Vec<Value>) -> Result<Value, Diagnostic> {
        let function = self.functions.get(name).unwrap();
        let res = (function.runner)(function, args);
        Ok(res)
    }
}
