use crate::Context;
use juniper_codegen::graphql_object;
use txtx_vm::types::{ConstructUuid, VariableConstruct};

pub struct Variable {
    pub uuid: ConstructUuid,
    pub data: VariableConstruct,
}

impl Variable {
    pub fn new(uuid: &ConstructUuid, data: &VariableConstruct) -> Self {
        Self {
            uuid: uuid.clone(),
            data: data.clone(),
        }
    }
}

#[graphql_object(context = Context)]
impl Variable {
    pub fn uuid(&self) -> String {
        self.uuid.value().to_string()
    }

    pub fn name(&self) -> String {
        self.data.name.to_string()
    }
}
