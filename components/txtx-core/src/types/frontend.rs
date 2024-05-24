use txtx_addon_kit::{types::diagnostics::Diagnostic, uuid::Uuid};

pub enum RunbookExecutionState {
    RunbookGenesis,
    RunbookGlobalsUpdated,
}

pub enum Block {
    Checklist(Checklist),
}

pub struct Checklist {
    uuid: Uuid,
    name: String,
    description: String,
    items: Vec<ChecklistAction>,
}

pub enum ChecklistActionResultProvider {
    TermConsole,
    LocalWebConsole,
    RemoteWebConsole,
}

pub enum ChecklistActionStatus {
    Todo,
    Success,
    InProgress(String),
    Error(Diagnostic),
    Warning(Diagnostic),
}

pub struct ChecklistAction {
    uuid: Uuid,
    name: String,
    description: String,
    status: ChecklistActionStatus,
    action_type: ChecklistActionType,
}

pub enum ChecklistActionType {
    ReviewInput,
    ProvideInput,
    ProvidePublicKey(ProvidePublicKeyData),
    ProvideSignedTransaction(ProvideSignedTransactionData),
    ValidateChecklist,
}

pub struct ProvidePublicKeyData {
    check_expectation_action_uuid: Option<Uuid>,
}

pub struct ProvideSignedTransactionData {
    check_expectation_action_uuid: Option<Uuid>,
}

pub struct ChecklistActionEvent {
    checklist_action_uuid: Uuid,
    payload: Vec<u8>,
}
