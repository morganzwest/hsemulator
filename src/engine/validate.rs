use anyhow::Result;
use std::path::Path;

use crate::config::Config;
use crate::engine::ValidationResult;
use crate::util::read_to_string;

pub fn validate_config(cfg: &Config) -> Result<ValidationResult> {
    let mut result = ValidationResult::ok();

    validate_action(cfg, &mut result)?;
    validate_fixtures(cfg, &mut result)?;
    validate_runtime(cfg, &mut result)?;
    validate_budgets(cfg, &mut result)?;

    Ok(result)
}

/* ---------------- action ---------------- */

fn validate_action(cfg: &Config, result: &mut ValidationResult) -> Result<()> {
    let action = match &cfg.action {
        Some(a) => a,
        None => {
            result.push_error(
                "ACTION_MISSING",
                "No action defined in config",
            );
            return Ok(());
        }
    };

    let entry = Path::new(&action.entry);

    if !entry.exists() {
        result.push_error(
            "ACTION_NOT_FOUND",
            format!("Action entry does not exist: {}", entry.display()),
        );
        return Ok(());
    }

    if !entry.is_file() {
        result.push_error(
            "ACTION_NOT_FILE",
            format!("Action entry is not a file: {}", entry.display()),
        );
    }

    let ext = entry.extension().and_then(|s| s.to_str()).unwrap_or("");

    if ext != "js" && ext != "mjs" && ext != "cjs" && ext != "py" {
        result.push_error(
            "ACTION_UNSUPPORTED_TYPE",
            format!("Unsupported action file extension: .{}", ext),
        );
    }

    Ok(())
}

/* ---------------- fixtures ---------------- */

fn validate_fixtures(cfg: &Config, result: &mut ValidationResult) -> Result<()> {
    if cfg.fixtures.is_empty() {
        result.push_error(
            "FIXTURES_EMPTY",
            "At least one fixture must be provided",
        );
        return Ok(());
    }

    for fixture in &cfg.fixtures {
        let path = Path::new(fixture);

        if !path.exists() {
            result.push_error(
                "FIXTURE_NOT_FOUND",
                format!("Fixture not found: {}", path.display()),
            );
            continue;
        }

        let raw = match read_to_string(path) {
            Ok(v) => v,
            Err(e) => {
                result.push_error(
                    "FIXTURE_READ_FAILED",
                    format!("Failed to read {}: {}", path.display(), e),
                );
                continue;
            }
        };

        if serde_json::from_str::<serde_json::Value>(&raw).is_err() {
            result.push_error(
                "FIXTURE_INVALID_JSON",
                format!("Fixture is not valid JSON: {}", path.display()),
            );
        }
    }

    Ok(())
}

/* ---------------- runtime ---------------- */

fn validate_runtime(cfg: &Config, result: &mut ValidationResult) -> Result<()> {
    let action = match &cfg.action {
        Some(a) => a,
        None => return Ok(()), // already reported by validate_action
    };

    let ext = Path::new(&action.entry)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    match ext {
        "py" => {
            if cfg.runtime.python.trim().is_empty() {
                result.push_error(
                    "RUNTIME_PYTHON_MISSING",
                    "Python runtime is not configured",
                );
            }
        }
        "js" | "mjs" | "cjs" => {
            if cfg.runtime.node.trim().is_empty() {
                result.push_error(
                    "RUNTIME_NODE_MISSING",
                    "Node runtime is not configured",
                );
            }
        }
        _ => {}
    }

    Ok(())
}


/* ---------------- budgets ---------------- */

fn validate_budgets(cfg: &Config, result: &mut ValidationResult) -> Result<()> {
    if let Some(b) = &cfg.budgets {
        if let Some(ms) = b.duration_ms {
            if ms == 0 {
                result.push_error(
                    "BUDGET_DURATION_INVALID",
                    "duration_ms must be greater than zero",
                );
            }
        }

        if let Some(mem) = b.memory_mb {
            if mem == 0 {
                result.push_error(
                    "BUDGET_MEMORY_INVALID",
                    "memory_mb must be greater than zero",
                );
            }
        }
    }

    Ok(())
}
