use anyhow::Result;

use crate::config::Config;
use crate::engine::ExecutionResult;
use crate::engine::sink::EventSink;
use crate::engine::events::{ExecutionEvent, ExecutionEventKind};
use crate::execution_id::ExecutionId;

pub async fn execute_action(
    cfg: Config,
    execution_id: ExecutionId,
    sink: &mut dyn EventSink,
) -> Result<ExecutionResult> {
    // ---- execution started ----
    sink.emit(ExecutionEvent {
        execution_id: execution_id.clone(),
        kind: ExecutionEventKind::ExecutionStarted,
        timestamp: std::time::SystemTime::now(),
    });

    // ---- run action ----
    let summary = crate::runner::execute(cfg, None).await?;

    // ---- execution finished ----
    sink.emit(ExecutionEvent {
        execution_id: execution_id.clone(),
        kind: ExecutionEventKind::ExecutionFinished,
        timestamp: std::time::SystemTime::now(),
    });

    // ---- map result ----
    Ok(ExecutionResult {
        ok: summary.ok,
        runs: summary.runs,
        failures: summary.failures,
        max_duration_ms: summary.max_duration_ms,
        max_memory_kb: summary.max_memory_kb,
        snapshots_ok: summary.snapshots_ok,
    })
}
