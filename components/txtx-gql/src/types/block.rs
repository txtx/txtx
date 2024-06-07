use crate::Context;
use juniper_codegen::graphql_object;
use txtx_core::kit::types::frontend::{
    ActionGroup, ActionItemRequest, ActionPanelData, ActionSubGroup, Block, ModalPanelData,
    NormalizedActionItemRequestUpdate, Panel, ProgressBarStatus,
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
    pub fn uuid(&self) -> String {
        self.update.uuid.to_string()
    }
    pub fn title(&self) -> Option<String> {
        self.update.title.clone()
    }
    pub fn description(&self) -> Option<String> {
        self.update.description.clone().unwrap_or(None)
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
