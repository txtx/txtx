use crate::Context;
use juniper_codegen::graphql_object;
use txtx_core::kit::types::frontend::{
    ActionGroup, ActionItemRequest, ActionSubGroup, Block, BlockEvent, Panel, SetActionItemStatus,
};

#[derive(Clone)]
pub enum GqlBlockEvent {
    Append(GqlBlock),
    Clear,
    UpdateActionItems(Vec<GqlSetActionItemStatus>),
}
impl GqlBlockEvent {
    pub fn new(block_event: BlockEvent) -> Self {
        match block_event {
            BlockEvent::Append(block) => GqlBlockEvent::Append(GqlBlock::new(block)),
            BlockEvent::Clear => GqlBlockEvent::Clear,
            BlockEvent::UpdateActionItems(updates) => GqlBlockEvent::UpdateActionItems(
                updates
                    .into_iter()
                    .map(GqlSetActionItemStatus::new)
                    .collect(),
            ),
        }
    }
}

#[graphql_object(context = Context)]
impl GqlBlockEvent {
    pub fn append(&self) -> Option<GqlBlock> {
        match self {
            GqlBlockEvent::Append(block) => Some(block.clone()),
            _ => None,
        }
    }

    pub fn update_action_items(&self) -> Option<Vec<GqlSetActionItemStatus>> {
        match self {
            GqlBlockEvent::UpdateActionItems(updates) => Some(updates.to_vec()),
            _ => None,
        }
    }
}

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
pub struct GqlBlock {
    block: Block,
}
impl GqlBlock {
    pub fn new(block: Block) -> Self {
        GqlBlock { block }
    }
}

#[graphql_object(context = Context)]
impl GqlBlock {
    pub fn uuid(&self) -> String {
        self.block.uuid.to_string()
    }

    pub fn title(&self) -> String {
        match &self.block.panel {
            Panel::ActionPanel(panel) => panel.title.clone(),
        }
    }

    pub fn description(&self) -> String {
        match &self.block.panel {
            Panel::ActionPanel(panel) => panel.description.clone(),
        }
    }

    pub fn groups(&self) -> Option<Vec<GqlActionGroup>> {
        let groups = match &self.block.panel {
            Panel::ActionPanel(panel) => panel
                .groups
                .clone()
                .into_iter()
                .map(GqlActionGroup::new)
                .collect(),
        };
        Some(groups)
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
