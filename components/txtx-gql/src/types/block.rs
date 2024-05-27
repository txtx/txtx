use crate::Context;
use juniper_codegen::graphql_object;
use txtx_core::kit::types::frontend::{ActionGroup, ActionItem, ActionSubGroup, Block, Panel};

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

    pub fn action_items(&self) -> Vec<GqlActionItem> {
        self.sub_group
            .action_items
            .clone()
            .into_iter()
            .map(GqlActionItem::new)
            .collect()
    }
}

pub struct GqlActionItem {
    action_item: ActionItem,
}
impl GqlActionItem {
    pub fn new(action_item: ActionItem) -> Self {
        GqlActionItem { action_item }
    }
}
#[graphql_object(context = Context)]
impl GqlActionItem {
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
