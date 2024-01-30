use txtx_ext_kit::types::diagnostics::Diagnostic;

#[derive(Debug)]
pub enum DiscoveryError {
    UnknownConstruct(Diagnostic),
    VariableConstruct(Diagnostic),
    OutputConstruct(Diagnostic),
    ModuleConstruct(Diagnostic),
    ImportConstruct(Diagnostic),
    ExtConstruct(Diagnostic),
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
