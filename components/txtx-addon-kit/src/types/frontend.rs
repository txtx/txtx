use std::{borrow::BorrowMut, collections::BTreeMap, fmt::Display};

use super::{
    diagnostics::Diagnostic,
    types::{Type, Value},
    ConstructUuid,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub enum BlockEvent {
    Action(Block),
    Clear,
    UpdateActionItems(Vec<NormalizedActionItemRequestUpdate>),
    RunbookCompleted,
    Exit,
    ProgressBar(Block),
    UpdateProgressBarStatus(ProgressBarStatusUpdate),
    UpdateProgressBarVisibility(ProgressBarVisibilityUpdate),
    Modal(Block),
    Error(Block),
}

impl BlockEvent {
    pub fn as_block(&self) -> Option<&Block> {
        match &self {
            BlockEvent::Action(ref block) => Some(block),
            _ => None,
        }
    }

    pub fn as_modal(&self) -> Option<&Block> {
        match &self {
            BlockEvent::Modal(ref block) => Some(block),
            _ => None,
        }
    }

    pub fn expect_block(&self) -> &Block {
        match &self {
            BlockEvent::Action(ref block) => block,
            _ => unreachable!("block expected"),
        }
    }

    pub fn expect_modal(&self) -> &Block {
        match &self {
            BlockEvent::Modal(ref block) => block,
            _ => unreachable!("block expected"),
        }
    }

    pub fn expect_updated_action_items(&self) -> &Vec<NormalizedActionItemRequestUpdate> {
        match &self {
            BlockEvent::UpdateActionItems(ref updates) => updates,
            _ => unreachable!("block expected"),
        }
    }

    pub fn expect_runbook_completed(&self) {
        match &self {
            BlockEvent::RunbookCompleted => {}
            _ => unreachable!("block expected"),
        }
    }

    pub fn new_modal(title: &str, description: &str, groups: Vec<ActionGroup>) -> Block {
        Block {
            uuid: Uuid::new_v4(),
            panel: Panel::new_modal_panel(title, description, groups),
            visible: false,
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
    pub fn new(uuid: &Uuid, panel: Panel) -> Self {
        Block {
            uuid: uuid.clone(),
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
            Panel::ModalPanel(panel) => {
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
            Panel::ErrorPanel(panel) => {
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
            Panel::ProgressBar(_) => None,
        }
    }

    pub fn update_action_item(&mut self, update: NormalizedActionItemRequestUpdate) {
        match self.panel.borrow_mut() {
            Panel::ActionPanel(panel) => {
                for group in panel.groups.iter_mut() {
                    for sub_group in group.sub_groups.iter_mut() {
                        for action in sub_group.action_items.iter_mut() {
                            if action.uuid == update.uuid {
                                if let Some(title) = update.title.clone() {
                                    action.title = title;
                                }
                                if let Some(some_description) = update.description.clone() {
                                    action.description = some_description;
                                }
                                if let Some(action_status) = update.action_status.clone() {
                                    action.action_status = action_status;
                                }
                                if let Some(action_type) = update.action_type.clone() {
                                    action.action_type = action_type;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn update_progress_bar_status(
        &mut self,
        construct_uuid: &Uuid,
        new_status: &ProgressBarStatus,
    ) {
        match self.panel.borrow_mut() {
            Panel::ProgressBar(progress_bar) => {
                let mut construct_status_found = false;
                for construct_statuses in progress_bar.iter_mut() {
                    if &construct_statuses.construct_uuid == construct_uuid {
                        construct_statuses.push_status(new_status);
                        construct_status_found = true;
                    }
                }
                if !construct_status_found {
                    progress_bar.push(ConstructProgressBarStatuses {
                        construct_uuid: construct_uuid.clone(),
                        statuses: vec![new_status.clone()],
                    })
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Serialize)]
/// Note: though the `action_status` field is optional, it is required for many functions. I kep it like this
/// because I like the `.set_status` pattern :-)
pub struct NormalizedActionItemRequestUpdate {
    pub uuid: Uuid,
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub action_status: Option<ActionItemStatus>,
    pub action_type: Option<ActionItemRequestType>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionItemRequestUpdate {
    pub id: ActionItemRequestUpdateIdentifier,
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub action_status: Option<ActionItemStatus>,
    pub action_type: Option<ActionItemRequestType>,
}

#[derive(Debug, Clone, Serialize)]
pub enum ActionItemRequestUpdateIdentifier {
    Uuid(Uuid),
    ConstructUuidWithKey((Uuid, String)),
}

impl ActionItemRequestUpdate {
    pub fn from_uuid(uuid: &Uuid) -> Self {
        ActionItemRequestUpdate {
            id: ActionItemRequestUpdateIdentifier::Uuid(uuid.clone()),
            title: None,
            description: None,
            action_status: None,
            action_type: None,
        }
    }
    pub fn from_context(construct_uuid: &ConstructUuid, internal_key: &str) -> Self {
        ActionItemRequestUpdate {
            id: ActionItemRequestUpdateIdentifier::ConstructUuidWithKey((
                construct_uuid.value(),
                internal_key.to_string(),
            )),
            title: None,
            description: None,
            action_status: None,
            action_type: None,
        }
    }

    pub fn set_status(&mut self, new_status: ActionItemStatus) -> Self {
        self.action_status = Some(new_status);
        self.clone()
    }

    pub fn set_type(&mut self, new_type: ActionItemRequestType) -> Self {
        self.action_type = Some(new_type);
        self.clone()
    }

    pub fn set_description(&mut self, new_description: Option<String>) -> Self {
        self.description = Some(new_description);
        self.clone()
    }

    pub fn normalize(
        &self,
        action_item_requests: &BTreeMap<Uuid, ActionItemRequest>,
    ) -> Option<NormalizedActionItemRequestUpdate> {
        for (_, action) in action_item_requests.iter() {
            match &self.id {
                ActionItemRequestUpdateIdentifier::Uuid(uuid) => {
                    if action.uuid.eq(uuid) {
                        return Some(NormalizedActionItemRequestUpdate {
                            uuid: uuid.clone(),
                            title: self.title.clone(),
                            description: self.description.clone(),
                            action_status: self.action_status.clone(),
                            action_type: self.action_type.clone(),
                        });
                    }
                }
                ActionItemRequestUpdateIdentifier::ConstructUuidWithKey((
                    construct_uuid,
                    internal_key,
                )) => {
                    let Some(action_construct_uuid) = action.construct_uuid else {
                        continue;
                    };
                    if action_construct_uuid.eq(construct_uuid)
                        && action.internal_key.eq(internal_key)
                    {
                        return Some(NormalizedActionItemRequestUpdate {
                            uuid: action.uuid,
                            title: self.title.clone(),
                            description: self.description.clone(),
                            action_status: self.action_status.clone(),
                            action_type: self.action_type.clone(),
                        });
                    }
                }
            }
        }
        None
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Block {} {{", self.uuid)?;
        match &self.panel {
            Panel::ActionPanel(panel) => {
                writeln!(f, "  title: {}", panel.title)?;
                for group in panel.groups.iter() {
                    writeln!(f, "  group: {} {{", group.title)?;
                    for sub_group in group.sub_groups.iter() {
                        writeln!(f, "    sub_group: {{")?;
                        for item in sub_group.action_items.iter() {
                            writeln!(f, "      items: {} {{", item.uuid)?;
                            writeln!(f, "          title: {:?}", item.title)?;
                            writeln!(f, "          consctruct: {:?}", item.construct_uuid)?;
                            writeln!(f, "          status: {:?}", item.action_status)?;
                            writeln!(f, "          action: {:?}", item.action_type)?;
                            writeln!(f, "      }}")?;
                        }
                        writeln!(f, "    }}")?;
                    }
                    writeln!(f, "  }}")?;
                }
                writeln!(f, "}}")
            }
            Panel::ModalPanel(panel) => {
                writeln!(f, "  title: {}", panel.title)?;
                for group in panel.groups.iter() {
                    writeln!(f, "  group: {} {{", group.title)?;
                    for sub_group in group.sub_groups.iter() {
                        writeln!(f, "    sub_group: {{")?;
                        for item in sub_group.action_items.iter() {
                            writeln!(f, "      items: {} {{", item.uuid)?;
                            writeln!(f, "          title: {:?}", item.title)?;
                            writeln!(f, "          consctruct: {:?}", item.construct_uuid)?;
                            writeln!(f, "          status: {:?}", item.action_status)?;
                            writeln!(f, "          action: {:?}", item.action_type)?;
                            writeln!(f, "      }}")?;
                        }
                        writeln!(f, "    }}")?;
                    }
                    writeln!(f, "  }}")?;
                }
                writeln!(f, "}}")
            }

            _ => {
                writeln!(f, "?????")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum Panel {
    ActionPanel(ActionPanelData),
    ModalPanel(ModalPanelData),
    ProgressBar(Vec<ConstructProgressBarStatuses>),
    ErrorPanel(ErrorPanelData),
}

impl Panel {
    pub fn new_action_panel(title: &str, description: &str, groups: Vec<ActionGroup>) -> Self {
        Panel::ActionPanel(ActionPanelData {
            title: title.to_string(),
            description: description.to_string(),
            groups,
        })
    }

    pub fn new_modal_panel(title: &str, description: &str, groups: Vec<ActionGroup>) -> Self {
        Panel::ModalPanel(ModalPanelData {
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

    pub fn as_modal_panel(&self) -> Option<&ModalPanelData> {
        match &self {
            Panel::ModalPanel(ref data) => Some(data),
            _ => None,
        }
    }

    pub fn expect_action_panel(&self) -> &ActionPanelData {
        match &self {
            Panel::ActionPanel(ref data) => data,
            _ => panic!("expected action panel, got {:?}", self),
        }
    }

    pub fn expect_modal_panel(&self) -> &ModalPanelData {
        match &self {
            Panel::ModalPanel(ref data) => data,
            _ => panic!("expected action panel, got {:?}", self),
        }
    }

    pub fn expect_modal_panel_mut(&mut self) -> &mut ModalPanelData {
        match self {
            Panel::ModalPanel(ref mut data) => data,
            _ => panic!("expected action panel, got {:?}", self),
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
pub struct ConstructProgressBarStatuses {
    pub construct_uuid: Uuid,
    pub statuses: Vec<ProgressBarStatus>,
}

impl ConstructProgressBarStatuses {
    pub fn new(construct_uuid: &Uuid) -> Self {
        ConstructProgressBarStatuses {
            construct_uuid: construct_uuid.clone(),
            statuses: vec![],
        }
    }
    pub fn push_status(&mut self, new_status: &ProgressBarStatus) {
        self.statuses.push(new_status.clone());
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressBarStatus {
    pub status: String,
    pub message: String,
    pub diagnostic: Option<Diagnostic>,
}

impl ProgressBarStatus {
    pub fn new_msg(status: &str, message: &str) -> Self {
        ProgressBarStatus {
            status: status.to_string(),
            message: message.to_string(),
            diagnostic: None,
        }
    }
    pub fn new_err(status: &str, message: &str, diag: &Diagnostic) -> Self {
        ProgressBarStatus {
            status: status.to_string(),
            message: message.to_string(),
            diagnostic: Some(diag.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressBarStatusUpdate {
    pub progress_bar_uuid: Uuid,
    pub construct_uuid: Uuid,
    pub new_status: ProgressBarStatus,
}

impl ProgressBarStatusUpdate {
    pub fn new(
        progress_bar_uuid: &Uuid,
        &construct_uuid: &Uuid,
        new_status: &ProgressBarStatus,
    ) -> Self {
        ProgressBarStatusUpdate {
            progress_bar_uuid: progress_bar_uuid.clone(),
            construct_uuid: construct_uuid.clone(),
            new_status: new_status.clone(),
        }
    }
    pub fn update_status(&mut self, new_status: &ProgressBarStatus) {
        self.new_status = new_status.clone();
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressBarVisibilityUpdate {
    pub progress_bar_uuid: Uuid,
    pub visible: bool,
}
impl ProgressBarVisibilityUpdate {
    pub fn new(progress_bar_uuid: &Uuid, visible: bool) -> Self {
        ProgressBarVisibilityUpdate {
            progress_bar_uuid: progress_bar_uuid.clone(),
            visible,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModalPanelData {
    pub title: String,
    pub description: String,
    pub groups: Vec<ActionGroup>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPanelData {
    pub title: String,
    pub description: String,
    pub groups: Vec<ActionGroup>,
}

impl ErrorPanelData {
    pub fn from_diagnostics(diagnostics: &Vec<Diagnostic>) -> Self {
        let mut diag_actions = vec![];
        for (i, diag) in diagnostics.iter().enumerate() {
            diag_actions.push(ActionItemRequest {
                uuid: Uuid::new_v4(),
                construct_uuid: None,
                index: i as u16,
                title: "".into(),
                description: None,
                action_status: ActionItemStatus::Error(diag.clone()),
                action_type: ActionItemRequestType::DisplayErrorLog(DisplayErrorLogRequest {
                    diagnostic: diag.clone(),
                }),
                internal_key: "diagnostic".to_string(),
            });
        }
        ErrorPanelData {
            title: "EXECUTION ERROR".into(),
            description: "Review the following execution errors and restart the runbook.".into(),
            groups: vec![ActionGroup {
                title: "".into(),
                sub_groups: vec![ActionSubGroup {
                    action_items: diag_actions,
                    allow_batch_completion: false,
                }],
            }],
        }
    }
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
    pub fn contains_validate_modal_item(&self) -> bool {
        for sub_group in self.sub_groups.iter() {
            for item in sub_group.action_items.iter() {
                if let ActionItemRequestType::ValidateModal = item.action_type {
                    return true;
                }
            }
        }
        false
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

    pub fn contains_validate_modal_item(&self) -> bool {
        for item in self.action_items.iter() {
            if let ActionItemRequestType::ValidateModal = item.action_type {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionItemRequest {
    pub uuid: Uuid,
    pub construct_uuid: Option<Uuid>,
    pub index: u16,
    pub title: String,
    pub description: Option<String>,
    pub action_status: ActionItemStatus,
    pub action_type: ActionItemRequestType,
    pub internal_key: String,
}

impl ActionItemRequest {
    pub fn new(
        uuid: &Uuid,
        construct_uuid: &Option<Uuid>,
        title: &str,
        description: Option<String>,
        action_status: ActionItemStatus,
        action_type: ActionItemRequestType,
        internal_key: &str,
    ) -> Self {
        ActionItemRequest {
            uuid: uuid.clone(),
            construct_uuid: construct_uuid.clone(),
            index: 0,
            title: title.to_string(),
            description,
            action_status,
            action_type,
            internal_key: internal_key.to_string(),
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
    pub action_item_update: ActionItemRequestUpdate,
    pub internal_key: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenModalData {
    pub modal_uuid: Uuid,
    pub title: String,
}

#[derive(Clone, Debug)]
pub enum ActionType {
    UpdateActionItemRequest(ActionItemRequestUpdate),
    AppendSubGroup(ActionSubGroup),
    AppendGroup(ActionGroup),
    AppendItem(ActionItemRequest, Option<String>, Option<String>),
    NewBlock(ActionPanelData),
    NewModal(Block),
}

#[derive(Clone, Debug)]
pub struct Actions {
    pub store: Vec<ActionType>,
}

impl Actions {
    pub fn none() -> Actions {
        Actions { store: vec![] }
    }

    pub fn has_pending_actions(&self) -> bool {
        for item in self.store.iter() {
            match item {
                ActionType::AppendSubGroup(_)
                | ActionType::AppendGroup(_)
                | ActionType::AppendItem(_, _, _)
                | ActionType::NewBlock(_) => return true,
                ActionType::NewModal(_) => return true,
                ActionType::UpdateActionItemRequest(data) => {
                    match data.action_status.clone().unwrap() {
                        ActionItemStatus::Success(_) => continue,
                        _ => return true,
                    }
                }
            }
        }
        false
    }

    pub fn append(&mut self, actions: &mut Actions) {
        self.store.append(&mut actions.store);
    }

    pub fn push_modal(&mut self, block: Block) {
        self.store.push(ActionType::NewModal(block));
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
        if !action_items.is_empty() {
            self.store.push(ActionType::AppendSubGroup(ActionSubGroup {
                action_items,
                allow_batch_completion: false,
            }));
        }
    }
    pub fn push_action_item_update(&mut self, update: ActionItemRequestUpdate) {
        self.store.push(ActionType::UpdateActionItemRequest(update))
    }

    pub fn new_panel(title: &str, description: &str) -> Actions {
        let store = vec![ActionType::NewBlock(ActionPanelData {
            title: title.to_string(),
            description: description.to_string(), //todo, make optional
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

    pub fn append_item(
        item: ActionItemRequest,
        group_title: Option<&str>,
        panel_title: Option<&str>,
    ) -> Actions {
        let store = vec![ActionType::AppendItem(
            item,
            group_title.map(|t| t.to_string()),
            panel_title.map(|t| t.to_string()),
        )];
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
                ActionType::AppendItem(item, _, _) => {
                    new_action_item_requests.push(item);
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
                ActionType::NewModal(data) => {
                    for group in data.panel.expect_modal_panel().groups.iter() {
                        for subgroup in group.sub_groups.iter() {
                            for item in subgroup.action_items.iter() {
                                new_action_item_requests.push(item);
                            }
                        }
                    }
                }
                ActionType::UpdateActionItemRequest(_) => continue,
            }
        }
        new_action_item_requests
    }

    pub fn compile_actions_to_block_events(
        &mut self,
        action_item_requests: &BTreeMap<Uuid, ActionItemRequest>,
    ) -> Vec<BlockEvent> {
        let mut blocks = vec![];
        let mut current_panel_data = ActionPanelData {
            title: "".to_string(),
            description: "".to_string(),
            groups: vec![],
        };
        let mut index = 0;
        let mut current_modal: Option<Block> = None;
        let mut updates = vec![];
        for item in self.store.iter_mut() {
            match item {
                ActionType::AppendItem(item, group_title, panel_title) => {
                    item.index = index;
                    index += 1;
                    match current_modal {
                        None => {
                            if current_panel_data.groups.len() > 0 {
                                let Some(group) = current_panel_data.groups.last_mut() else {
                                    continue;
                                };
                                if group.sub_groups.len() > 0 {
                                    // if the last sub group has no action items, don't push a new group, just replace it
                                    let Some(sub_group) = group.sub_groups.last_mut() else {
                                        continue;
                                    };
                                    if sub_group.action_items.is_empty() {
                                        *sub_group = ActionSubGroup {
                                            action_items: vec![item.clone()],
                                            allow_batch_completion: true,
                                        };
                                        continue;
                                    }
                                }
                                group
                                    .sub_groups
                                    .last_mut()
                                    .unwrap()
                                    .action_items
                                    .push(item.clone());
                            } else {
                                current_panel_data.groups.push(ActionGroup {
                                    title: group_title.as_ref().unwrap_or(&"".into()).into(),
                                    sub_groups: vec![ActionSubGroup {
                                        action_items: vec![item.clone()],
                                        allow_batch_completion: true,
                                    }],
                                });
                            }
                            if let Some(panel_title) = panel_title {
                                current_panel_data.title = panel_title.to_string();
                            }
                        }
                        Some(ref mut modal) => {
                            if modal.panel.expect_modal_panel().groups.len() > 0 {
                                let Some(group) =
                                    modal.panel.expect_modal_panel_mut().groups.last_mut()
                                else {
                                    continue;
                                };
                                if group.sub_groups.len() > 0 {
                                    // if the last sub group has no action items, don't push a new group, just replace it
                                    let Some(sub_group) = group.sub_groups.last_mut() else {
                                        continue;
                                    };
                                    if sub_group.action_items.is_empty() {
                                        *sub_group = ActionSubGroup {
                                            action_items: vec![item.clone()],
                                            allow_batch_completion: true,
                                        };
                                        continue;
                                    }
                                }
                                group.sub_groups.push(ActionSubGroup {
                                    action_items: vec![item.clone()],
                                    allow_batch_completion: true,
                                });
                            } else {
                                modal
                                    .panel
                                    .expect_modal_panel_mut()
                                    .groups
                                    .push(ActionGroup {
                                        title: group_title.as_ref().unwrap_or(&"".into()).into(),
                                        sub_groups: vec![ActionSubGroup {
                                            action_items: vec![item.clone()],
                                            allow_batch_completion: true,
                                        }],
                                    });
                            }
                            if let ActionItemRequestType::ValidateModal = item.action_type {
                                blocks.push(BlockEvent::Modal(modal.clone()));
                                current_modal = None;
                            }
                        }
                    }
                }
                ActionType::AppendSubGroup(data) => {
                    for item in data.action_items.iter_mut() {
                        item.index = index;
                        index += 1;
                    }
                    match current_modal {
                        None => {
                            if current_panel_data.groups.len() > 0 {
                                let Some(group) = current_panel_data.groups.last_mut() else {
                                    continue;
                                };
                                if group.sub_groups.len() > 0 {
                                    // if the last sub group has no action items, don't push a new group, just replace it
                                    let Some(sub_group) = group.sub_groups.last_mut() else {
                                        continue;
                                    };
                                    if sub_group.action_items.is_empty() {
                                        *sub_group = data.clone();
                                        continue;
                                    }
                                }
                                group.sub_groups.push(data.clone());
                            } else {
                                current_panel_data.groups.push(ActionGroup {
                                    title: "".to_string(),
                                    sub_groups: vec![data.clone()],
                                });
                            }
                        }
                        Some(ref mut modal) => {
                            if modal.panel.expect_modal_panel().groups.len() > 0 {
                                let Some(group) =
                                    modal.panel.expect_modal_panel_mut().groups.last_mut()
                                else {
                                    continue;
                                };
                                if group.sub_groups.len() > 0 {
                                    // if the last sub group has no action items, don't push a new group, just replace it
                                    let Some(sub_group) = group.sub_groups.last_mut() else {
                                        continue;
                                    };
                                    if sub_group.action_items.is_empty() {
                                        *sub_group = data.clone();
                                        continue;
                                    }
                                }
                                group.sub_groups.push(data.clone());
                            } else {
                                modal
                                    .panel
                                    .expect_modal_panel_mut()
                                    .groups
                                    .push(ActionGroup {
                                        title: "".to_string(),
                                        sub_groups: vec![data.clone()],
                                    });
                            }
                            if data.contains_validate_modal_item() {
                                blocks.push(BlockEvent::Modal(modal.clone()));
                                current_modal = None;
                            }
                        }
                    }
                }
                ActionType::AppendGroup(data) => {
                    for subgroup in data.sub_groups.iter_mut() {
                        for item in subgroup.action_items.iter_mut() {
                            item.index = index;
                            index += 1;
                        }
                    }
                    match current_modal {
                        None => {
                            current_panel_data.groups.push(data.clone());
                        }
                        Some(ref mut modal) => {
                            modal
                                .panel
                                .expect_modal_panel_mut()
                                .groups
                                .push(data.clone());
                            if data.contains_validate_modal_item() {
                                blocks.push(BlockEvent::Modal(modal.clone()));
                                current_modal = None;
                            }
                        }
                    }
                }
                ActionType::NewBlock(data) => {
                    if current_panel_data.groups.len() > 1 {
                        blocks.push(BlockEvent::Action(Block {
                            uuid: Uuid::new_v4(),
                            panel: Panel::ActionPanel(current_panel_data.clone()),
                            visible: true,
                        }));
                    }
                    current_panel_data = data.clone();
                }
                ActionType::NewModal(data) => {
                    current_modal = Some(data.clone());
                }
                ActionType::UpdateActionItemRequest(data) => {
                    if let Some(update) = data.normalize(&action_item_requests) {
                        updates.push(update);
                    }
                }
            }
        }
        if !updates.is_empty() {
            blocks.push(BlockEvent::UpdateActionItems(updates));
        }
        blocks.push(BlockEvent::Action(Block {
            uuid: Uuid::new_v4(),
            panel: Panel::ActionPanel(current_panel_data.clone()),
            visible: true,
        }));
        blocks
    }

    pub fn compile_actions_to_item_updates(&self) -> Vec<ActionItemRequestUpdate> {
        let mut updates = vec![];

        for item in self.store.iter() {
            match item {
                ActionType::AppendSubGroup(_) => {}
                ActionType::AppendGroup(_) => {}
                ActionType::AppendItem(_, _, _) => {}
                ActionType::NewBlock(_) | ActionType::NewModal(_) => {}
                ActionType::UpdateActionItemRequest(update) => updates.push(update.clone()),
            }
        }

        updates
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ActionItemRequestType {
    ReviewInput(ReviewInputRequest),
    ProvideInput(ProvideInputRequest),
    PickInputOption(PickInputOptionRequest),
    ProvidePublicKey(ProvidePublicKeyRequest),
    ProvideSignedTransaction(ProvideSignedTransactionRequest),
    ProvideSignedMessage(ProvideSignedMessageRequest),
    DisplayOutput(DisplayOutputRequest),
    DisplayErrorLog(DisplayErrorLogRequest),
    OpenModal(OpenModalData),
    ValidateBlock,
    ValidateModal,
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
pub struct ReviewInputRequest {
    pub input_name: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvideInputRequest {
    pub default_value: Option<Value>,
    pub input_name: String,
    pub typing: Type,
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
pub struct DisplayErrorLogRequest {
    pub diagnostic: Diagnostic,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickInputOptionRequest {
    pub options: Vec<InputOption>,
    pub selected: InputOption,
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
    pub signer_uuid: Uuid,
    pub payload: Value,
    pub namespace: String,
    pub network_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvideSignedMessageRequest {
    pub check_expectation_action_uuid: Option<Uuid>,
    pub signer_uuid: Uuid,
    pub message: Value,
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
    ProvideSignedMessage(ProvideSignedMessageResponse),
    ProvideSignedTransaction(ProvideSignedTransactionResponse),
    ValidateBlock,
    ValidateModal,
}

impl ActionItemResponseType {
    pub fn is_validate_panel(&self) -> bool {
        match &self {
            ActionItemResponseType::ValidateBlock => true,
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
pub struct ProvideSignedMessageResponse {
    pub signed_message_bytes: String,
    pub signer_uuid: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvideSignedTransactionResponse {
    pub signed_transaction_bytes: String,
    pub signer_uuid: Uuid,
}
