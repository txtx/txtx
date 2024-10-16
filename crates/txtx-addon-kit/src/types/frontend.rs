use std::{borrow::BorrowMut, collections::BTreeMap, fmt::Display};

use super::{
    block_id::BlockId,
    diagnostics::Diagnostic,
    types::{Type, Value},
    ConstructDid, Did,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub uuid: Uuid,
    #[serde(flatten)]
    pub panel: Panel,
    pub visible: bool,
}

impl Block {
    pub fn new(uuid: &Uuid, panel: Panel) -> Self {
        Block { uuid: uuid.clone(), panel, visible: true }
    }

    pub fn apply_action_item_updates(&mut self, update: NormalizedActionItemRequestUpdate) -> bool {
        let mut did_update = false;
        match self.panel.borrow_mut() {
            Panel::ActionPanel(panel) => {
                for group in panel.groups.iter_mut() {
                    let group_did_update = group.apply_action_item_updates(&update);
                    if group_did_update {
                        did_update = true;
                    }
                }
            }
            Panel::ModalPanel(panel) => {
                for group in panel.groups.iter_mut() {
                    let group_did_update = group.apply_action_item_updates(&update);
                    if group_did_update {
                        did_update = true;
                    }
                }
            }
            _ => {}
        };
        did_update
    }

    pub fn update_progress_bar_status(
        &mut self,
        construct_did: &ConstructDid,
        new_status: &ProgressBarStatus,
    ) {
        match self.panel.borrow_mut() {
            Panel::ProgressBar(progress_bar) => {
                let mut construct_status_found = false;
                for construct_statuses in progress_bar.iter_mut() {
                    if &construct_statuses.construct_did == construct_did {
                        construct_statuses.push_status(new_status);
                        construct_status_found = true;
                    }
                }
                if !construct_status_found {
                    progress_bar.push(ConstructProgressBarStatuses {
                        construct_did: construct_did.clone(),
                        statuses: vec![new_status.clone()],
                    })
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Note: though the `action_status` field is optional, it is required for many functions. I kep it like this
/// because I like the `.set_status` pattern :-)
pub struct NormalizedActionItemRequestUpdate {
    pub id: BlockId,
    pub action_status: Option<ActionItemStatus>,
    pub action_type: Option<ActionItemRequestType>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionItemRequestUpdate {
    pub id: ActionItemRequestUpdateIdentifier,
    pub action_status: Option<ActionItemStatus>,
    pub action_type: Option<ActionItemRequestType>,
}

#[derive(Debug, Clone, Serialize)]
pub enum ActionItemRequestUpdateIdentifier {
    Id(BlockId),
    ConstructDidWithKey((ConstructDid, String)),
}

impl ActionItemRequestUpdate {
    pub fn from_id(id: &BlockId) -> Self {
        ActionItemRequestUpdate {
            id: ActionItemRequestUpdateIdentifier::Id(id.clone()),
            action_status: None,
            action_type: None,
        }
    }
    pub fn from_context(construct_did: &ConstructDid, internal_key: &str) -> Self {
        ActionItemRequestUpdate {
            id: ActionItemRequestUpdateIdentifier::ConstructDidWithKey((
                construct_did.clone(),
                internal_key.to_string(),
            )),
            action_status: None,
            action_type: None,
        }
    }
    ///
    /// Compares `new_item` and `existing_item`, returning an `ActionItemRequestUpdate` if
    /// the ids are the same and either the mutable properties of the type or that status have been updated.
    ///
    pub fn from_diff(
        new_item: &ActionItemRequest,
        existing_item: &ActionItemRequest,
    ) -> Option<Self> {
        let id_match = new_item.id == existing_item.id;
        let status_match = new_item.action_status == existing_item.action_status;
        let type_diff = ActionItemRequestType::diff_mutable_properties(
            &new_item.action_type,
            &existing_item.action_type,
        );
        if !id_match || (status_match && type_diff.is_none()) {
            return None;
        }
        let mut update = ActionItemRequestUpdate::from_id(&new_item.id);
        if !status_match {
            update.set_status(new_item.action_status.clone());
        }
        if let Some(new_type) = type_diff {
            update.set_type(new_type);
        }
        Some(update)
    }

    pub fn set_status(&mut self, new_status: ActionItemStatus) -> Self {
        self.action_status = Some(new_status);
        self.clone()
    }

    pub fn set_type(&mut self, new_type: ActionItemRequestType) -> Self {
        self.action_type = Some(new_type);
        self.clone()
    }

    pub fn normalize(
        &self,
        action_item_requests: &BTreeMap<BlockId, ActionItemRequest>,
    ) -> Option<NormalizedActionItemRequestUpdate> {
        for (_, action) in action_item_requests.iter() {
            match &self.id {
                ActionItemRequestUpdateIdentifier::Id(id) => {
                    if action.id.eq(id) {
                        return Some(NormalizedActionItemRequestUpdate {
                            id: id.clone(),
                            action_status: self.action_status.clone(),
                            action_type: self.action_type.clone(),
                        });
                    }
                }
                ActionItemRequestUpdateIdentifier::ConstructDidWithKey((
                    construct_did,
                    internal_key,
                )) => {
                    let Some(ref action_construct_did) = action.construct_did else {
                        continue;
                    };
                    if action_construct_did.eq(construct_did)
                        && action.internal_key.eq(internal_key)
                    {
                        return Some(NormalizedActionItemRequestUpdate {
                            id: action.id.clone(),
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
                            writeln!(f, "          title: {:?}", item.title)?;
                            writeln!(f, "          consctruct: {:?}", item.construct_did)?;
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
                            writeln!(f, "          title: {:?}", item.title)?;
                            writeln!(f, "          consctruct: {:?}", item.construct_did)?;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "panel")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionPanelData {
    pub title: String,
    pub description: String,
    pub groups: Vec<ActionGroup>,
}

impl ActionPanelData {
    pub fn compile_actions_to_item_updates(
        &self,
        action_item_requests: &BTreeMap<BlockId, ActionItemRequest>,
    ) -> Vec<ActionItemRequestUpdate> {
        let mut updates = vec![];
        for group in self.groups.iter() {
            let mut group_updates = group.compile_actions_to_item_updates(&action_item_requests);
            updates.append(&mut group_updates);
        }
        updates
    }

    pub fn filter_existing_action_items(
        &mut self,
        existing_requests: &Vec<&mut ActionItemRequest>,
    ) -> &mut Self {
        let mut group_idx_to_remove = vec![];
        for (i, group) in self.groups.iter_mut().enumerate() {
            group.filter_existing_action_items(&existing_requests);
            if group.sub_groups.is_empty() {
                group_idx_to_remove.push(i);
            }
        }
        group_idx_to_remove.iter().rev().for_each(|i| {
            self.groups.remove(*i);
        });
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstructProgressBarStatuses {
    pub construct_did: ConstructDid,
    pub statuses: Vec<ProgressBarStatus>,
}

impl ConstructProgressBarStatuses {
    pub fn new(construct_did: &ConstructDid) -> Self {
        ConstructProgressBarStatuses { construct_did: construct_did.clone(), statuses: vec![] }
    }
    pub fn push_status(&mut self, new_status: &ProgressBarStatus) {
        self.statuses.push(new_status.clone());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressBarStatus {
    pub status: String,
    pub status_color: ProgressBarStatusColor,
    pub message: String,
    pub diagnostic: Option<Diagnostic>,
}

impl ProgressBarStatus {
    pub fn new_msg(status_color: ProgressBarStatusColor, status: &str, message: &str) -> Self {
        ProgressBarStatus {
            status: status.to_string(),
            status_color,
            message: message.to_string(),
            diagnostic: None,
        }
    }
    pub fn new_err(status: &str, message: &str, diag: &Diagnostic) -> Self {
        ProgressBarStatus {
            status: status.to_string(),
            status_color: ProgressBarStatusColor::Red,
            message: message.to_string(),
            diagnostic: Some(diag.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressBarStatusColor {
    Green,
    Yellow,
    Red,
}
impl ProgressBarStatusColor {
    pub fn to_string(&self) -> String {
        match &self {
            ProgressBarStatusColor::Green => "Green".into(),
            ProgressBarStatusColor::Yellow => "Yellow".into(),
            ProgressBarStatusColor::Red => "Red".into(),
        }
    }
}

const PROGRESS_SYMBOLS: [&str; 8] = ["|", "/", "-", "\\", "|", "/", "-", "\\"];
#[derive(Clone, Debug)]
pub struct ProgressSymbol {
    current: usize,
}
impl ProgressSymbol {
    pub fn new() -> Self {
        ProgressSymbol { current: 0 }
    }
    pub fn next(&mut self) -> &str {
        self.current = (self.current + 1) % PROGRESS_SYMBOLS.len();
        PROGRESS_SYMBOLS[self.current]
    }
    pub fn pending(&mut self) -> String {
        format!("Pending {}", self.next())
    }
}

#[derive(Debug, Clone)]
pub struct StatusUpdater {
    status_update: ProgressBarStatusUpdate,
    tx: channel::Sender<BlockEvent>,
    progress: ProgressSymbol,
}
impl StatusUpdater {
    pub fn new(
        background_tasks_uuid: &Uuid,
        construct_did: &ConstructDid,
        tx: &channel::Sender<BlockEvent>,
    ) -> Self {
        let mut progress = ProgressSymbol::new();
        let initial_status = ProgressBarStatusUpdate::new(
            &background_tasks_uuid,
            &construct_did,
            &&ProgressBarStatus::new_msg(ProgressBarStatusColor::Yellow, &progress.pending(), ""),
        );
        StatusUpdater { status_update: initial_status, tx: tx.clone(), progress }
    }
    pub fn propagate_pending_status(&mut self, new_status_msg: &str) {
        self.status_update.update_status(&ProgressBarStatus::new_msg(
            ProgressBarStatusColor::Yellow,
            &self.progress.pending(),
            new_status_msg,
        ));
        let _ = self.tx.send(BlockEvent::UpdateProgressBarStatus(self.status_update.clone()));
    }

    pub fn propagate_status(&mut self, new_status: ProgressBarStatus) {
        self.status_update.update_status(&new_status);
        let _ = self.tx.send(BlockEvent::UpdateProgressBarStatus(self.status_update.clone()));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressBarStatusUpdate {
    pub progress_bar_uuid: Uuid,
    pub construct_did: ConstructDid,
    pub new_status: ProgressBarStatus,
}

impl ProgressBarStatusUpdate {
    pub fn new(
        progress_bar_uuid: &Uuid,
        construct_did: &ConstructDid,
        new_status: &ProgressBarStatus,
    ) -> Self {
        ProgressBarStatusUpdate {
            progress_bar_uuid: progress_bar_uuid.clone(),
            construct_did: construct_did.clone(),
            new_status: new_status.clone(),
        }
    }
    pub fn update_status(&mut self, new_status: &ProgressBarStatus) {
        self.new_status = new_status.clone();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressBarVisibilityUpdate {
    pub progress_bar_uuid: Uuid,
    pub visible: bool,
}
impl ProgressBarVisibilityUpdate {
    pub fn new(progress_bar_uuid: &Uuid, visible: bool) -> Self {
        ProgressBarVisibilityUpdate { progress_bar_uuid: progress_bar_uuid.clone(), visible }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModalPanelData {
    pub title: String,
    pub description: String,
    pub groups: Vec<ActionGroup>,
}

impl ModalPanelData {
    pub fn compile_actions_to_item_updates(
        &self,
        action_item_requests: &BTreeMap<BlockId, ActionItemRequest>,
    ) -> Vec<ActionItemRequestUpdate> {
        let mut updates = vec![];
        for group in self.groups.iter() {
            let mut group_updates = group.compile_actions_to_item_updates(&action_item_requests);
            updates.append(&mut group_updates);
        }
        updates
    }

    pub fn filter_existing_action_items(
        &mut self,
        existing_requests: &Vec<&mut ActionItemRequest>,
    ) -> &mut Self {
        let mut group_idx_to_remove = vec![];
        for (i, group) in self.groups.iter_mut().enumerate() {
            group.filter_existing_action_items(&existing_requests);
            if group.sub_groups.is_empty() {
                group_idx_to_remove.push(i);
            }
        }
        group_idx_to_remove.iter().rev().for_each(|i| {
            self.groups.remove(*i);
        });
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            let mut action = ActionItemRequest::new(
                &None,
                "",
                None,
                ActionItemStatus::Error(diag.clone()),
                ActionItemRequestType::DisplayErrorLog(DisplayErrorLogRequest {
                    diagnostic: diag.clone(),
                }),
                "diagnostic",
            );
            action.index = (i + 1) as u16;
            diag_actions.push(action);
        }
        ErrorPanelData {
            title: "EXECUTION ERROR".into(),
            description: "Review the following execution errors and restart the runbook.".into(),
            groups: vec![ActionGroup {
                title: "".into(),
                sub_groups: vec![ActionSubGroup {
                    title: None,
                    action_items: diag_actions,
                    allow_batch_completion: false,
                }],
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionGroup {
    pub title: String,
    pub sub_groups: Vec<ActionSubGroup>,
}

impl ActionGroup {
    pub fn new(title: &str, sub_groups: Vec<ActionSubGroup>) -> Self {
        ActionGroup { title: title.to_string(), sub_groups }
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

    pub fn apply_action_item_updates(
        &mut self,
        update: &NormalizedActionItemRequestUpdate,
    ) -> bool {
        let mut did_update = false;
        for sub_group in self.sub_groups.iter_mut() {
            for action in sub_group.action_items.iter_mut() {
                if action.id == update.id {
                    if let Some(action_status) = update.action_status.clone() {
                        if action.action_status != action_status {
                            action.action_status = action_status;
                            did_update = true;
                        }
                    }
                    if let Some(action_type) = update.action_type.clone() {
                        if action.action_type != action_type {
                            action.action_type = action_type;
                            did_update = true;
                        }
                    }
                }
            }
        }
        did_update
    }

    pub fn compile_actions_to_item_updates(
        &self,
        action_item_requests: &BTreeMap<BlockId, ActionItemRequest>,
    ) -> Vec<ActionItemRequestUpdate> {
        let mut updates = vec![];
        for sub_group in self.sub_groups.iter() {
            let mut sub_group_updates =
                sub_group.compile_actions_to_item_updates(&action_item_requests);
            updates.append(&mut sub_group_updates);
        }
        updates
    }

    pub fn filter_existing_action_items(
        &mut self,
        existing_requests: &Vec<&mut ActionItemRequest>,
    ) -> &mut Self {
        let mut sub_group_idx_to_remove = vec![];
        for (i, sub_group) in self.sub_groups.iter_mut().enumerate() {
            sub_group.filter_existing_action_items(&existing_requests);
            if sub_group.action_items.is_empty() {
                sub_group_idx_to_remove.push(i);
            }
        }
        sub_group_idx_to_remove.iter().rev().for_each(|i| {
            self.sub_groups.remove(*i);
        });
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionSubGroup {
    pub title: Option<String>,
    pub action_items: Vec<ActionItemRequest>,
    pub allow_batch_completion: bool,
}

impl ActionSubGroup {
    pub fn new(
        title: Option<String>,
        action_items: Vec<ActionItemRequest>,
        allow_batch_completion: bool,
    ) -> Self {
        ActionSubGroup { title, action_items, allow_batch_completion }
    }

    pub fn contains_validate_modal_item(&self) -> bool {
        for item in self.action_items.iter() {
            if let ActionItemRequestType::ValidateModal = item.action_type {
                return true;
            }
        }
        false
    }

    pub fn compile_actions_to_item_updates(
        &self,
        action_item_requests: &BTreeMap<BlockId, ActionItemRequest>,
    ) -> Vec<ActionItemRequestUpdate> {
        let mut updates = vec![];
        for new_item in self.action_items.iter() {
            if let Some(existing_item) = action_item_requests.get(&new_item.id) {
                if let Some(update) = ActionItemRequestUpdate::from_diff(new_item, existing_item) {
                    updates.push(update);
                };
            };
        }
        updates
    }

    pub fn filter_existing_action_items(
        &mut self,
        existing_requests: &Vec<&mut ActionItemRequest>,
    ) -> &mut Self {
        let mut action_item_idx_to_remove = vec![];
        for (i, new_item) in self.action_items.iter().enumerate() {
            for existing_item in existing_requests.iter() {
                if existing_item.id.eq(&new_item.id) {
                    if let None = ActionItemRequestUpdate::from_diff(new_item, existing_item) {
                        action_item_idx_to_remove.push(i);
                    };
                }
            }
        }
        action_item_idx_to_remove.iter().rev().for_each(|i| {
            self.action_items.remove(*i);
        });
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionItemRequest {
    pub id: BlockId,
    pub construct_did: Option<ConstructDid>,
    pub index: u16,
    pub title: String,
    pub description: Option<String>,
    pub action_status: ActionItemStatus,
    pub action_type: ActionItemRequestType,
    pub internal_key: String,
}

impl ActionItemRequest {
    pub fn new(
        construct_did: &Option<ConstructDid>,
        title: &str,
        description: Option<String>,
        action_status: ActionItemStatus,
        action_type: ActionItemRequestType,
        internal_key: &str,
    ) -> Self {
        let data = format!(
            "{}-{}-{}-{}-{}",
            title,
            description.clone().unwrap_or("".into()),
            internal_key,
            construct_did.as_ref().and_then(|did| Some(did.to_string())).unwrap_or("".into()),
            action_type.get_block_id_string()
        );
        let id = BlockId::new(data.as_bytes());
        ActionItemRequest {
            id,
            construct_did: construct_did.clone(),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", content = "data")]
pub enum ActionItemStatus {
    Blocked,
    Todo,
    Success(Option<String>),
    InProgress(String),
    Error(Diagnostic),
    Warning(Diagnostic),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConstructData {
    pub construct_did: ConstructDid,
    pub action_item_update: ActionItemRequestUpdate,
    pub internal_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
                | ActionType::AppendItem(_, _, _) => return true,
                ActionType::NewBlock(_) => return true,
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
                title: None,
                action_items,
                allow_batch_completion: false,
            }],
            title: title.to_string(),
        }));
    }

    pub fn push_sub_group(&mut self, title: Option<String>, action_items: Vec<ActionItemRequest>) {
        if !action_items.is_empty() {
            self.store.push(ActionType::AppendSubGroup(ActionSubGroup {
                title,
                action_items,
                allow_batch_completion: false,
            }));
        }
    }
    pub fn push_action_item_update(&mut self, update: ActionItemRequestUpdate) {
        self.store.push(ActionType::UpdateActionItemRequest(update))
    }

    pub fn push_panel(&mut self, title: &str, description: &str) {
        self.store.push(ActionType::NewBlock(ActionPanelData {
            title: title.to_string(),
            description: description.to_string(), //todo, make optional
            groups: vec![],
        }))
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
                title: None,
                action_items,
                allow_batch_completion: false,
            }],
            title: title.to_string(),
        })];
        Actions { store }
    }

    pub fn new_sub_group_of_items(
        title: Option<String>,
        action_items: Vec<ActionItemRequest>,
    ) -> Actions {
        let store = vec![ActionType::AppendSubGroup(ActionSubGroup {
            title,
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
        action_item_requests: &BTreeMap<BlockId, ActionItemRequest>,
    ) -> Vec<BlockEvent> {
        let mut blocks = vec![];
        let mut current_panel_data =
            ActionPanelData { title: "".to_string(), description: "".to_string(), groups: vec![] };
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
                                            title: None,
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
                                        title: None,
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
                                            title: None,
                                            action_items: vec![item.clone()],
                                            allow_batch_completion: true,
                                        };
                                        continue;
                                    }
                                }
                                group.sub_groups.push(ActionSubGroup {
                                    title: None,
                                    action_items: vec![item.clone()],
                                    allow_batch_completion: true,
                                });
                            } else {
                                modal.panel.expect_modal_panel_mut().groups.push(ActionGroup {
                                    title: group_title.as_ref().unwrap_or(&"".into()).into(),
                                    sub_groups: vec![ActionSubGroup {
                                        title: None,
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
                                modal.panel.expect_modal_panel_mut().groups.push(ActionGroup {
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
                            modal.panel.expect_modal_panel_mut().groups.push(data.clone());
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

    pub fn compile_actions_to_item_updates(
        &self,
        action_item_requests: &BTreeMap<BlockId, ActionItemRequest>,
    ) -> Vec<ActionItemRequestUpdate> {
        let mut updates = vec![];

        for item in self.store.iter() {
            match item {
                ActionType::AppendSubGroup(sub_group) => {
                    let mut sub_group_updates =
                        sub_group.compile_actions_to_item_updates(&action_item_requests);
                    updates.append(&mut sub_group_updates);
                }
                ActionType::AppendGroup(group) => {
                    let mut group_updates =
                        group.compile_actions_to_item_updates(&action_item_requests);
                    updates.append(&mut group_updates);
                }
                ActionType::AppendItem(new_item, _, _) => {
                    if let Some(existing_item) = action_item_requests.get(&new_item.id) {
                        if let Some(update) =
                            ActionItemRequestUpdate::from_diff(new_item, existing_item)
                        {
                            updates.push(update);
                        };
                    };
                }
                ActionType::NewBlock(action_panel_data) => {
                    let mut block_updates =
                        action_panel_data.compile_actions_to_item_updates(&action_item_requests);
                    updates.append(&mut block_updates);
                }
                ActionType::NewModal(modal) => match &modal.panel {
                    Panel::ActionPanel(action_panel_data) => {
                        let mut block_updates = action_panel_data
                            .compile_actions_to_item_updates(&action_item_requests);
                        updates.append(&mut block_updates);
                    }
                    Panel::ModalPanel(modal_panel_data) => {
                        let mut block_updates =
                            modal_panel_data.compile_actions_to_item_updates(&action_item_requests);
                        updates.append(&mut block_updates);
                    }
                    _ => {}
                },
                ActionType::UpdateActionItemRequest(update) => updates.push(update.clone()),
            }
        }

        updates
    }

    pub fn filter_existing_action_items(
        &mut self,
        existing_requests: &Option<&Vec<&mut ActionItemRequest>>,
    ) -> &mut Self {
        let Some(existing_requests) = existing_requests else {
            return self;
        };

        let mut idx_to_remove = vec![];
        for (i, item) in self.store.iter_mut().enumerate() {
            match item {
                ActionType::UpdateActionItemRequest(_) => {}
                ActionType::AppendSubGroup(sub_group) => {
                    sub_group.filter_existing_action_items(&existing_requests);
                    if sub_group.action_items.is_empty() {
                        idx_to_remove.push(i);
                    }
                }
                ActionType::AppendGroup(group) => {
                    group.filter_existing_action_items(&existing_requests);
                    if group.sub_groups.is_empty() {
                        idx_to_remove.push(i);
                    }
                }
                ActionType::AppendItem(new_item, _, _) => {
                    for existing_item in existing_requests.iter() {
                        if existing_item.id.eq(&new_item.id) {
                            if let None =
                                ActionItemRequestUpdate::from_diff(new_item, existing_item)
                            {
                                idx_to_remove.push(i);
                            };
                        }
                    }
                }
                ActionType::NewBlock(block) => {
                    block.filter_existing_action_items(&existing_requests);
                    if block.groups.is_empty() {
                        idx_to_remove.push(i);
                    }
                }
                ActionType::NewModal(_) => {}
            }
        }
        idx_to_remove.iter().rev().for_each(|i| {
            self.store.remove(*i);
        });

        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum ActionItemRequestType {
    ReviewInput(ReviewInputRequest),
    ProvideInput(ProvideInputRequest),
    PickInputOption(PickInputOptionRequest),
    ProvidePublicKey(ProvidePublicKeyRequest),
    ProvideSignedTransaction(ProvideSignedTransactionRequest),
    ProvideSignedMessage(ProvideSignedMessageRequest),
    SendTransaction(SendTransactionRequest),
    DisplayOutput(DisplayOutputRequest),
    DisplayErrorLog(DisplayErrorLogRequest),
    OpenModal(OpenModalData),
    ValidateBlock(ValidateBlockData),
    ValidateModal,
}

impl ActionItemRequestType {
    pub fn as_review_input(&self) -> Option<&ReviewInputRequest> {
        match &self {
            ActionItemRequestType::ReviewInput(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_provide_input(&self) -> Option<&ProvideInputRequest> {
        match &self {
            ActionItemRequestType::ProvideInput(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_pick_input(&self) -> Option<&PickInputOptionRequest> {
        match &self {
            ActionItemRequestType::PickInputOption(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_provide_public_key(&self) -> Option<&ProvidePublicKeyRequest> {
        match &self {
            ActionItemRequestType::ProvidePublicKey(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_provide_signed_tx(&self) -> Option<&ProvideSignedTransactionRequest> {
        match &self {
            ActionItemRequestType::ProvideSignedTransaction(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_sign_tx(&self) -> Option<&SendTransactionRequest> {
        match &self {
            ActionItemRequestType::SendTransaction(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_provide_signed_msg(&self) -> Option<&ProvideSignedMessageRequest> {
        match &self {
            ActionItemRequestType::ProvideSignedMessage(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_display_output(&self) -> Option<&DisplayOutputRequest> {
        match &self {
            ActionItemRequestType::DisplayOutput(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_display_err(&self) -> Option<&DisplayErrorLogRequest> {
        match &self {
            ActionItemRequestType::DisplayErrorLog(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_open_modal(&self) -> Option<&OpenModalData> {
        match &self {
            ActionItemRequestType::OpenModal(value) => Some(value),
            _ => None,
        }
    }

    ///
    /// Serialize the immutable properties of the type to be used for an `ActionItemRequest`'s `BlockId`.
    ///
    pub fn get_block_id_string(&self) -> String {
        match self {
            ActionItemRequestType::ReviewInput(val) => {
                format!("ReviewInput({}-{})", val.input_name, val.force_execution)
            }
            ActionItemRequestType::ProvideInput(val) => format!(
                "ProvideInput({}-{})",
                val.input_name,
                serde_json::to_string(&val.typing).unwrap() //todo: make to_string prop?
            ),
            ActionItemRequestType::PickInputOption(_) => format!("PickInputOption"),
            ActionItemRequestType::ProvidePublicKey(val) => format!(
                "ProvidePublicKey({}-{}-{})",
                val.check_expectation_action_uuid
                    .as_ref()
                    .and_then(|u| Some(u.to_string()))
                    .unwrap_or("None".to_string()),
                val.namespace,
                val.network_id
            ),
            ActionItemRequestType::ProvideSignedTransaction(val) => {
                format!(
                    "ProvideSignedTransaction({}-{}-{}-{})",
                    val.check_expectation_action_uuid
                        .as_ref()
                        .and_then(|u| Some(u.to_string()))
                        .unwrap_or("None".to_string()),
                    val.signer_uuid.to_string(),
                    val.namespace,
                    val.network_id
                )
            }
            ActionItemRequestType::SendTransaction(val) => {
                format!(
                    "SendTransaction({}-{}-{}-{})",
                    val.check_expectation_action_uuid
                        .as_ref()
                        .and_then(|u| Some(u.to_string()))
                        .unwrap_or("None".to_string()),
                    val.signer_uuid.to_string(),
                    val.namespace,
                    val.network_id
                )
            }
            ActionItemRequestType::ProvideSignedMessage(val) => format!(
                "ProvideSignedMessage({}-{}-{}-{})",
                val.check_expectation_action_uuid
                    .as_ref()
                    .and_then(|u| Some(u.to_string()))
                    .unwrap_or("None".to_string()),
                val.signer_uuid.to_string(),
                val.namespace,
                val.network_id
            ),
            ActionItemRequestType::DisplayOutput(val) => format!(
                "DisplayOutput({}-{}-{})",
                val.name,
                val.description.clone().unwrap_or("None".to_string()),
                val.value.to_string()
            ),
            ActionItemRequestType::DisplayErrorLog(val) => {
                format!("DisplayErrorLog({})", val.diagnostic.to_string())
            }
            ActionItemRequestType::OpenModal(val) => {
                format!("OpenModal({}-{})", val.modal_uuid, val.title)
            }
            ActionItemRequestType::ValidateBlock(val) => {
                format!("ValidateBlock({})", val.internal_idx.to_string())
            }
            ActionItemRequestType::ValidateModal => format!("ValidateModal"),
        }
    }

    ///
    /// Compares all properties of `new_type` against `existing_type` to determine if any of the mutable properties
    /// of the type have been updated. Returns `Some(new_type)` if only mutable properties were updated, returns `None`
    /// otherwise.
    ///
    pub fn diff_mutable_properties(
        new_type: &ActionItemRequestType,
        existing_item: &ActionItemRequestType,
    ) -> Option<ActionItemRequestType> {
        match new_type {
            ActionItemRequestType::ReviewInput(new) => {
                let Some(existing) = existing_item.as_review_input() else {
                    unreachable!("cannot change action item request type")
                };
                if new.value != existing.value {
                    if new.input_name != existing.input_name {
                        unreachable!("cannot change review input request input_name")
                    }
                    if new.force_execution != existing.force_execution {
                        unreachable!("cannot change review input request force_execution")
                    }
                    Some(new_type.clone())
                } else {
                    None
                }
            }
            ActionItemRequestType::ProvideInput(new) => {
                let Some(existing) = existing_item.as_provide_input() else {
                    unreachable!("cannot change action item request type")
                };
                if new.default_value != existing.default_value {
                    if new.input_name != existing.input_name {
                        unreachable!("cannot change provide input request input_name")
                    }
                    if new.typing != existing.typing {
                        unreachable!("cannot change provide input request typing")
                    }
                    Some(new_type.clone())
                } else {
                    None
                }
            }
            ActionItemRequestType::PickInputOption(_) => {
                let Some(_) = existing_item.as_pick_input() else {
                    unreachable!("cannot change action item request type")
                };
                Some(new_type.clone())
            }
            ActionItemRequestType::ProvidePublicKey(new) => {
                let Some(existing) = existing_item.as_provide_public_key() else {
                    unreachable!("cannot change action item request type")
                };
                if new.message != existing.message {
                    if new.check_expectation_action_uuid != existing.check_expectation_action_uuid {
                        unreachable!("cannot change provide public key request check_expectation_action_uuid");
                    }
                    if new.namespace != existing.namespace {
                        unreachable!("cannot change provide public key request namespace");
                    }
                    if new.network_id != existing.network_id {
                        unreachable!("cannot change provide public key request network_id");
                    }
                    Some(new_type.clone())
                } else {
                    None
                }
            }
            ActionItemRequestType::ProvideSignedTransaction(new) => {
                let Some(existing) = existing_item.as_provide_signed_tx() else {
                    unreachable!("cannot change action item request type")
                };
                if new.payload != existing.payload || new.skippable != existing.skippable {
                    if new.check_expectation_action_uuid != existing.check_expectation_action_uuid {
                        unreachable!(
                            "cannot change provide signed tx request check_expectation_action_uuid"
                        );
                    }
                    if new.signer_uuid != existing.signer_uuid {
                        unreachable!("cannot change provide signed tx request signer_uuid");
                    }
                    if new.namespace != existing.namespace {
                        unreachable!("cannot change provide signed tx request namespace");
                    }
                    if new.network_id != existing.network_id {
                        unreachable!("cannot change provide signed tx request network_id");
                    }
                    if new.only_approval_needed != existing.only_approval_needed {
                        unreachable!(
                            "cannot change provide signed tx request only_approval_needed"
                        );
                    }
                    Some(new_type.clone())
                } else {
                    None
                }
            }
            ActionItemRequestType::SendTransaction(new) => {
                let Some(existing) = existing_item.as_sign_tx() else {
                    unreachable!("cannot change action item request type")
                };
                if new.payload != existing.payload {
                    if new.check_expectation_action_uuid != existing.check_expectation_action_uuid {
                        unreachable!(
                            "cannot change provide signed tx request check_expectation_action_uuid"
                        );
                    }
                    if new.signer_uuid != existing.signer_uuid {
                        unreachable!("cannot change provide signed tx request signer_uuid");
                    }
                    if new.namespace != existing.namespace {
                        unreachable!("cannot change provide signed tx request namespace");
                    }
                    if new.network_id != existing.network_id {
                        unreachable!("cannot change provide signed tx request network_id");
                    }
                    Some(new_type.clone())
                } else {
                    None
                }
            }
            ActionItemRequestType::ProvideSignedMessage(new) => {
                let Some(existing) = existing_item.as_provide_signed_msg() else {
                    unreachable!("cannot change action item request type")
                };
                if new.message != existing.message {
                    if new.check_expectation_action_uuid != existing.check_expectation_action_uuid {
                        unreachable!(
                            "cannot change provide signed msg request check_expectation_action_uuid"
                        );
                    }
                    if new.signer_uuid != existing.signer_uuid {
                        unreachable!("cannot change provide signed msg request signer_uuid");
                    }
                    if new.namespace != existing.namespace {
                        unreachable!("cannot change provide signed msg request namespace");
                    }
                    if new.network_id != existing.network_id {
                        unreachable!("cannot change provide signed msg request network_id");
                    }
                    Some(new_type.clone())
                } else {
                    None
                }
            }
            ActionItemRequestType::DisplayOutput(_) => None,
            ActionItemRequestType::DisplayErrorLog(_) => None,
            ActionItemRequestType::OpenModal(_) => None,
            ActionItemRequestType::ValidateBlock(_) => None,
            ActionItemRequestType::ValidateModal => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReviewInputRequest {
    pub input_name: String,
    pub value: Value,
    pub force_execution: bool,
}

impl ReviewInputRequest {
    pub fn new(input_name: &str, value: &Value) -> Self {
        ReviewInputRequest {
            input_name: input_name.to_string(),
            value: value.clone(),
            force_execution: false,
        }
    }
    pub fn force_execution(&mut self) -> &mut Self {
        self.force_execution = true;
        self
    }
    pub fn to_action_type(&self) -> ActionItemRequestType {
        ActionItemRequestType::ReviewInput(self.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProvideInputRequest {
    pub default_value: Option<Value>,
    pub input_name: String,
    pub typing: Type,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DisplayOutputRequest {
    pub name: String,
    pub description: Option<String>,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DisplayErrorLogRequest {
    pub diagnostic: Diagnostic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValidateBlockData {
    /// internal index used to differential one validate block instance from another
    internal_idx: u8,
}
impl ValidateBlockData {
    pub fn new(internal_idx: u8) -> Self {
        ValidateBlockData { internal_idx }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PickInputOptionRequest {
    pub options: Vec<InputOption>,
    pub selected: InputOption,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InputOption {
    pub value: String,
    pub displayed_value: String,
}

impl InputOption {
    pub fn default() -> Self {
        InputOption { value: String::new(), displayed_value: String::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProvidePublicKeyRequest {
    pub check_expectation_action_uuid: Option<ConstructDid>,
    pub message: String,
    pub namespace: String,
    pub network_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProvideSignedTransactionRequest {
    pub check_expectation_action_uuid: Option<ConstructDid>,
    pub signer_uuid: ConstructDid,
    pub expected_signer_address: Option<String>,
    pub skippable: bool,
    pub only_approval_needed: bool,
    pub payload: Value,
    pub formatted_payload: Option<String>,
    pub namespace: String,
    pub network_id: String,
}

impl ProvideSignedTransactionRequest {
    pub fn new(signer_uuid: &Did, payload: &Value, namespace: &str, network_id: &str) -> Self {
        ProvideSignedTransactionRequest {
            signer_uuid: ConstructDid(signer_uuid.clone()),
            check_expectation_action_uuid: None,
            expected_signer_address: None,
            skippable: false,
            payload: payload.clone(),
            formatted_payload: None,
            namespace: namespace.to_string(),
            network_id: network_id.to_string(),
            only_approval_needed: false,
        }
    }

    pub fn skippable(&mut self, is_skippable: bool) -> &mut Self {
        self.skippable = is_skippable;
        self
    }

    pub fn only_approval_needed(&mut self) -> &mut Self {
        self.only_approval_needed = true;
        self
    }

    pub fn check_expectation_action_uuid(&mut self, uuid: &ConstructDid) -> &mut Self {
        self.check_expectation_action_uuid = Some(uuid.clone());
        self
    }

    pub fn expected_signer_address(&mut self, address: Option<&str>) -> &mut Self {
        self.expected_signer_address = address.and_then(|a| Some(a.to_string()));
        self
    }

    pub fn formatted_payload(&mut self, display_payload: Option<String>) -> &mut Self {
        self.formatted_payload = display_payload;
        self
    }

    pub fn to_action_type(&self) -> ActionItemRequestType {
        ActionItemRequestType::ProvideSignedTransaction(self.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SendTransactionRequest {
    pub check_expectation_action_uuid: Option<ConstructDid>,
    pub signer_uuid: ConstructDid,
    pub expected_signer_address: Option<String>,
    pub payload: Value,
    pub formatted_payload: Option<String>,
    pub namespace: String,
    pub network_id: String,
}

impl SendTransactionRequest {
    pub fn new(signer_uuid: &Did, payload: &Value, namespace: &str, network_id: &str) -> Self {
        SendTransactionRequest {
            signer_uuid: ConstructDid(signer_uuid.clone()),
            check_expectation_action_uuid: None,
            expected_signer_address: None,
            payload: payload.clone(),
            formatted_payload: None,
            namespace: namespace.to_string(),
            network_id: network_id.to_string(),
        }
    }

    pub fn check_expectation_action_uuid(&mut self, uuid: &ConstructDid) -> &mut Self {
        self.check_expectation_action_uuid = Some(uuid.clone());
        self
    }

    pub fn expected_signer_address(&mut self, address: Option<&str>) -> &mut Self {
        self.expected_signer_address = address.and_then(|a| Some(a.to_string()));
        self
    }

    pub fn formatted_payload(&mut self, display_payload: Option<String>) -> &mut Self {
        self.formatted_payload = display_payload;
        self
    }

    pub fn to_action_type(&self) -> ActionItemRequestType {
        ActionItemRequestType::SendTransaction(self.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProvideSignedMessageRequest {
    pub check_expectation_action_uuid: Option<ConstructDid>,
    pub signer_uuid: ConstructDid,
    pub message: Value,
    pub namespace: String,
    pub network_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionItemResponse {
    pub action_item_id: BlockId,
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
    SendTransaction(SendTransactionResponse),
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
    pub force_execution: bool,
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
    pub signer_uuid: ConstructDid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvideSignedTransactionResponse {
    pub signed_transaction_bytes: Option<String>,
    pub signature_approved: Option<bool>,
    pub signer_uuid: ConstructDid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendTransactionResponse {
    pub transaction_hash: String,
    pub signer_uuid: ConstructDid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResponse {
    pub needs_credentials: bool,
    pub client_type: ClientType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientType {
    Operator,
    Participant,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelParticipantAuthRequest {
    pub otp_code: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelParticipantAuthResponse {
    pub auth_token: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename = "runbook", rename_all = "camelCase")]
pub struct OpenChannelRequest {
    pub runbook_name: String,
    pub runbook_description: Option<String>,
    pub registered_addons: Vec<String>,
    pub block_store: BTreeMap<usize, Block>,
    pub uuid: Uuid,
    pub slug: String,
    pub operating_token: String,
    pub totp: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename = "runbook", rename_all = "camelCase")]
pub struct DeleteChannelRequest {
    pub slug: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenChannelResponse {
    pub http_endpoint_url: String,
    pub ws_endpoint_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenChannelResponseBrowser {
    pub totp: String,
    pub http_endpoint_url: String,
    pub ws_endpoint_url: String,
    pub slug: String,
}
