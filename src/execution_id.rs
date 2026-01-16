use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionId(pub String);

impl ExecutionId {
    pub fn new() -> Self {
        Self(format!("exec_{}", Uuid::new_v4()))
    }
}
