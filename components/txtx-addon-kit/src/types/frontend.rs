use super::{diagnostics::Diagnostic, types::PrimitiveType};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum BlockEvent {
    Append(Block),
    Clear,
    SetActionItemStatus((Uuid, Uuid, ActionItemStatus)),
}

impl BlockEvent {
    pub fn as_block(&self) -> Option<&Block> {
        match &self {
            BlockEvent::Append(ref block) => Some(block),
            _ => None,
        }
    }

    pub fn expect_block(&self) -> &Block {
        match &self {
            BlockEvent::Append(ref block) => block,
            _ => unreachable!("block expected"),
        }
    }
}

pub enum RunbookExecutionState {
    RunbookGenesis,
    RunbookGlobalsUpdated,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    #[serde(skip_serializing)]
    pub uuid: Uuid,
    #[serde(flatten)]
    pub panel: Panel,
}

impl Block {
    pub fn new(uuid: Uuid, panel: Panel) -> Self {
        Block { uuid, panel }
    }

    pub fn find_action(&self, uuid: Uuid) -> Option<ActionItem> {
        match &self.panel {
            Panel::ActionPanel(panel) => {
                for group in panel.groups.iter() {
                    for sub_group in group.sub_groups.iter() {
                        for action in sub_group.action_items.iter() {
                            if action.uuid == uuid {
                                return Some(action.clone());
                            }
                        }
                    }
                }
                return None;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Panel {
    ActionPanel(ActionPanelData),
}

impl Panel {
    pub fn new_action_panel(title: &str, description: &str, groups: Vec<ActionGroup>) -> Self {
        Panel::ActionPanel(ActionPanelData {
            title: title.to_string(),
            description: description.to_string(),
            groups,
        })
    }

    pub fn as_action_panel(&self) -> Option<&ActionPanelData> {
        match &self {
            Panel::ActionPanel(ref data) => Some(data),
            _ => None,
        }
    }

    pub fn expect_action_panel(&self) -> &ActionPanelData {
        match &self {
            Panel::ActionPanel(ref data) => data,
            _ => unreachable!("action panel expected"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionPanelData {
    pub title: String,
    pub description: String,
    pub groups: Vec<ActionGroup>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionGroup {
    pub title: String,
    pub sub_groups: Vec<ActionSubGroup>,
}

impl ActionGroup {
    pub fn new(title: &str, sub_groups: Vec<ActionSubGroup>) -> Self {
        ActionGroup {
            title: title.to_string(),
            sub_groups: sub_groups,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionSubGroup {
    pub action_items: Vec<ActionItem>,
    pub allow_batch_completion: bool,
}

impl ActionSubGroup {
    pub fn new(action_items: Vec<ActionItem>, allow_batch_completion: bool) -> Self {
        ActionSubGroup {
            action_items,
            allow_batch_completion,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionItem {
    pub uuid: Uuid,
    pub index: u16,
    pub title: String,
    pub description: String,
    pub action_status: ActionItemStatus,
    pub action_type: ActionItemType,
}

impl ActionItem {
    pub fn new(
        uuid: &Uuid,
        index: u16,
        title: &str,
        description: &str,
        action_status: ActionItemStatus,
        action_type: ActionItemType,
    ) -> Self {
        ActionItem {
            uuid: uuid.clone(),
            index,
            title: title.to_string(),
            description: description.to_string(),
            action_status,
            action_type,
        }
    }
}

pub enum ChecklistActionResultProvider {
    TermConsole,
    LocalWebConsole,
    RemoteWebConsole,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ActionItemStatus {
    Todo,
    Success,
    InProgress(String),
    Error(Diagnostic),
    Warning(Diagnostic),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ActionItemType {
    ReviewInput,
    ProvideInput(ProvideInputContext),
    PickInputOption(Vec<InputOption>),
    ProvidePublicKey(ProvidePublicKeyData),
    ProvideSignedTransaction(ProvideSignedTransactionData),
    ValidatePanel,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvideInputContext {
    pub input_name: String,
    pub typing: PrimitiveType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputOption {
    pub value: String,
    pub displayed_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvidePublicKeyData {
    pub check_expectation_action_uuid: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvideSignedTransactionData {
    pub check_expectation_action_uuid: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionItemEvent {
    pub action_item_uuid: Uuid,
    #[serde(flatten)]
    pub payload: ActionItemPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type", content = "data")]
pub enum ActionItemPayload {
    ReviewInput,
    ProvideInput(ProvidedInputData),
    PickInputOption(String),
    ProvidePublicKey(ProvidePublicKeyData),
    ProvideSignedTransaction(ProvideSignedTransactionData),
    ValidatePanel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvidedInputData {
    pub input_name: String,
    pub value: String,
    pub typing: PrimitiveType,
}
