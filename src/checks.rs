// src/checks.rs

//! Assertions and budget enforcement.

use crate::config::Assertion;
use anyhow::{bail, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::BTreeMap;

/// Resolved budget values after combining config.yaml and CLI overrides.
#[derive(Debug, Clone)]
pub struct BudgetsResolved {
    pub duration_ms: Option<u64>,
    pub memory_kb: Option<u64>,
}

/// Resolve a dotted / indexed path into a JSON value.
///
/// Supports:
/// - callback.outputFields.success
/// - items[0].id
/// - items.0.id
pub fn get_by_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;

    let normalized = path.replace('[', ".").replace(']', "");
    for segment in normalized.split('.') {
        if segment.is_empty() {
            continue;
        }

        if let Ok(idx) = segment.parse::<usize>() {
            current = current.get(idx)?;
        } else {
            current = current.get(segment)?;
        }
    }

    Some(current)
}

/// Apply assertions to an actual JSON output.
///
/// Fails on the first mismatch found.
pub fn assert_json(actual: &Value, assertions: &BTreeMap<String, Assertion>) -> Result<()> {
    for (path, assertion) in assertions {
        let actual_value = get_by_path(actual, path)
            .ok_or_else(|| anyhow::anyhow!("Assertion path not found: {}", path))?;

        match assertion {
            Assertion::Eq { eq } => {
                if actual_value != eq {
                    bail!(
                        "Assertion failed at '{}': expected {}, got {}",
                        path,
                        json(eq),
                        json(actual_value)
                    );
                }
            }

            Assertion::Gt { gt } => {
                let a = as_number(actual_value)?;
                let b = as_number(gt)?;
                if a <= b {
                    bail!("Assertion failed at '{}': {} <= {}", path, a, b);
                }
            }

            Assertion::Lt { lt } => {
                let a = as_number(actual_value)?;
                let b = as_number(lt)?;
                if a >= b {
                    bail!("Assertion failed at '{}': {} >= {}", path, a, b);
                }
            }

            Assertion::Exists { exists } => {
                if *exists && actual_value.is_null() {
                    bail!("Assertion failed at '{}': value does not exist", path);
                }
            }

            Assertion::Regex { regex } => {
                let s = actual_value
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Value at '{}' is not a string", path))?;

                let re = Regex::new(regex)
                    .map_err(|e| anyhow::anyhow!("Invalid regex '{}': {}", regex, e))?;

                if !re.is_match(s) {
                    bail!(
                        "Assertion failed at '{}': '{}' does not match /{}/",
                        path,
                        s,
                        regex
                    );
                }
            }
        }
    }

    Ok(())
}

/// Enforce duration and memory budgets.
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
        let actual_kb = max_rss_kb
            .ok_or_else(|| anyhow::anyhow!("Memory budget set but memory measurement unavailable"))?;

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

/* ---------------- helpers ---------------- */

fn as_number(v: &Value) -> Result<f64> {
    v.as_f64()
        .ok_or_else(|| anyhow::anyhow!("Expected numeric value, got {}", json(v)))
}

fn json(v: &Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "<json>".to_string())
}
