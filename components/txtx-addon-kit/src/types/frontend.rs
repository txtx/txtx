use std::{
    borrow::BorrowMut,
    collections::{BTreeMap, HashMap},
    fmt::Display,
};

use super::{
    diagnostics::Diagnostic,
    types::{PrimitiveType, Value},
    ConstructUuid,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub enum BlockEvent {
    Append(Block),
    Clear,
    UpdateActionItems(Vec<SetActionItemStatus>),
    Exit,
    ProgressBar(ProgressBarStatus),
    Modal(Block),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressBarStatus {
    pub uuid: Uuid,
    pub visible: bool,
    pub status: String,
    pub message: String,
    pub diagnostic: Option<Diagnostic>,
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
    pub visible: bool,
}

impl Block {
    pub fn new(uuid: Uuid, panel: Panel) -> Self {
        Block {
            uuid,
            panel,
            visible: true,
        }
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

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Block {} {{", self.uuid)?;
        let panel = self.panel.expect_action_panel();
        writeln!(f, "  title: {}", panel.title)?;
        for group in self.panel.as_action_panel().unwrap().groups.iter() {
            writeln!(f, "  group: {} {{", group.title)?;
            for sub_group in group.sub_groups.iter() {
                writeln!(f, "    sub_group: {{")?;
                for item in sub_group.action_items.iter() {
                    writeln!(f, "      items: {} {{", item.uuid)?;
                    writeln!(f, "          status: {:?}", item.action_status)?;
                    writeln!(f, "          status: {:?}", item.action_type)?;
                    writeln!(f, "      }}")?;
                }
                writeln!(f, "    }}")?;
            }
            writeln!(f, "  }}")?;
        }
        writeln!(f, "}}")
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
#[serde(rename_all = "camelCase")]
pub struct UpdateConstructData {
    pub construct_uuid: ConstructUuid,
    pub action_status: ActionItemStatus,
}

#[derive(Clone, Debug)]
pub enum ActionType {
    UpdateActionItemRequest(ActionItemRequest),
    UpdateConstruct(UpdateConstructData),
    AppendSubGroup(ActionSubGroup),
    AppendGroup(ActionGroup),
    NewBlock(ActionPanelData),
}

#[derive(Clone, Debug)]
pub struct Actions {
    pub store: Vec<ActionType>,
}

impl Actions {
    pub fn none() -> Actions {
        Actions { store: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    pub fn append(&mut self, actions: &mut Actions) {
        self.store.append(&mut actions.store);
    }

    pub fn push_group(&mut self, title: &str, action_items: Vec<ActionItemRequest>) {
        self.store.push(ActionType::AppendGroup(ActionGroup {
            sub_groups: vec![ActionSubGroup {
                action_items,
                allow_batch_completion: false,
            }],
            title: title.to_string(),
        }));
    }

    pub fn push_sub_group(&mut self, action_items: Vec<ActionItemRequest>) {
        self.store.push(ActionType::AppendSubGroup(ActionSubGroup {
            action_items,
            allow_batch_completion: false,
        }));
    }

    pub fn push_status_update(&mut self, action_item_request: &ActionItemRequest) {
        self.store.push(ActionType::UpdateActionItemRequest(
            action_item_request.clone(),
        ))
    }

    pub fn push_status_udpate_construct_uuid(
        &mut self,
        construct_uuid: &ConstructUuid,
        status_update: ActionItemStatus,
    ) {
        self.store
            .push(ActionType::UpdateConstruct(UpdateConstructData {
                construct_uuid: construct_uuid.clone(),
                action_status: status_update.clone(),
            }))
    }

    pub fn new_panel(title: &str, description: &str) -> Actions {
        let store = vec![ActionType::NewBlock(ActionPanelData {
            title: title.to_string(),
            description: description.to_string(),
            groups: vec![],
        })];
        Actions { store }
    }

    pub fn new_group_of_items(title: &str, action_items: Vec<ActionItemRequest>) -> Actions {
        let store = vec![ActionType::AppendGroup(ActionGroup {
            sub_groups: vec![ActionSubGroup {
                action_items,
                allow_batch_completion: false,
            }],
            title: title.to_string(),
        })];
        Actions { store }
    }

    pub fn new_sub_group_of_items(action_items: Vec<ActionItemRequest>) -> Actions {
        let store = vec![ActionType::AppendSubGroup(ActionSubGroup {
            action_items,
            allow_batch_completion: false,
        })];
        Actions { store }
    }

    pub fn get_new_action_item_requests(&self) -> Vec<&ActionItemRequest> {
        let mut new_action_item_requests = vec![];
        for item in self.store.iter() {
            match item {
                ActionType::AppendSubGroup(data) => {
                    for item in data.action_items.iter() {
                        new_action_item_requests.push(item);
                    }
                }
                ActionType::AppendGroup(data) => {
                    for subgroup in data.sub_groups.iter() {
                        for item in subgroup.action_items.iter() {
                            new_action_item_requests.push(item);
                        }
                    }
                }
                ActionType::NewBlock(data) => {
                    for group in data.groups.iter() {
                        for subgroup in group.sub_groups.iter() {
                            for item in subgroup.action_items.iter() {
                                new_action_item_requests.push(item);
                            }
                        }
                    }
                }
                ActionType::UpdateActionItemRequest(_) => continue,
                ActionType::UpdateConstruct(_) => continue,
            }
        }
        new_action_item_requests
    }

    pub fn compile_actions_to_block_events(&self) -> Vec<BlockEvent> {
        let mut blocks = vec![];
        let mut current_panel_data = ActionPanelData {
            title: "".to_string(),
            description: "".to_string(),
            groups: vec![],
        };

        for item in self.store.iter() {
            match item {
                ActionType::AppendSubGroup(data) => {
                    if current_panel_data.groups.len() > 0 {
                        let Some(group) = current_panel_data.groups.last_mut() else {
                            continue;
                        };
                        group.sub_groups.push(data.clone());
                    } else {
                        current_panel_data.groups.push(ActionGroup {
                            title: "".to_string(),
                            sub_groups: vec![data.clone()],
                        });
                    }
                }
                ActionType::AppendGroup(data) => {
                    current_panel_data.groups.push(data.clone());
                }
                ActionType::NewBlock(data) => {
                    if current_panel_data.groups.len() > 1 {
                        blocks.push(BlockEvent::Append(Block {
                            uuid: Uuid::new_v4(),
                            panel: Panel::ActionPanel(current_panel_data.clone()),
                            visible: true,
                        }));
                    }
                    current_panel_data = data.clone();
                }
                ActionType::UpdateActionItemRequest(_) => continue,
                ActionType::UpdateConstruct(_) => continue,
            }
        }
        blocks.push(BlockEvent::Append(Block {
            uuid: Uuid::new_v4(),
            panel: Panel::ActionPanel(current_panel_data.clone()),
            visible: true,
        }));
        blocks
    }

    pub fn compile_actions_to_item_updates(
        &self,
        action_items_requests: &BTreeMap<Uuid, ActionItemRequest>,
    ) -> Vec<SetActionItemStatus> {
        let mut updates = vec![];
        let mut status_updates = HashMap::new();

        for item in self.store.iter() {
            match item {
                ActionType::AppendSubGroup(_) => {}
                ActionType::AppendGroup(_) => {}
                ActionType::NewBlock(_) => {}
                ActionType::UpdateConstruct(data) => {
                    status_updates.insert(data.construct_uuid.value(), data.action_status.clone());
                }
                ActionType::UpdateActionItemRequest(data) => updates.push(SetActionItemStatus {
                    action_item_uuid: data.construct_uuid.unwrap().clone(),
                    new_status: data.action_status.clone(),
                }),
            }
        }

        for (_, request) in action_items_requests.iter() {
            if let Some(construct_uuid) = request.construct_uuid {
                if let Some(status_update) = status_updates.get(&construct_uuid) {
                    updates.push(SetActionItemStatus {
                        action_item_uuid: construct_uuid.clone(),
                        new_status: status_update.clone(),
                    })
                }
            }
        }

        updates
    }
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

impl ActionItemRequestType {
    pub fn as_display_output(&self) -> Option<&DisplayOutputRequest> {
        match &self {
            ActionItemRequestType::DisplayOutput(value) => Some(value),
            _ => None,
        }
    }
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
pub struct ProvideSignedTransactionResponse {
    pub signed_transaction_bytes: String,
}
