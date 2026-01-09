// src/runner.rs

use crate::checks::{assert_json, check_budgets, BudgetsResolved};
use crate::cli::{Cli, Command};
use crate::config::{Assertion, Budgets, Config, Mode, OutputMode};
use crate::metrics::{InvocationMetrics, MemoryTracker};
use crate::shim::{node_shim, python_shim};
use crate::snapshot::{compare_snapshot, load_snapshot, snapshot_path, write_snapshot};
use crate::util::{ensure_dir, read_to_string, snapshot_key};

use anyhow::{bail, Context, Result};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tokio::process::Command as TokioCommand;

#[derive(Debug)]
struct ExecSummary {
    ok: bool,
    failures: Vec<String>,
    runs: u64,
}

/// Entry point from `main.rs`.
pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Init { language } => init_scaffold(language),

        Command::Test { config } => run_test_mode(config).await,

        Command::Run {
            config,
            action,
            fixture,
            assert,
            snapshot,
            watch,
            repeat,
            budget_time,
            budget_mem,
        } => {
            let mut cfg = Config::load(&config)?;

            // CLI overrides
            if let Some(path) = action {
                cfg.action.entry = path.to_string_lossy().to_string();
            }
            if !fixture.is_empty() {
                cfg.fixtures = fixture
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect();
            }
            if snapshot {
                cfg.snapshots.enabled = true;
            }
            if watch {
                cfg.watch = true;
            }
            if let Some(r) = repeat {
                cfg.repeat = r;
            }
            if budget_time.is_some() || budget_mem.is_some() {
                cfg.budgets = Some(resolve_budgets(cfg.budgets.clone(), budget_time, budget_mem));
            }

            if cfg.watch {
                execute_with_watch(config, assert).await
            } else {
                let summary = execute(cfg, assert).await?;

                if !summary.ok {
                    for f in &summary.failures {
                        eprintln!("✖ {}", f);
                    }
                    bail!("Run failed");
                }
                Ok(())
            }
        }
    }
}

/* ---------------- test mode (CI-first) ---------------- */

async fn run_test_mode(config_arg: PathBuf) -> Result<()> {
    // If the user explicitly passed a non-default config path, just run that config.
    // If they left it as default `config.yaml`, discover all configs recursively.
    let configs = if config_arg == PathBuf::from("config.yaml") {
        discover_configs()?
    } else {
        vec![config_arg]
    };

    let mut any_fail = false;
    let mut results: Vec<Value> = Vec::new();

    for cfg_path in configs {
        let mut cfg = Config::load(&cfg_path)?;
        cfg.mode = Mode::Ci;
        cfg.snapshots.enabled = true;

        let summary = execute(cfg, None).await?;
        if !summary.ok {
            any_fail = true;
        }

        results.push(serde_json::json!({
            "config": cfg_path.to_string_lossy(),
            "ok": summary.ok,
            "runs": summary.runs,
            "failures": summary.failures,
        }));
    }

    // CI JSON emitter: always print one stable JSON blob in test mode.
    let out = serde_json::json!({
        "ok": !any_fail,
        "results": results
    });
    println!("{}", serde_json::to_string(&out)?);

    if any_fail {
        bail!("One or more configs failed");
    }

    Ok(())
}

fn discover_configs() -> Result<Vec<PathBuf>> {
    // Requires dependency: walkdir = "2.5"
    let mut configs = Vec::new();

    for entry in walkdir::WalkDir::new(".") {
        let entry = entry?;
        if entry.file_name() == "config.yaml" {
            configs.push(entry.path().to_path_buf());
        }
    }

    if configs.is_empty() {
        bail!("No config.yaml files found");
    }

    // Stable ordering is nice in CI
    configs.sort();
    Ok(configs)
}

/* ---------------- watch mode ---------------- */

async fn execute_with_watch(config_path: PathBuf, assertion_file: Option<PathBuf>) -> Result<()> {
    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher =
        Watcher::new(tx, notify::Config::default())
            .context("Failed to initialise file watcher")?;

    // Always watch the specific config file used
    watcher.watch(&config_path, RecursiveMode::NonRecursive)?;

    // Initial load so we can watch action + fixtures too
    let cfg0 = Config::load(&config_path)?;

    watcher.watch(Path::new(&cfg0.action.entry), RecursiveMode::NonRecursive)?;
    for f in &cfg0.fixtures {
        watcher.watch(Path::new(f), RecursiveMode::NonRecursive)?;
    }

    loop {
        clear_screen();

        // Reload config each run so edits to config.yaml apply immediately
        let cfg = Config::load(&config_path)?;

        match execute(cfg, assertion_file.clone()).await {
            Ok(summary) => {
                // In watch mode, print a compact JSON line in CI mode, otherwise just show failures.
                if matches!(summary.ok, true) {
                    eprintln!("OK");
                } else {
                    eprintln!("FAILED:");
                    for f in summary.failures {
                        eprintln!("  - {}", f);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
            }
        }

        // Block until something changes
        let _ = rx.recv();
    }
}

fn clear_screen() {
    print!("\x1b[2J\x1b[H");
    let _ = std::io::stdout().flush();
}

/* ---------------- core execution ---------------- */

async fn execute(cfg: Config, assertion_file: Option<PathBuf>) -> Result<ExecSummary> {
    let action_file = PathBuf::from(&cfg.action.entry)
        .canonicalize()
        .context("Unable to resolve action entry")?;

    let assertions_override = if let Some(path) = assertion_file {
        Some(load_external_assertions(&path)?)
    } else if let Some(path) = cfg.assertions_file.as_ref() {
        Some(load_external_assertions(Path::new(path))?)
    } else {
        None
    };

    let runs = cfg.repeat.max(1) as u64;
    let total_runs = runs * cfg.fixtures.len() as u64;
    let write_file = matches!(cfg.output.mode, OutputMode::File);
    let emit_stdout = !matches!(cfg.mode, Mode::Ci) && !write_file;
    let use_color = should_use_color();
    let output_file = if write_file {
        Some(PathBuf::from(
            cfg.output
                .file
                .as_ref()
                .context("output.file must be set when output.mode = file")?,
        ))
    } else {
        None
    };
    let mut file_outputs: Vec<Value> = Vec::new();

    let mut failures_all: Vec<String> = Vec::new();

    for fixture in &cfg.fixtures {
        let event: Value = serde_json::from_str(&read_to_string(Path::new(fixture))?)
            .with_context(|| format!("Fixture is not valid JSON: {}", fixture))?;

        let snap_key = snapshot_key(&action_file, fixture);

        // Snapshots stored in ./snapshots by default
        let snap_path = snapshot_path(Path::new("snapshots"), &snap_key);

        let mut baseline = if cfg.snapshots.enabled && snap_path.exists() {
            Some(load_snapshot(&snap_path)?)
        } else {
            None
        };

        for run_idx in 0..runs {
            let (output, metrics) = invoke_once(&cfg, &action_file, &event).await?;
            let mut failures = Vec::new();

            if !output.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                failures.push("Action returned ok=false".to_string());
            }

            // Assertions: CLI override wins, else config assertions
            let assertion_source = assertions_override.as_ref().unwrap_or(&cfg.assertions);

            if !assertion_source.is_empty() {
                if let Err(e) = assert_json(&output, assertion_source) {
                    failures.push(format!("Assertion failed: {}", e));
                }
            }

            // Budgets
            if let Some(b) = &cfg.budgets {
                if let Err(e) = check_budgets(
                    metrics.duration_ms,
                    metrics.max_rss_kb,
                    &BudgetsResolved {
                        duration_ms: b.duration_ms,
                        memory_kb: b.memory_mb.map(|mb| mb * 1024),
                    },
                ) {
                    failures.push(format!("Budget failed: {}", e));
                }
            }

            // Snapshots
            if cfg.snapshots.enabled {
                if baseline.is_none() {
                    ensure_dir(Path::new("snapshots"))?;
                    write_snapshot(&snap_path, &output)?;
                    baseline = Some(output.clone());
                } else if let Some(b) = &baseline {
                    // If you implement snapshot ignore rules, update compare_snapshot signature:
                    // compare_snapshot(b, &output, &cfg.snapshots.ignore)?
                    if let Err(e) = compare_snapshot(b, &output) {
                        failures.push(format!(
    "Snapshot mismatch ({}): {}",
    snap_path.display(),
    e
));
                    }
                }
            }

            let render_ctx = RenderContext {
                action_file: &action_file,
                fixture,
                run_idx,
                runs,
                output: &output,
                metrics: &metrics,
                failures: &failures,
            };
            let envelope = build_output_envelope(&render_ctx);

            if emit_stdout {
                let rendered =
                    render_output(&cfg.output.mode, &render_ctx, &envelope, use_color)?;
                println!("{}", rendered);
            }

            if write_file {
                file_outputs.push(envelope);
            }

            if !failures.is_empty() {
                // Include fixture context for diagnostics
                for f in failures {
                    failures_all.push(format!("[{}] {}", fixture, f));
                }

                // Fail fast in CI
                if matches!(cfg.mode, Mode::Ci) {
                    return Ok(ExecSummary {
                        ok: false,
                        failures: failures_all,
                        runs,
                    });
                }
            }
        }
    }

    if let Some(path) = output_file {
        let payload = if total_runs > 1 {
            Value::Array(file_outputs)
        } else {
            file_outputs.into_iter().next().unwrap_or(Value::Null)
        };
        write_output_file(&path, &payload)?;
    }

    Ok(ExecSummary {
        ok: failures_all.is_empty(),
        failures: failures_all,
        runs,
    })
}

/* ---------------- invocation ---------------- */

async fn invoke_once(
    cfg: &Config,
    action_file: &Path,
    event: &Value,
) -> Result<(Value, InvocationMetrics)> {
    let tmp = tempdir().context("Failed to create temp dir")?;

    // Write event.json for shim
    let event_path = tmp.path().join("event.json");
    std::fs::write(&event_path, serde_json::to_vec_pretty(event)?)
        .context("Failed to write event.json")?;

    // Select runtime + shim by extension
    let ext = action_file
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let (runtime, shim_name, shim_code) = match ext.as_str() {
        "py" => (&cfg.runtime.python, "hs_python_runner.py", python_shim()),
        "js" | "mjs" | "cjs" => (&cfg.runtime.node, "hs_node_runner.mjs", node_shim()),
        _ => bail!("Unsupported action file extension: {}", ext),
    };

    let shim_path = tmp.path().join(shim_name);
    std::fs::write(&shim_path, shim_code).context("Failed to write runner shim")?;

    let mut cmd = TokioCommand::new(runtime);
    cmd.arg(&shim_path)
        .arg(action_file)
        .arg(&event_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    for (k, v) in &cfg.env {
        cmd.env(k, v);
    }

    let start = Instant::now();
    let child = cmd.spawn().context("Failed to spawn runtime")?;

    let pid = child.id().context("Failed to get child PID")?;
    let mem = MemoryTracker::start(pid, Duration::from_millis(20));

    let output = child
        .wait_with_output()
        .await
        .context("Failed while waiting for action to complete")?;

    let duration_ms = start.elapsed().as_millis();
    let max_rss_kb = mem.stop_and_take();

    let stdout = String::from_utf8(output.stdout).context("stdout not valid UTF-8")?;
    let parsed: Value = serde_json::from_str(stdout.trim())
        .context("Shim did not emit valid JSON")?;

    Ok((
        parsed,
        InvocationMetrics {
            duration_ms,
            max_rss_kb,
        },
    ))
}

/* ---------------- utilities ---------------- */

fn load_external_assertions(path: &Path) -> Result<BTreeMap<String, Assertion>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read assertions file: {:?}", path))?;
    let map = serde_json::from_str(&raw).context("Failed to parse assertions JSON")?;
    Ok(map)
}

fn resolve_budgets(base: Option<Budgets>, dur: Option<u64>, mem: Option<u64>) -> Budgets {
    let mut b = base.unwrap_or(Budgets {
        duration_ms: None,
        memory_mb: None,
    });

    if let Some(d) = dur {
        b.duration_ms = Some(d);
    }
    if let Some(m) = mem {
        b.memory_mb = Some(m);
    }

    b
}

struct RenderContext<'a> {
    action_file: &'a Path,
    fixture: &'a str,
    run_idx: u64,
    runs: u64,
    output: &'a Value,
    metrics: &'a InvocationMetrics,
    failures: &'a [String],
}

fn build_output_envelope(ctx: &RenderContext<'_>) -> Value {
    let mut meta = serde_json::Map::new();
    meta.insert(
        "action".to_string(),
        Value::String(ctx.action_file.display().to_string()),
    );
    meta.insert("fixture".to_string(), Value::String(ctx.fixture.to_string()));
    if ctx.runs > 1 {
        meta.insert("run".to_string(), Value::Number((ctx.run_idx + 1).into()));
        meta.insert("runs".to_string(), Value::Number(ctx.runs.into()));
    }

    let duration_value = u64::try_from(ctx.metrics.duration_ms)
        .map(Value::from)
        .unwrap_or_else(|_| Value::String(ctx.metrics.duration_ms.to_string()));
    meta.insert("duration_ms".to_string(), duration_value);

    let mem_value = ctx
        .metrics
        .max_rss_kb
        .map(Value::from)
        .unwrap_or(Value::Null);
    meta.insert("max_rss_kb".to_string(), mem_value);

    let mut envelope = serde_json::Map::new();
    envelope.insert(
        "ok".to_string(),
        Value::Bool(ctx.failures.is_empty()),
    );
    envelope.insert("meta".to_string(), Value::Object(meta));
    envelope.insert("output".to_string(), ctx.output.clone());
    if !ctx.failures.is_empty() {
        let failures = ctx
            .failures
            .iter()
            .cloned()
            .map(Value::String)
            .collect::<Vec<_>>();
        envelope.insert("failures".to_string(), Value::Array(failures));
    }

    Value::Object(envelope)
}

fn select_simple_output<'a>(output: &'a Value) -> &'a Value {
    if output.get("ok").and_then(|v| v.as_bool()) == Some(false) {
        if let Some(err) = output.get("error") {
            if !err.is_null() {
                return err;
            }
        }
    }

    if let Some(callback) = output.get("callback") {
        if !callback.is_null() {
            return callback;
        }
    }

    if let Some(result) = output.get("result") {
        if !result.is_null() {
            return result;
        }
    }

    output
}

fn render_output(
    mode: &OutputMode,
    ctx: &RenderContext<'_>,
    envelope: &Value,
    use_color: bool,
) -> Result<String> {
    match mode {
        OutputMode::Stdout => serde_json::to_string(envelope)
            .context("Failed to format output as JSON"),
        OutputMode::Pretty => serde_json::to_string_pretty(envelope)
            .context("Failed to format output as pretty JSON"),
        OutputMode::Simple => format_simple_output(ctx, use_color),
        OutputMode::File => bail!("output.mode = file should be handled separately"),
    }
}

fn format_simple_output(ctx: &RenderContext<'_>, use_color: bool) -> Result<String> {
    let ok = ctx.failures.is_empty();
    let status = if ok { "OK" } else { "FAIL" };
    let status = paint(status, if ok { "32" } else { "31" }, use_color);

    let mut out = String::new();
    out.push_str(&format!(
        "{} {}\n",
        status,
        ctx.action_file.display()
    ));
    out.push_str(&format!("fixture: {}\n", ctx.fixture));
    if ctx.runs > 1 {
        out.push_str(&format!("run: {}/{}\n", ctx.run_idx + 1, ctx.runs));
    }

    let duration_ms = u64::try_from(ctx.metrics.duration_ms)
        .map(|v| v.to_string())
        .unwrap_or_else(|_| ctx.metrics.duration_ms.to_string());
    out.push_str(&format!("time: {}ms\n", duration_ms));

    let mem = ctx
        .metrics
        .max_rss_kb
        .map(|v| format!("{}kb", v))
        .unwrap_or_else(|| "n/a".to_string());
    out.push_str(&format!("memory: {}\n", mem));

    if !ctx.failures.is_empty() {
        out.push_str("failures:\n");
        for failure in ctx.failures {
            out.push_str(&format!("- {}\n", failure));
        }
    }

    let simple = select_simple_output(ctx.output);
    if simple != &Value::Null {
        out.push_str("output:\n");
        let rendered = serde_json::to_string_pretty(simple)
            .context("Failed to format simple output")?;
        out.push_str(&rendered);
        out.push('\n');
    }

    Ok(out.trim_end().to_string())
}

fn should_use_color() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stdout().is_terminal()
}

fn paint(text: &str, color: &str, use_color: bool) -> String {
    if use_color {
        format!("\x1b[{}m{}\x1b[0m", color, text)
    } else {
        text.to_string()
    }
}

fn write_output_file(path: &Path, payload: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            ensure_dir(parent)?;
        }
    }

    let bytes = serde_json::to_vec_pretty(payload)
        .context("Failed to serialize output JSON")?;

    std::fs::write(path, bytes)
        .with_context(|| format!("Failed to write output file {:?}", path))?;

    Ok(())
}

/* -------------------------------------------------
   init_scaffold + defaults (kept from your file)
-------------------------------------------------- */

fn init_scaffold(language: Option<String>) -> Result<()> {
    if !Path::new("config.yaml").exists() {
        std::fs::write("config.yaml", default_config_yaml())?;
        eprintln!("Created config.yaml");
    } else {
        eprintln!("config.yaml already exists (skipping)");
    }

    if !Path::new("fixtures").exists() {
        ensure_dir(Path::new("fixtures"))?;
    }
    if !Path::new("fixtures/event.json").exists() {
        std::fs::write("fixtures/event.json", default_fixture_json())?;
        eprintln!("Created fixtures/event.json");
    } else {
        eprintln!("fixtures/event.json already exists (skipping)");
    }

    if !Path::new("assertions.json").exists() {
        std::fs::write("assertions.json", default_assertions_json())?;
        eprintln!("Created assertions.json");
    } else {
        eprintln!("assertions.json already exists (skipping)");
    }

    if let Some(lang) = language {
        let lang = lang.to_lowercase();
        ensure_dir(Path::new("actions"))?;
        match lang.as_str() {
            "js" | "javascript" => {
                let p = Path::new("actions/action.js");
                if !p.exists() {
                    std::fs::write(p, default_action_js())?;
                    eprintln!("Created actions/action.js");
                }
            }
            "python" | "py" => {
                let p = Path::new("actions/action.py");
                if !p.exists() {
                    std::fs::write(p, default_action_py())?;
                    eprintln!("Created actions/action.py");
                }
            }
            _ => eprintln!("Unknown language {:?} (skipping action scaffold)", lang),
        }
    }

    Ok(())
}

fn default_assertions_json() -> &'static str {
    r#"
{
  "callback.outputFields.success": { "eq": true },
  "language": { "regex": "node|python" }
}
"#
}

fn default_config_yaml() -> &'static str {
    r#"
version: 1

action:
  type: js
  entry: actions/action.js

fixtures:
  - fixtures/event.json

env:
  HUBSPOT_TOKEN: "pat-your-token-here"
  HUBSPOT_BASE_URL: "https://api.hubapi.com"

runtime:
  node: node
  python: python3

output:
  mode: simple # simple | pretty | stdout | file
  # file: results.json

assertions_file: assertions.json

snapshots:
  enabled: true
  ignore:
    - output.timestamp
    - meta.runId
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
    callback({ outputFields: { success: true } });
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
        return { "outputFields": { "success": True } }
    except Exception as e:
        print(e)
        raise
"#
}
