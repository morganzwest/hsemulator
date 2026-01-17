use crate::execution_id::ExecutionId;
use serde::{Serialize, Deserialize};
use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ExecutionEventKind {
    ExecutionCreated,
    ValidationFailed,
    ValidationSucceeded,
    ExecutionStarted,
    ExecutionFinished,
    ValidationStarted,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionEvent {
    pub execution_id: ExecutionId,
    pub kind: ExecutionEventKind,
    pub timestamp: SystemTime,
}

pub fn execution_created(execution_id: ExecutionId) -> ExecutionEvent {
    ExecutionEvent {
        execution_id,
        kind: ExecutionEventKind::ExecutionCreated,
        timestamp: SystemTime::now(),
    }
}
