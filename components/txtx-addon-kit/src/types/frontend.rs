use std::borrow::BorrowMut;

use super::{
    diagnostics::Diagnostic,
    types::{PrimitiveType, Value},
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub enum BlockEvent {
    Append(Block),
    Clear,
    UpdateActionItems(Vec<SetActionItemStatus>),
    Exit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetActionItemStatus {
    pub action_item_uuid: Uuid,
    pub new_status: ActionItemStatus,
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

    pub fn expect_updated_action_items(&self) -> &Vec<SetActionItemStatus> {
        match &self {
            BlockEvent::UpdateActionItems(ref updates) => updates,
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
    pub fn set_action_status(&mut self, action_item_uuid: Uuid, new_status: ActionItemStatus) {
        match self.panel.borrow_mut() {
            Panel::ActionPanel(panel) => {
                for group in panel.groups.iter_mut() {
                    for sub_group in group.sub_groups.iter_mut() {
                        for action in sub_group.action_items.iter_mut() {
                            if action.uuid == action_item_uuid {
                                action.action_status = new_status.clone();
                            }
                        }
                    }
                }
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
        }
    }

    pub fn expect_action_panel(&self) -> &ActionPanelData {
        match &self {
            Panel::ActionPanel(ref data) => data,
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
    pub construct_uuid: Option<Uuid>,
    pub index: u16,
    pub title: String,
    pub description: String,
    pub action_status: ActionItemStatus,
    pub action_type: ActionItemRequestType,
}

impl ActionItemRequest {
    pub fn new(
        uuid: &Uuid,
        construct_uuid: &Option<Uuid>,
        index: u16,
        title: &str,
        description: &str,
        action_status: ActionItemStatus,
        action_type: ActionItemRequestType,
    ) -> Self {
        ActionItemRequest {
            uuid: uuid.clone(),
            construct_uuid: construct_uuid.clone(),
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
#[serde(tag = "status", content = "data")]
pub enum ActionItemStatus {
    Todo,
    Success(Option<String>),
    InProgress(String),
    Error(Diagnostic),
    Warning(Diagnostic),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ActionItemRequestType {
    ReviewInput,
    ProvideInput(ProvideInputRequest),
    PickInputOption(Vec<InputOption>),
    ProvidePublicKey(ProvidePublicKeyRequest),
    ProvideSignedTransaction(ProvideSignedTransactionRequest),
    DisplayOutput(DisplayOutputRequest),
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
pub struct DisplayOutputRequest {
    pub name: String,
    pub description: Option<String>,
    pub value: Value,
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
    pub message: String,
    pub namespace: String,
    pub network_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvideSignedTransactionRequest {
    pub check_expectation_action_uuid: Option<Uuid>,
    pub payload: Value,
    pub namespace: String,
    pub network_id: String,
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
    ReviewInput(ReviewedInputResponse),
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
pub struct ReviewedInputResponse {
    pub input_name: String,
    pub value_checked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvidedInputResponse {
    pub input_name: String,
    pub updated_value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvidePublicKeyResponse {
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvideSignedTransactionResponse {}
