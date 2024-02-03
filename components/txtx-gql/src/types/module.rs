use crate::Context;
use juniper_codegen::graphql_object;
use txtx_core::types::{ConstructUuid, ModuleConstruct};

pub struct Module {
    pub uuid: ConstructUuid,
    pub data: ModuleConstruct,
}

impl Module {
    pub fn new(uuid: &ConstructUuid, data: &ModuleConstruct) -> Self {
        Self {
            uuid: uuid.clone(),
            data: data.clone(),
        }
    }
}

#[graphql_object(context = Context)]
impl Module {
    pub fn uuid(&self) -> String {
        self.uuid.value().to_string()
    }

    pub fn id(&self) -> String {
        self.data.id.to_string()
    }
}
