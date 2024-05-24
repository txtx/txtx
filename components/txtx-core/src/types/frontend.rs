use txtx_addon_kit::{types::diagnostics::Diagnostic, uuid::Uuid};

pub enum RunbookExecutionState {
    RunbookGenesis,
    RunbookGlobalsUpdated,
}

#[derive(Debug, Clone)]
pub enum Block {
    ActionPanel(ActionPanelData),
}

#[derive(Debug, Clone)]
pub struct ActionPanelData {
    pub uuid: Uuid,
    pub title: String,
    pub description: String,
    pub groups: Vec<ActionGroup>,
}

#[derive(Debug, Clone)]
pub struct ActionGroup {
    pub title: String,
    pub sub_groups: Vec<ActionSubGroup>,
}

#[derive(Debug, Clone)]
pub struct ActionSubGroup {
    pub action_items: Vec<ActionItem>,
    pub allow_batch_completion: bool,
}

#[derive(Debug, Clone)]
pub struct ActionItem {
    pub uuid: Uuid,
    pub index: u16,
    pub title: String,
    pub description: String,
    pub action_status: ActionItemStatus,
    pub action_type: ActionItemType,
}

pub enum ChecklistActionResultProvider {
    TermConsole,
    LocalWebConsole,
    RemoteWebConsole,
}

#[derive(Debug, Clone)]
pub enum ActionItemStatus {
    Todo,
    Success,
    InProgress(String),
    Error(Diagnostic),
    Warning(Diagnostic),
}

#[derive(Debug, Clone)]
pub enum ActionItemType {
    ReviewInput,
    ProvideInput,
    PickInputOption(Vec<InputOption>),
    ProvidePublicKey(ProvidePublicKeyData),
    ProvideSignedTransaction(ProvideSignedTransactionData),
    ValidateChecklist,
}

#[derive(Debug, Clone)]
pub struct InputOption {
    pub value: String,
    pub displayed_value: String,
}

#[derive(Debug, Clone)]
pub struct ProvidePublicKeyData {
    pub check_expectation_action_uuid: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ProvideSignedTransactionData {
    pub check_expectation_action_uuid: Option<Uuid>,
}

pub struct ActionItemEvent {
    pub action_item_uuid: Uuid,
    pub payload: Vec<u8>,
}
