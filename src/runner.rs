// src/runner.rs

//! Core execution engine for hsemulate.
//!
//! Responsibilities:
//! - Handle `init` scaffolding
//! - Execute a HubSpot custom code action locally (JS or Python)
//! - Feed fixture JSON as the `event` payload
//! - Stream action logs to the terminal
//! - Collect timing + memory metrics
//! - Apply assertions, snapshots, budgets
//! - Repeat runs to detect flaky behaviour
//! - Emit a final machine-readable summary JSON
//!
//! This file intentionally contains the orchestration logic.
//! Lower-level concerns live in other modules (checks, metrics, shim, snapshot, util).

use crate::checks::{assert_json, check_budgets, load_assertions, BudgetsResolved};
use crate::cli::{Cli, Command};
use crate::config::{Budgets, Config, OutputMode};
use crate::metrics::{InvocationMetrics, MemoryTracker};
use crate::shim::{node_shim, python_shim};
use crate::snapshot::{compare_snapshot, load_snapshot, snapshot_path, write_snapshot};
use crate::util::{ensure_dir, read_to_string, resolve_dir_relative_to_config, snapshot_key};

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tokio::process::Command as TokioCommand;

/// Outcome of a single run (used for flaky detection).
#[derive(Debug, Clone)]
struct RunOutcome {
    run_index: u32,
    ok: bool,
    output: Value,
    metrics: InvocationMetrics,
    failures: Vec<String>,
}

/// Entry point from `main.rs`.
pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Init { language } => init_scaffold(language),
        Command::Run {
            file,
            config,
            fixture,
            assert,
            snapshot,
            repeat,
            budget_time,
            budget_mem,
        } => {
            run_action(
                file,
                config,
                fixture,
                assert,
                snapshot,
                repeat,
                budget_time,
                budget_mem,
            )
            .await
        }
    }
}

/// Execute an action with all checks enabled.
async fn run_action(
    action_file: PathBuf,
    config_path: PathBuf,
    fixture_override: Option<String>,
    assert_file: Option<PathBuf>,
    snapshot_enabled: bool,
    repeat: u32,
    budget_time_override: Option<u64>,
    budget_mem_override_mb: Option<u64>,
) -> Result<()> {
    let cfg = Config::load(&config_path)?;
    let action_file = action_file
        .canonicalize()
        .context("Unable to resolve action file path")?;

    // Resolve fixture
    let fixture_name = fixture_override.unwrap_or_else(|| cfg.fixtures.default.clone());
    let fixtures_dir = resolve_dir_relative_to_config(&config_path, &cfg.fixtures.dir)?;
    let fixture_path = fixtures_dir.join(&fixture_name);

    let event: Value = serde_json::from_str(&read_to_string(&fixture_path)?)?;

    // Load assertions if provided
    let assertions = if let Some(path) = assert_file.as_ref() {
        Some(load_assertions(path)?)
    } else {
        None
    };

    // Resolve budgets (config + CLI overrides)
    let budgets = resolve_budgets(cfg.budgets.clone(), budget_time_override, budget_mem_override_mb);

    // Snapshot setup (store snapshots next to config by default for predictability)
    let snap_key = snapshot_key(&action_file, &fixture_name);
    let snapshots_dir = resolve_dir_relative_to_config(&config_path, "snapshots")?;
    let snap_path = snapshot_path(&snapshots_dir, &snap_key);

    let mut snapshot_baseline: Option<Value> = None;
    if snapshot_enabled && snap_path.exists() {
        snapshot_baseline = Some(load_snapshot(&snap_path)?);
    }

    let runs_to_do = repeat.max(1);
    let mut outcomes: Vec<RunOutcome> = Vec::with_capacity(runs_to_do as usize);

    // Execute runs
    for i in 1..=runs_to_do {
        let (output, metrics) = invoke_once(&cfg, &action_file, &event).await?;
        let mut failures: Vec<String> = Vec::new();

        // Shim-level success
        let shim_ok = output
            .get("ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !shim_ok {
            let msg = output
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            failures.push(format!("Action error: {}", msg));
        }

        // Assertions
        if let Some(assert_map) = assertions.as_ref() {
            if let Err(e) = assert_json(&output, assert_map) {
                failures.push(format!("Assertion failed: {}", e));
            }
        }

        // Budgets
        if let Err(e) = check_budgets(
            metrics.duration_ms,
            metrics.max_rss_kb,
            &budgets,
        ) {
            failures.push(format!("Budget failed: {}", e));
        }



        // Snapshots
        if snapshot_enabled {
            if snapshot_baseline.is_none() && !snap_path.exists() {
                ensure_dir(&snapshots_dir)?;
                write_snapshot(&snap_path, &output)?;
                snapshot_baseline = Some(output.clone());
            } else if let Some(baseline) = snapshot_baseline.as_ref() {
                if let Err(e) = compare_snapshot(baseline, &output) {
                    failures.push(format!("Snapshot mismatch: {}", e));
                }
            }
        }

        let ok = failures.is_empty();
        outcomes.push(RunOutcome {
            run_index: i,
            ok,
            output,
            metrics,
            failures,
        });
    }

    // Flaky detection
    let flaky_report = detect_flakiness(&outcomes);

    // Emit final summary
    let summary = build_summary_json(&action_file, &fixture_name, &outcomes, &budgets, flaky_report);
    emit_output(&cfg, &summary)?;

    let all_ok = outcomes.iter().all(|o| o.ok);
    let flaky = summary
        .get("meta")
        .and_then(|m| m.get("flaky"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !all_ok || flaky {
        bail!("Run failed (see summary output above)");
    }

    Ok(())
}

/// Invoke the action once and collect metrics.
///
/// Important behaviour:
/// - The shim must emit a single JSON object as the **last non-empty line** of stdout.
/// - Any earlier stdout lines are treated as user logs and are printed through (so action prints show).
async fn invoke_once(
    cfg: &Config,
    action_file: &Path,
    event: &Value,
) -> Result<(Value, InvocationMetrics)> {
    let tmp = tempdir().context("Failed to create temp dir")?;

    // Write event.json
    let event_path = tmp.path().join("event.json");
    std::fs::write(&event_path, serde_json::to_vec_pretty(event)?)
        .context("Failed to write event.json")?;

    // Select runtime + shim
    let ext = action_file
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let (runtime_bin, shim_name, shim_code) = match ext.as_str() {
        "py" => (&cfg.runtime.python, "hs_python_runner.py", python_shim()),
        "js" | "mjs" | "cjs" => (&cfg.runtime.node, "hs_node_runner.mjs", node_shim()),
        _ => bail!("Unsupported action file extension: {}", ext),
    };

    let shim_path = tmp.path().join(shim_name);
    std::fs::write(&shim_path, shim_code).context("Failed to write runner shim")?;

    // Build process
    let mut cmd = TokioCommand::new(runtime_bin);
    cmd.arg(&shim_path)
        .arg(action_file)
        .arg(&event_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    for (k, v) in cfg.env.iter() {
        cmd.env(k, v);
    }

    let start = Instant::now();
    let child = cmd.spawn().context("Failed to spawn runtime")?;
    let pid = child.id().context("Failed to get child PID")?;

    // Track memory (best-effort; may be None on unsupported platforms)
    let mem_tracker = MemoryTracker::start(pid, Duration::from_millis(20));

    let output = child
        .wait_with_output()
        .await
        .context("Failed while waiting for action to complete")?;

    let duration_ms = start.elapsed().as_millis();
    let max_rss_kb = mem_tracker.stop_and_take();

    // If the process failed, show stdout to aid debugging.
    let stdout = String::from_utf8(output.stdout)
    .context("stdout not valid UTF-8")?;

    let parsed: Value = match serde_json::from_str(stdout.trim()) {
        Ok(v) => v,
        Err(_) => {
            // Only bail if:
            // - exit code != 0
            // - AND stdout is NOT valid JSON
            if !output.status.success() {
                bail!(
                    "Action process exited with {} and did not emit valid JSON.\nSTDOUT:\n{}",
                    output.status,
                    stdout
                );
            } else {
                bail!("Shim emitted invalid JSON:\n{}", stdout);
            }
        }
    };

    Ok((
        parsed,
        InvocationMetrics {
            duration_ms,
            max_rss_kb,
        },
    ))
}

/// Combine budgets from config + CLI overrides.
fn resolve_budgets(
    cfg_budgets: Option<Budgets>,
    duration_override: Option<u64>,
    memory_override_mb: Option<u64>,
) -> BudgetsResolved {
    let mut duration_ms = cfg_budgets.as_ref().and_then(|b| b.duration_ms);
    let mut memory_kb = cfg_budgets
        .as_ref()
        .and_then(|b| b.memory_mb)
        .map(|mb| mb * 1024);

    if let Some(d) = duration_override {
        duration_ms = Some(d);
    }
    if let Some(mb) = memory_override_mb {
        memory_kb = Some(mb * 1024);
    }

    BudgetsResolved {
        duration_ms,
        memory_kb,
    }
}

/// Detect flaky behaviour across runs.
fn detect_flakiness(outcomes: &[RunOutcome]) -> Option<String> {
    if outcomes.len() <= 1 {
        return None;
    }

    let mut reasons: Vec<String> = Vec::new();

    // Output mismatch
    let base = &outcomes[0].output;
    let mismatches: Vec<u32> = outcomes
        .iter()
        .skip(1)
        .filter(|o| &o.output != base)
        .map(|o| o.run_index)
        .collect();

    if !mismatches.is_empty() {
        reasons.push(format!(
            "Output mismatch vs run 1 on runs {:?}",
            mismatches
        ));
    }

    // Mixed pass/fail
    let any_ok = outcomes.iter().any(|o| o.ok);
    let any_fail = outcomes.iter().any(|o| !o.ok);
    if any_ok && any_fail {
        let failed: Vec<u32> = outcomes
            .iter()
            .filter(|o| !o.ok)
            .map(|o| o.run_index)
            .collect();
        reasons.push(format!(
            "Some runs failed while others passed (runs {:?})",
            failed
        ));
    }

    if reasons.is_empty() {
        None
    } else {
        Some(reasons.join(" | "))
    }
}

/// Build final summary JSON.
fn build_summary_json(
    action_file: &Path,
    fixture_name: &str,
    outcomes: &[RunOutcome],
    budgets: &BudgetsResolved,
    flaky_report: Option<String>,
) -> Value {
    let runs = outcomes.len() as u64;

    let durations: Vec<u128> = outcomes.iter().map(|o| o.metrics.duration_ms).collect();
    let duration_max = durations.iter().copied().max().unwrap_or(0);
    let duration_sum: u128 = durations.iter().copied().sum();
    let duration_avg = if runs > 0 {
        duration_sum / runs as u128
    } else {
        0
    };

    let mem_vals: Vec<u64> = outcomes
        .iter()
        .filter_map(|o| o.metrics.max_rss_kb)
        .collect();
    let max_rss_kb = mem_vals.iter().copied().max();

    let failures: Vec<Value> = outcomes
        .iter()
        .filter(|o| !o.ok)
        .map(|o| {
            serde_json::json!({
                "run": o.run_index,
                "failures": o.failures,
                "duration_ms": o.metrics.duration_ms,
                "max_rss_kb": o.metrics.max_rss_kb
            })
        })
        .collect();

    serde_json::json!({
        "ok": outcomes.iter().all(|o| o.ok) && flaky_report.is_none(),
        "action": action_file.to_string_lossy(),
        "fixture": fixture_name,
        "meta": {
            "runs": runs,
            "flaky": flaky_report.is_some(),
            "flaky_reason": flaky_report,
            "duration_ms_avg": duration_avg,
            "duration_ms_max": duration_max,
            "max_rss_mb": max_rss_kb.map(|kb| kb / 1024),
            "budgets": {
                "duration_ms": budgets.duration_ms,
                "memory_mb": budgets.memory_kb.map(|kb| kb / 1024)
            }
        },
        "failures": failures,
        "output": outcomes.last().map(|o| o.output.clone()).unwrap_or(Value::Null)
    })
}

/// Scaffold initial files for `init`.
fn init_scaffold(language: Option<String>) -> Result<()> {
    // config.yaml
    if !Path::new("config.yaml").exists() {
        std::fs::write("config.yaml", default_config_yaml())
            .context("Failed to write config.yaml")?;
        eprintln!("Created config.yaml");
    } else {
        eprintln!("config.yaml already exists (skipping)");
    }

    // fixtures/event.json
    if !Path::new("fixtures").exists() {
        ensure_dir(Path::new("fixtures"))?;
    }
    if !Path::new("fixtures/event.json").exists() {
        std::fs::write("fixtures/event.json", default_fixture_json())
            .context("Failed to write fixtures/event.json")?;
        eprintln!("Created fixtures/event.json");
    } else {
        eprintln!("fixtures/event.json already exists (skipping)");
    }

    // assertions.json
    if !Path::new("assertions.json").exists() {
        std::fs::write("assertions.json", default_assertions_json())
            .context("Failed to write assertions.json")?;
        eprintln!("Created assertions.json");
    } else {
        eprintln!("assertions.json already exists (skipping)");
    }

    // Optional action file
    if let Some(lang) = language {
        let lang = lang.to_lowercase();
        ensure_dir(Path::new("actions"))?;
        match lang.as_str() {
            "js" | "javascript" => {
                let p = Path::new("actions/action.js");
                if !p.exists() {
                    std::fs::write(p, default_action_js())
                        .context("Failed to write actions/action.js")?;
                    eprintln!("Created actions/action.js");
                } else {
                    eprintln!("actions/action.js already exists (skipping)");
                }
            }
            "py" | "python" => {
                let p = Path::new("actions/action.py");
                if !p.exists() {
                    std::fs::write(p, default_action_py())
                        .context("Failed to write actions/action.py")?;
                    eprintln!("Created actions/action.py");
                } else {
                    eprintln!("actions/action.py already exists (skipping)");
                }
            }
            _ => {
                // Unknown language => no-op (init should still succeed)
                eprintln!("Unknown language {:?} (skipping action scaffold)", lang);
            }
        }
    }

    Ok(())
}

fn default_assertions_json() -> &'static str {
    r#"
{
  "_example": {
    "callback.outputFields.success": true,
    "callback.outputFields.abc": 123
  }
}
"#
}


fn default_config_yaml() -> &'static str {
    r#"
fixtures:
  dir: fixtures
  default: event.json

env:
  HUBSPOT_TOKEN: "pat-your-token-here"
  HUBSPOT_BASE_URL: "https://api.hubapi.com"

runtime:
  node: node
  python: python3

output:
  mode: simple   # stdout | pretty | simple | file
  # file: results.json

# Optional budgets
# budgets:
#   duration_ms: 500
#   memory_mb: 64
"#
}

fn default_fixture_json() -> &'static str {
    r#"
{
  "object": {
    "objectType": "CONTACT",
    "objectId": 123456
  },
  "inputFields": {},
  "fields": {},
  "portalId": 12345678
}
"#
}

fn default_action_js() -> &'static str {
    r##"
exports.main = async (event, callback) => {
  try {
    console.log("Event:", JSON.stringify(event, null, 2));

    callback({
      outputFields: {
        success: true
      }
    });
  } catch (err) {
    console.error(err);
    throw err;
  }
};
"##
}

fn default_action_py() -> &'static str {
    r#"
import json

def main(event):
    try:
        print("Event:", json.dumps(event, indent=2))
        return {
            "outputFields": {
                "success": True
            }
        }
    except Exception as e:
        print(e)
        raise
"#
}

fn emit_output(cfg: &Config, summary: &Value) -> Result<()> {
    match cfg.output.mode {
        OutputMode::Stdout => {
            println!("{}", serde_json::to_string(summary)?);
        }

        OutputMode::Pretty => {
            println!("{}", serde_json::to_string_pretty(summary)?);
        }

        OutputMode::Simple => {
            emit_simple(summary);
        }

        OutputMode::File => {
            let path = cfg
                .output
                .file
                .as_deref()
                .unwrap_or("hsemulate-output.json");

            std::fs::write(path, serde_json::to_vec_pretty(summary)?)?;
            eprintln!("Output written to {}", path);
        }
    }

    Ok(())
}

// Simple ANSI colour helpers (no dependencies)
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";

fn color(text: &str, c: &str) -> String {
    format!("{c}{text}{RESET}")
}

fn bold(text: &str) -> String {
    format!("{BOLD}{text}{RESET}")
}

fn emit_simple(summary: &Value) {
    let ok = summary.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let meta = &summary["meta"];

    println!("{}", DIM.to_string() + "--------------------------------" + RESET);

    if ok {
        println!(
            "{} {}\n",
            color("✔", GREEN),
            bold(&color("Action succeeded", GREEN))
        );
    } else {
        println!(
            "{} {}\n",
            color("✖", RED),
            bold(&color("Action failed", RED))
        );
    }

    println!(
        "{} {}",
        bold("Action:"),
        summary["action"].as_str().unwrap_or("-")
    );
    println!(
        "{} {}",
        bold("Fixture:"),
        summary["fixture"].as_str().unwrap_or("-")
    );
    println!();

    println!(
        "{} {}",
        bold("Runs:"),
        meta["runs"].as_u64().unwrap_or(1)
    );

    println!(
        "{} {} ms",
        bold("Duration:"),
        meta["duration_ms_max"].as_u64().unwrap_or(0)
    );

    if let Some(mem) = meta.get("max_rss_mb").and_then(|v| v.as_u64()) {
        println!(
            "{} {} MB {}",
            bold("Memory:"),
            mem,
            DIM.to_string() + "(peak)" + RESET
        );
    }

    if !ok {
        println!("\n{}", bold(&color("Failures:", RED)));

        if let Some(failures) = summary["failures"].as_array() {
            for f in failures {
                if let Some(list) = f["failures"].as_array() {
                    for msg in list {
                        println!(
                            "  {} {}",
                            color("•", RED),
                            msg.as_str().unwrap_or("Unknown failure")
                        );
                    }
                }
            }
        }

        if let Some(err) = summary["output"]["error"].as_object() {
            let typ = err.get("type").and_then(|v| v.as_str()).unwrap_or("error");
            let msg = err.get("message").and_then(|v| v.as_str()).unwrap_or("");

            println!("\n{}", bold(&color("Error details:", YELLOW)));
            println!(
                "  {} {}",
                color(&format!("[{}]", typ), YELLOW),
                msg
            );

            if let Some(stack) = err.get("stack").and_then(|v| v.as_str()) {
                for line in stack.lines().take(3) {
                    println!("    {}", DIM.to_string() + line + RESET);
                }
            }
        }
    }
}
