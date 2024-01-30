use crate::Context;
use juniper_codegen::graphql_object;
use txtx_vm::types::{ConstructUuid, OutputConstruct};

pub struct Output {
    pub uuid: ConstructUuid,
    pub data: OutputConstruct,
}

impl Output {
    pub fn new(uuid: &ConstructUuid, data: &OutputConstruct) -> Self {
        Self {
            uuid: uuid.clone(),
            data: data.clone(),
        }
    }
}

#[graphql_object(context = Context)]
impl Output {
    pub fn uuid(&self) -> String {
        self.uuid.value().to_string()
    }

    pub fn name(&self) -> String {
        self.data.name.to_string()
    }
}
