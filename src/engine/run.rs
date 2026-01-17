use anyhow::Result;

use crate::{
    config::Config,
    engine::{
        execute_action,
        validate::validate_config,
        ExecutionMode,
        events::{execution_created, ExecutionEvent, ExecutionEventKind},
        summary::ExecutionSummary,
    },
    execution_id::ExecutionId,
    sinks::collecting::CollectingEventSink,
};
use crate::engine::sink::EventSink;

/// Execute a full run (validation + execution) and collect all emitted events.
///
/// This function owns the event sink to avoid holding mutable trait objects
/// across `.await`, ensuring the returned future is `Send`.
pub async fn run_execution(
    cfg: Config,
    mode: ExecutionMode,
) -> Result<(ExecutionSummary, CollectingEventSink)> {
    let mut sink = CollectingEventSink::new();
    let execution_id = ExecutionId::new();

    // ---- execution created ----
    sink.emit(execution_created(execution_id.clone()));

    // ---- validation started ----
    sink.emit(ExecutionEvent {
        execution_id: execution_id.clone(),
        kind: ExecutionEventKind::ValidationStarted,
        timestamp: std::time::SystemTime::now(),
    });

    let validation = validate_config(&cfg)?;

    if !validation.is_valid() {
        sink.emit(ExecutionEvent {
            execution_id: execution_id.clone(),
            kind: ExecutionEventKind::ValidationFailed,
            timestamp: std::time::SystemTime::now(),
        });

        return Ok((
            ExecutionSummary::validation_failed(execution_id),
            sink,
        ));
    }

    // ---- validate-only mode ----
    if mode == ExecutionMode::Validate {
        return Ok((
            ExecutionSummary::validated_only(execution_id),
            sink,
        ));
    }

    // ---- execution ----
    let result = execute_action(cfg, execution_id.clone(), &mut sink).await?;

    Ok((
        ExecutionSummary::executed(execution_id, result),
        sink,
    ))
}
