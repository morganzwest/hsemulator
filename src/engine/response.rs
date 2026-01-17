// src/engine/response.rs
use serde::Serialize;

use crate::{
    execution_id::ExecutionId,
    engine::{ValidationError},
};

#[derive(Debug, Serialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
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
