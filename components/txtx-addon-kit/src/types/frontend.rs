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
    pub uuid: Uuid,
    #[serde(flatten)]
    pub panel: Panel,
}

impl Block {
    pub fn new(uuid: Uuid, panel: Panel) -> Self {
        Block { uuid, panel }
    }

    pub fn find_action(&self, uuid: Uuid) -> Option<ActionItemRequest> {
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
    pub action_items: Vec<ActionItemRequest>,
    pub allow_batch_completion: bool,
}

impl ActionSubGroup {
    pub fn new(action_items: Vec<ActionItemRequest>, allow_batch_completion: bool) -> Self {
        ActionSubGroup {
            action_items,
            allow_batch_completion,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionItemRequest {
    pub uuid: Uuid,
    pub index: u16,
    pub title: String,
    pub description: String,
    pub action_status: ActionItemStatus,
    pub action_type: ActionItemRequestType,
}

impl ActionItemRequest {
    pub fn new(
        uuid: &Uuid,
        index: u16,
        title: &str,
        description: &str,
        action_status: ActionItemStatus,
        action_type: ActionItemRequestType,
    ) -> Self {
        ActionItemRequest {
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", content = "status_data")]
pub enum ActionItemStatus {
    Todo,
    Success,
    InProgress(String),
    Error(Diagnostic),
    Warning(Diagnostic),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "type_data")]
pub enum ActionItemRequestType {
    ReviewInput,
    ProvideInput(ProvideInputRequest),
    PickInputOption(Vec<InputOption>),
    ProvidePublicKey(ProvidePublicKeyRequest),
    ProvideSignedTransaction(ProvideSignedTransactionRequest),
    ValidatePanel,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvideInputRequest {
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
pub struct ProvidePublicKeyRequest {
    pub check_expectation_action_uuid: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvideSignedTransactionRequest {
    pub check_expectation_action_uuid: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionItemResponse {
    pub action_item_uuid: Uuid,
    #[serde(flatten)]
    pub payload: ActionItemResponseType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ActionItemResponseType {
    ReviewInput(bool),
    ProvideInput(ProvidedInputResponse),
    PickInputOption(String),
    ProvidePublicKey(ProvidePublicKeyResponse),
    ProvideSignedTransaction(ProvideSignedTransactionResponse),
    ValidatePanel,
}

impl ActionItemResponseType {
    pub fn is_validate_panel(&self) -> bool {
        match &self {
            ActionItemResponseType::ValidatePanel => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvidedInputResponse {
    pub input_name: String,
    pub updated_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvidePublicKeyResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvideSignedTransactionResponse {}
