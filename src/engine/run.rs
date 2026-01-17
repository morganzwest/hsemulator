// src/engine/run.rs
use anyhow::Result;

use crate::{
    config::Config,
    engine::{
        validate::validate_config,
        execute_action,
        ExecutionMode,
    },
    execution_id::ExecutionId,
};

use super::response::ExecutionResponse;

pub async fn run(
    cfg: Config,
    mode: ExecutionMode,
) -> Result<ExecutionResponse> {
    let execution_id = ExecutionId::new();

    let validation = validate_config(&cfg)?;

    if !validation.is_valid() {
        return Ok(ExecutionResponse::Validate {
            execution_id,
            valid: false,
            errors: validation.errors,
        });
    }

    if mode == ExecutionMode::Validate {
        return Ok(ExecutionResponse::Validate {
            execution_id,
            valid: true,
            errors: vec![],
        });
    }

    let result = execute_action(cfg, None).await?;


    Ok(ExecutionResponse::Execute {
        execution_id,
        result,
    })
}
