use crate::Context;
use juniper_codegen::graphql_object;
use txtx_core::types::{CommandInstance, ConstructUuid};

pub struct Construct {
    pub uuid: ConstructUuid,
    pub data: CommandInstance,
}

impl Construct {
    pub fn new(uuid: &ConstructUuid, data: &CommandInstance) -> Self {
        Self {
            uuid: uuid.clone(),
            data: data.clone(),
        }
    }
}

#[graphql_object(context = Context)]
impl Construct {
    pub fn uuid(&self) -> String {
        self.uuid.value().to_string()
    }

    pub fn id(&self) -> String {
        self.data.name.to_string()
    }
}
