use serde::{Serialize, Serializer};
use uuid::Uuid;

pub mod commands;
pub mod diagnostics;
pub mod functions;
pub mod typing;

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

impl Serialize for ConstructUuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Local(uuid) => serializer.serialize_str(&format!("local:{}", uuid.to_string())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PackageUuid {
    Local(Uuid),
}

impl Serialize for PackageUuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Local(uuid) => serializer.serialize_str(&format!("local:{}", uuid.to_string())),
        }
    }
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
