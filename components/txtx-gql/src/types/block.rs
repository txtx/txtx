use crate::Context;
use juniper_codegen::graphql_object;
use txtx_core::kit::types::frontend::{
    ActionGroup, ActionItemRequest, ActionPanelData, ActionSubGroup, Block, ModalPanelData, Panel,
    ProgressBarStatus, SetActionItemStatus,
};

#[derive(Clone)]
pub struct GqlSetActionItemStatus {
    set_action_item_status: SetActionItemStatus,
}
impl GqlSetActionItemStatus {
    pub fn new(update: SetActionItemStatus) -> Self {
        GqlSetActionItemStatus {
            set_action_item_status: update,
        }
    }
}

#[graphql_object(context = Context)]
impl GqlSetActionItemStatus {
    pub fn action_item_uuid(&self) -> String {
        self.set_action_item_status.action_item_uuid.to_string()
    }
    pub fn new_status(&self) -> Result<String, String> {
        serde_json::to_string(&self.set_action_item_status.new_status).map_err(|e| e.to_string())
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

    pub fn panel(&self) -> GqlProgressBarStatus {
        match &self.block.panel {
            Panel::ProgressBar(panel_data) => GqlProgressBarStatus::new(panel_data.clone()),
            _ => unreachable!(),
        }
    }
}
pub struct GqlPanel {
    panel: Panel,
}
impl GqlPanel {
    pub fn new(panel: Panel) -> Self {
        GqlPanel { panel }
    }
}
#[graphql_object(context = Context)]
impl GqlPanel {
    pub fn action_panel_data(&self) -> Option<GqlActionPanelData> {
        match &self.panel {
            Panel::ActionPanel(panel_data) => Some(GqlActionPanelData::new(panel_data.clone())),
            _ => None,
        }
    }

    pub fn modal_panel_data(&self) -> Option<GqlModalPanelData> {
        match &self.panel {
            Panel::ModalPanel(panel_data) => Some(GqlModalPanelData::new(panel_data.clone())),
            _ => None,
        }
    }

    pub fn progress_bar_data(&self) -> Option<GqlProgressBarStatus> {
        match &self.panel {
            Panel::ProgressBar(panel_data) => Some(GqlProgressBarStatus::new(panel_data.clone())),
            _ => None,
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
    pub fn uuid(&self) -> String {
        self.action_item.uuid.to_string()
    }

    pub fn index(&self) -> i32 {
        self.action_item.index.clone() as i32
    }

    pub fn title(&self) -> String {
        self.action_item.title.clone()
    }

    pub fn description(&self) -> String {
        self.action_item.description.clone()
    }

    pub fn action_status(&self) -> String {
        serde_json::to_string(&self.action_item.action_status).unwrap()
    }

    pub fn action_type(&self) -> String {
        serde_json::to_string(&self.action_item.action_type).unwrap()
    }
}
