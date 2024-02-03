use txtx_addon_kit::types::diagnostics::Diagnostic;

#[derive(Debug)]
pub enum DiscoveryError {
    UnknownConstruct(Diagnostic),
    VariableConstruct(Diagnostic),
    OutputConstruct(Diagnostic),
    ModuleConstruct(Diagnostic),
    ImportConstruct(Diagnostic),
    AddonConstruct(Diagnostic),
}

#[derive(Debug)]
pub enum DependenciesError {
    CycleDetected(Diagnostic),
}

#[derive(Debug)]
pub enum ConstructErrors {
    Discovery(DiscoveryError),
    Dependencies(DependenciesError),
}
