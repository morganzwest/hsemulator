// src/checks.rs

//! Assertions and budget enforcement.
//!
//! This module is responsible for:
//! - Loading assertion files
//! - Resolving dot-paths into JSON values
//! - Comparing expected vs actual output
//! - Enforcing time and memory budgets
//!
//! Assertions and budgets are optional and opt-in.
//! When enabled, failures are surfaced clearly and cause the run to fail.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::Path;

/// Resolved budget values after combining config.yaml and CLI overrides.
#[derive(Debug, Clone)]
pub struct BudgetsResolved {
    /// Maximum allowed duration in milliseconds
    pub duration_ms: Option<u64>,

    /// Maximum allowed peak memory in KB (RSS)
    pub memory_kb: Option<u64>,
}

/// Load assertions from a JSON file.
///
/// Expected format:
/// {
///   "callback.outputFields.success": true,
///   "result.some.nested.value": 123
/// }
///
/// Keys are dot-paths, values are JSON values to compare against.
pub fn load_assertions(path: &Path) -> Result<serde_json::Map<String, Value>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read assertions file {:?}", path))?;

    let parsed: Value =
        serde_json::from_str(&raw).context("Assertions file is not valid JSON")?;

    let obj = parsed
        .as_object()
        .context("Assertions JSON must be an object of { path: expectedValue }")?;

    Ok(obj.clone())
}

/// Resolve a dot-path into a JSON value.
///
/// Supports:
/// - Object keys: "callback.outputFields.success"
/// - Array indices: "items.0.id"
///
/// Returns `None` if the path cannot be resolved.
pub fn get_by_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;

    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }

        // Try array index first
        if let Ok(index) = segment.parse::<usize>() {
            current = current.get(index)?;
        } else {
            current = current.get(segment)?;
        }
    }

    Some(current)
}

/// Apply assertions to an actual JSON output.
///
/// Fails on the first mismatch found.
pub fn assert_json(
    actual: &Value,
    assertions: &serde_json::Map<String, Value>,
) -> Result<()> {
    for (path, expected) in assertions {
        let actual_value = get_by_path(actual, path)
            .ok_or_else(|| anyhow::anyhow!("Assertion path not found: {}", path))?;

        if actual_value != expected {
            bail!(
                "Assertion failed at '{}': expected {}, got {}",
                path,
                serde_json::to_string(expected)
                    .unwrap_or_else(|_| "<expected>".to_string()),
                serde_json::to_string(actual_value)
                    .unwrap_or_else(|_| "<actual>".to_string()),
            );
        }
    }

    Ok(())
}

/// Enforce duration and memory budgets.
///
/// Fails if any configured budget is exceeded.
pub fn check_budgets(
    duration_ms: u128,
    max_rss_kb: Option<u64>,
    budgets: &BudgetsResolved,
) -> Result<()> {
    if let Some(max_duration) = budgets.duration_ms {
        if duration_ms > u128::from(max_duration) {
            bail!(
                "Duration budget exceeded: {}ms (budget {}ms)",
                duration_ms,
                max_duration
            );
        }
    }

    if let Some(max_mem_kb) = budgets.memory_kb {
        let actual_kb = max_rss_kb.ok_or_else(|| {
            anyhow::anyhow!("Memory budget set but memory measurement unavailable")
        })?;

        if actual_kb > max_mem_kb {
            bail!(
                "Memory budget exceeded: {}MB (budget {}MB)",
                actual_kb / 1024,
                max_mem_kb / 1024
            );
        }
    }

    Ok(())
}
