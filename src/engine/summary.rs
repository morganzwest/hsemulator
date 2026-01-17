use crate::execution_id::ExecutionId;
use crate::engine::ExecutionResult;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub execution_id: ExecutionId,
    pub status: ExecutionStatus,
    pub result: Option<ExecutionResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ExecutionStatus {
    ValidatedOnly,
    ValidationFailed,
    Executed,
}

impl ExecutionSummary {
    pub fn validation_failed(execution_id: ExecutionId) -> Self {
        Self {
            execution_id,
            status: ExecutionStatus::ValidationFailed,
            result: None,
        }
    }

    pub fn validated_only(execution_id: ExecutionId) -> Self {
        Self {
            execution_id,
            status: ExecutionStatus::ValidatedOnly,
            result: None,
        }
    }

    pub fn executed(execution_id: ExecutionId, result: ExecutionResult) -> Self {
        Self {
            execution_id,
            status: ExecutionStatus::Executed,
            result: Some(result),
        }
    }
}
