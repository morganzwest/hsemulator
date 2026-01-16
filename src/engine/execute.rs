use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::config::Config;
use crate::engine::{ExecutionResult};
use crate::engine::validate::validate_config;

pub async fn execute_action(
    cfg: Config,
    assertion_file: Option<PathBuf>,
) -> Result<ExecutionResult> {
    let validation = validate_config(&cfg)?;

    if !validation.valid {
        bail!(
            "Validation failed: {}",
            validation
                .errors
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        );
    }

    let summary = crate::runner::execute(cfg, assertion_file).await?;

    Ok(ExecutionResult {
        ok: summary.ok,
        runs: summary.runs,
        failures: summary.failures,
        max_duration_ms: summary.max_duration_ms,
        max_memory_kb: summary.max_memory_kb,
        snapshots_ok: summary.snapshots_ok,
    })
}
