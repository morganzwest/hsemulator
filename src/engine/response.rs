// src/engine/response.rs
use serde::Serialize;

use crate::{engine::ValidationError, execution_id::ExecutionId};

#[derive(Debug, Serialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
#[allow(dead_code)]
pub enum ExecutionResponse {
    Validate {
        execution_id: ExecutionId,
        valid: bool,
        errors: Vec<ValidationError>,
    },
    Execute {
        execution_id: ExecutionId,
        result: crate::engine::ExecutionResult,
    },
}
