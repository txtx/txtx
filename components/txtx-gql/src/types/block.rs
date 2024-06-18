use crate::Context;
use juniper_codegen::graphql_object;
use txtx_addon_kit::types::frontend::{
    ActionGroup, ActionItemRequest, ActionPanelData, ActionSubGroup, Block,
    ConstructProgressBarStatuses, ErrorPanelData, ModalPanelData,
    NormalizedActionItemRequestUpdate, Panel, ProgressBarStatus, ProgressBarStatusUpdate,
    ProgressBarVisibilityUpdate,
};

#[derive(Clone)]
pub struct GqlActionItemRequestUpdate {
    update: NormalizedActionItemRequestUpdate,
}
impl GqlActionItemRequestUpdate {
    pub fn new(update: NormalizedActionItemRequestUpdate) -> Self {
        GqlActionItemRequestUpdate { update }
    }
}

#[graphql_object(context = Context)]
impl GqlActionItemRequestUpdate {
    pub fn id(&self) -> String {
        self.update.id.to_string()
    }
    pub fn action_status(&self) -> Result<Option<String>, String> {
        match &self.update.action_status {
            Some(action_status) => {
                match serde_json::to_string(action_status).map_err(|e| e.to_string()) {
                    Ok(str) => Ok(Some(str)),
                    Err(e) => Err(e),
                }
            }
            None => Ok(None),
        }
    }
    pub fn action_type(&self) -> Result<Option<String>, String> {
        match &self.update.action_type {
            Some(action_type) => {
                match serde_json::to_string(action_type).map_err(|e| e.to_string()) {
                    Ok(str) => Ok(Some(str)),
                    Err(e) => Err(e),
                }
            }
            None => Ok(None),
        }
    }
}

#[derive(Clone)]
pub struct GqlActionBlock {
    block: Block,
}
impl GqlActionBlock {
    pub fn new(block: Block) -> Self {
        GqlActionBlock { block }
    }
}

#[graphql_object(context = Context)]
impl GqlActionBlock {
    #[graphql(name = "type")]
    pub fn typing(&self) -> String {
        match &self.block.panel {
            Panel::ActionPanel(_) => "ActionPanel".into(),
            _ => unreachable!(),
        }
    }

    pub fn uuid(&self) -> String {
        self.block.uuid.to_string()
    }

    pub fn visible(&self) -> bool {
        self.block.visible
    }

    pub fn panel(&self) -> GqlActionPanelData {
        match &self.block.panel {
            Panel::ActionPanel(panel_data) => GqlActionPanelData::new(panel_data.clone()),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub struct GqlModalBlock {
    block: Block,
}
impl GqlModalBlock {
    pub fn new(block: Block) -> Self {
        GqlModalBlock { block }
    }
}

#[graphql_object(context = Context)]
impl GqlModalBlock {
    #[graphql(name = "type")]
    pub fn typing(&self) -> String {
        match &self.block.panel {
            Panel::ModalPanel(_) => "ModalPanel".into(),
            _ => unreachable!(),
        }
    }

    pub fn uuid(&self) -> String {
        self.block.uuid.to_string()
    }

    pub fn visible(&self) -> bool {
        self.block.visible
    }

    pub fn panel(&self) -> GqlModalPanelData {
        match &self.block.panel {
            Panel::ModalPanel(panel_data) => GqlModalPanelData::new(panel_data.clone()),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub struct GqlErrorBlock {
    block: Block,
}
impl GqlErrorBlock {
    pub fn new(block: Block) -> Self {
        GqlErrorBlock { block }
    }
}

#[graphql_object(context = Context)]
impl GqlErrorBlock {
    #[graphql(name = "type")]
    pub fn typing(&self) -> String {
        match &self.block.panel {
            Panel::ErrorPanel(_) => "ErrorPanel".into(),
            _ => unreachable!(),
        }
    }

    pub fn uuid(&self) -> String {
        self.block.uuid.to_string()
    }

    pub fn visible(&self) -> bool {
        self.block.visible
    }

    pub fn panel(&self) -> GqlErrorPanelData {
        match &self.block.panel {
            Panel::ErrorPanel(panel_data) => GqlErrorPanelData::new(panel_data.clone()),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub struct GqlProgressBlock {
    block: Block,
}
impl GqlProgressBlock {
    pub fn new(block: Block) -> Self {
        GqlProgressBlock { block }
    }
}

#[graphql_object(context = Context)]
impl GqlProgressBlock {
    #[graphql(name = "type")]
    pub fn typing(&self) -> String {
        match &self.block.panel {
            Panel::ProgressBar(_) => "ProgressBar".into(),
            _ => unreachable!(),
        }
    }

    pub fn uuid(&self) -> String {
        self.block.uuid.to_string()
    }

    pub fn visible(&self) -> bool {
        self.block.visible
    }

    pub fn panel(&self) -> Vec<GqlConstructProgressBarStatuses> {
        match self.block.panel.clone() {
            Panel::ProgressBar(panel_data) => panel_data
                .into_iter()
                .map(GqlConstructProgressBarStatuses::new)
                .collect(),
            _ => unreachable!(),
        }
    }
}

pub struct GqlActionPanelData {
    data: ActionPanelData,
}
impl GqlActionPanelData {
    pub fn new(data: ActionPanelData) -> Self {
        GqlActionPanelData { data }
    }
}
#[graphql_object(context = Context)]
impl GqlActionPanelData {
    pub fn title(&self) -> String {
        self.data.title.clone()
    }

    pub fn description(&self) -> String {
        self.data.description.clone()
    }

    pub fn groups(&self) -> Vec<GqlActionGroup> {
        self.data
            .groups
            .clone()
            .into_iter()
            .map(GqlActionGroup::new)
            .collect()
    }
}

pub struct GqlModalPanelData {
    data: ModalPanelData,
}
impl GqlModalPanelData {
    pub fn new(data: ModalPanelData) -> Self {
        GqlModalPanelData { data }
    }
}
#[graphql_object(context = Context)]
impl GqlModalPanelData {
    pub fn title(&self) -> String {
        self.data.title.clone()
    }

    pub fn description(&self) -> String {
        self.data.description.clone()
    }

    pub fn groups(&self) -> Vec<GqlActionGroup> {
        self.data
            .groups
            .clone()
            .into_iter()
            .map(GqlActionGroup::new)
            .collect()
    }
}

pub struct GqlErrorPanelData {
    data: ErrorPanelData,
}
impl GqlErrorPanelData {
    pub fn new(data: ErrorPanelData) -> Self {
        GqlErrorPanelData { data }
    }
}
#[graphql_object(context = Context)]
impl GqlErrorPanelData {
    pub fn title(&self) -> String {
        self.data.title.clone()
    }

    pub fn description(&self) -> String {
        self.data.description.clone()
    }

    pub fn groups(&self) -> Vec<GqlActionGroup> {
        self.data
            .groups
            .clone()
            .into_iter()
            .map(GqlActionGroup::new)
            .collect()
    }
}
pub struct GqlConstructProgressBarStatuses {
    data: ConstructProgressBarStatuses,
}
impl GqlConstructProgressBarStatuses {
    pub fn new(data: ConstructProgressBarStatuses) -> Self {
        GqlConstructProgressBarStatuses { data }
    }
}
#[graphql_object(context = Context)]
impl GqlConstructProgressBarStatuses {
    pub fn construct_uuid(&self) -> String {
        self.data.construct_uuid.to_string()
    }
    pub fn statuses(&self) -> Vec<GqlProgressBarStatus> {
        self.data
            .statuses
            .clone()
            .into_iter()
            .map(GqlProgressBarStatus::new)
            .collect()
    }
}

pub struct GqlProgressBarStatus {
    data: ProgressBarStatus,
}
impl GqlProgressBarStatus {
    pub fn new(data: ProgressBarStatus) -> Self {
        GqlProgressBarStatus { data }
    }
}
#[graphql_object(context = Context)]
impl GqlProgressBarStatus {
    pub fn status(&self) -> String {
        self.data.status.clone()
    }

    pub fn message(&self) -> String {
        self.data.message.clone()
    }

    pub fn diagnostic(&self) -> Option<String> {
        match &self.data.diagnostic {
            Some(diag) => Some(serde_json::to_string(&diag).unwrap()),
            None => None,
        }
    }
}

pub struct GqlProgressBarStatusUpdate {
    data: ProgressBarStatusUpdate,
}
impl GqlProgressBarStatusUpdate {
    pub fn new(data: ProgressBarStatusUpdate) -> Self {
        GqlProgressBarStatusUpdate { data }
    }
}
#[graphql_object(context = Context)]
impl GqlProgressBarStatusUpdate {
    pub fn progress_bar_uuid(&self) -> String {
        self.data.progress_bar_uuid.to_string()
    }
    pub fn construct_uuid(&self) -> String {
        self.data.construct_uuid.to_string()
    }
    pub fn new_status(&self) -> GqlProgressBarStatus {
        GqlProgressBarStatus::new(self.data.new_status.clone())
    }
}

pub struct GqlProgressBarVisibilityUpdate {
    data: ProgressBarVisibilityUpdate,
}
impl GqlProgressBarVisibilityUpdate {
    pub fn new(data: ProgressBarVisibilityUpdate) -> Self {
        GqlProgressBarVisibilityUpdate { data }
    }
}
#[graphql_object(context = Context)]
impl GqlProgressBarVisibilityUpdate {
    pub fn progress_bar_uuid(&self) -> String {
        self.data.progress_bar_uuid.to_string()
    }
    pub fn visible(&self) -> bool {
        self.data.visible
    }
}

pub struct GqlActionGroup {
    group: ActionGroup,
}
impl GqlActionGroup {
    pub fn new(group: ActionGroup) -> Self {
        GqlActionGroup { group }
    }
}

#[graphql_object(context = Context)]
impl GqlActionGroup {
    pub fn title(&self) -> String {
        self.group.title.clone()
    }

    pub fn sub_groups(&self) -> Vec<GqlActionSubGroup> {
        self.group
            .sub_groups
            .clone()
            .into_iter()
            .map(GqlActionSubGroup::new)
            .collect()
    }
}

pub struct GqlActionSubGroup {
    sub_group: ActionSubGroup,
}
impl GqlActionSubGroup {
    pub fn new(sub_group: ActionSubGroup) -> Self {
        GqlActionSubGroup { sub_group }
    }
}

#[graphql_object(context = Context)]
impl GqlActionSubGroup {
    pub fn allow_batch_completion(&self) -> bool {
        self.sub_group.allow_batch_completion.clone()
    }

    pub fn action_items(&self) -> Vec<GqlActionItemRequest> {
        self.sub_group
            .action_items
            .clone()
            .into_iter()
            .map(GqlActionItemRequest::new)
            .collect()
    }
}

pub struct GqlActionItemRequest {
    action_item: ActionItemRequest,
}
impl GqlActionItemRequest {
    pub fn new(action_item: ActionItemRequest) -> Self {
        GqlActionItemRequest { action_item }
    }
}
#[graphql_object(context = Context)]
impl GqlActionItemRequest {
    pub fn id(&self) -> String {
        self.action_item.id.to_string()
    }

    pub fn index(&self) -> i32 {
        self.action_item.index.clone() as i32
    }

    pub fn title(&self) -> String {
        self.action_item.title.clone()
    }

    pub fn description(&self) -> Option<String> {
        self.action_item.description.clone()
    }

    pub fn action_status(&self) -> String {
        serde_json::to_string(&self.action_item.action_status).unwrap()
    }

    pub fn action_type(&self) -> String {
        serde_json::to_string(&self.action_item.action_type).unwrap()
    }
}
