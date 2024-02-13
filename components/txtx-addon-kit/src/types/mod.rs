use uuid::Uuid;

pub mod commands;
pub mod diagnostics;
pub mod functions;
pub mod types;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstructUuid {
    Local(Uuid),
}

impl ConstructUuid {
    pub fn new() -> Self {
        Self::Local(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: &Uuid) -> Self {
        Self::Local(uuid.clone())
    }

    pub fn value(&self) -> Uuid {
        match &self {
            Self::Local(v) => v.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PackageUuid {
    Local(Uuid),
}

impl PackageUuid {
    pub fn new() -> Self {
        Self::Local(Uuid::new_v4())
    }

    pub fn value(&self) -> Uuid {
        match &self {
            Self::Local(v) => v.clone(),
        }
    }
}
