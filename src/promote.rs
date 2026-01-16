// src/promote.rs

use crate::config::Config;
use crate::util::read_to_string;

use anyhow::{bail, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const HUBSPOT_BASE_URL: &str = "https://api.hubapi.com";

/// Entry point for `hsemulate promote <target> [--force]`.
pub async fn handle(target: String, force: bool, config_path: PathBuf) -> Result<()> {
    // 1) Load cicd.yaml first
    let cicd = load_cicd_config(Path::new(".hsemulator/cicd.yaml"))
        .context("Failed to load .hsemulator/cicd.yaml")?;

    if cicd.version != 1 {
        bail!(
            "Unsupported cicd.yaml version: {} (expected 1)",
            cicd.version
        );
    }

    // 2) Resolve HUBSPOT_TOKEN (env preferred; yaml allowed)
    let token_from_env = std::env::var("HUBSPOT_TOKEN").ok();

    let token = match token_from_env {
        Some(v) => v,
        None => {
            let t = cicd
                .hubspot
                .as_ref()
                .and_then(|h| h.token.as_ref())
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "No HubSpot token available.\n\
                        \n\
                        Promotion requires a HubSpot private app token.\n\
                        Provide it using ONE of the following:\n\
                        • Environment variable (recommended):\n\
                            export HUBSPOT_TOKEN=pat-...\n\
                        • cicd.yaml (local only):\n\
                            hubspot:\n\
                                token: pat-...\n"
                    )
                })?;

            eprintln!(
                "WARNING: Using HubSpot token from cicd.yaml. This is insecure and should only be used locally."
            );
            t
        }
    };

    // 3) Load target
    let t = cicd.targets.get(&target).with_context(|| {
        let available = cicd.targets.keys().cloned().collect::<Vec<_>>().join(", ");
        format!(
            "Target '{}' not found in cicd.yaml.\n\
                Available targets: {}",
            target, available
        )
    })?;

    // In force mode, only selector + workflow ID are required (plus HUBSPOT_TOKEN).
    // In non-force mode, we also enforce last-test.json + safety constraints (if present).
    validate_target_minimum(t, force)?;

    // 3) If not forced, enforce test gate from .hsemulator/last-test.json
    if !force {
        let last = load_last_test(Path::new(".hsemulator/last-test.json")).with_context(|| {
            "Promotion is test-gated.\n\
            Missing .hsemulator/last-test.json.\n\
            \n\
            Run:\n\
            hsemulate test\n\
            \n\
            Or bypass the test gate explicitly:\n\
            hsemulate promote <target> --force"
                .to_string()
        })?;

        enforce_last_test(&last, t)?;
    }

    // 4) Load local action code to promote (from config.yaml -> action.entry)
    let action_code = load_action_source(&config_path).with_context(|| {
        format!(
            "Failed to load action source via config at {:?}",
            config_path
        )
    })?;

    // 5) Build hash + inject marker comment
    let canonical_source = strip_hash_marker(&action_code);
    let hash = sha256_hex(canonical_source.as_bytes());
    let promoted_source = inject_hash_marker(&canonical_source, &hash);

    // 6) Fetch workflow (revision-safe)
    let client = reqwest::Client::new();
    let headers = hubspot_headers(&token)?;

    let flow = hubspot_get_flow(&client, &headers, &t.workflow_id).await?;

    // 7) Locate target action deterministically
    let action_index = find_target_action_index(&flow, &t.selector)?;

    // 8) Drift guard (checksum comment) — fail in non-force if mismatch; warn otherwise
    {
        let existing_source = get_action_source_code(&flow, action_index)?;
        if let Some(existing_hash) = extract_hash_marker(&existing_source) {
            if existing_hash == hash {
                eprintln!(
                    "Action already up to date (hash {}). No changes required.",
                    hash
                );
                return Ok(());
            }

            // Hash differs → this is a normal promotion update
            eprintln!("Updating action: {} → {}", existing_hash, hash);
        } else {
            // No marker = unknown origin
            if !force {
                bail!(
                    "Refusing to overwrite action.\n\
                    \n\
                    Reason: The target CUSTOM_CODE action does not appear to be managed by hsemulator\n\
                    (missing hsemulator-sha marker).\n\
                    \n\
                    This usually means the action was:\n\
                    • Created manually in HubSpot, or\n\
                    • Managed by another tool or user\n\
                    \n\
                    To take ownership anyway, re-run with:\n\
                    hsemulate promote <target> --force"
                );
            }
            eprintln!("WARNING: Overwriting action with no hash marker due to --force.");
        }
    }

    // 9) Apply mutation (sourceCode [+ runtime if specified]) and PUT with revision guard
    let dry_run = t.deploy.as_ref().and_then(|d| d.dry_run).unwrap_or(false);

    let runtime_to_set = t.runtime.clone(); // optional (force mode does not require runtime)

    let updated_flow = build_updated_flow_payload(
        &flow,
        action_index,
        &promoted_source,
        runtime_to_set.as_deref(),
    )?;

    if dry_run {
        eprintln!(
            "Dry-run enabled (cicd.yaml deploy.dry_run: true). No changes will be sent to HubSpot."
        );
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "target": target,
                "workflow_id": t.workflow_id,
                "selector": {
                    "type": t.selector.selector_type,
                    "value": t.selector.value,
                },
                "new_hash": hash,
                "action_index": action_index,
            }))?
        );
        return Ok(());
    }

    let put_result = hubspot_put_flow(&client, &headers, &t.workflow_id, &updated_flow).await?;

    // 10) Output success summary (machine readable)
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "target": target,
            "workflow_id": t.workflow_id,
            "new_hash": hash,
            "revision_id_before": flow.get("revisionId").cloned().unwrap_or(JsonValue::Null),
            "revision_id_after": put_result.get("revisionId").cloned().unwrap_or(JsonValue::Null),
        }))?
    );

    Ok(())
}

/* ---------------- config models ---------------- */

#[derive(Debug, Deserialize)]
struct CicdConfig {
    version: u32,
    targets: BTreeMap<String, CicdTarget>,

    #[serde(default)]
    hubspot: Option<CicdHubSpot>,
}

#[derive(Debug, Deserialize)]
struct CicdTarget {
    // Optional in schema but required for promotion always (both modes).
    workflow_id: String,

    selector: CicdSelector,

    // Optional: in force mode user said "nothing else required"
    runtime: Option<String>,

    safety: Option<CicdSafety>,
    deploy: Option<CicdDeploy>,

    #[allow(dead_code)]
    portal: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CicdSelector {
    #[serde(rename = "type")]
    selector_type: String,
    value: String,
    require_unique: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CicdSafety {
    require_clean_tests: Option<bool>,
    require_snapshot_match: Option<bool>,
    max_duration_ms: Option<u64>,
    max_memory_mb: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct CicdDeploy {
    #[allow(dead_code)]
    mode: Option<String>,

    dry_run: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct LastTestResult {
    ok: bool,
    snapshots_ok: bool,
    max_duration_ms: Option<u128>,
    max_memory_kb: Option<u64>,
    run_at: String,
}

#[derive(Debug, Deserialize)]
struct CicdHubSpot {
    token: Option<String>,
}

/* ---------------- file loading ---------------- */

fn load_cicd_config(path: &Path) -> Result<CicdConfig> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read cicd config at {:?}", path))?;
    let cfg: CicdConfig = serde_yaml::from_str(&raw).context("Failed to parse cicd.yaml")?;
    Ok(cfg)
}

fn load_last_test(path: &Path) -> Result<LastTestResult> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read last test result at {:?}", path))?;
    let parsed: LastTestResult =
        serde_json::from_str(&raw).context("Failed to parse last-test.json")?;
    Ok(parsed)
}

/// Loads action source via config.yaml's `action.entry`.
fn load_action_source(config_path: &Path) -> Result<String> {
    let cfg = Config::load(config_path)?;
    let action = cfg.action.as_ref().expect("config validated");
    let entry = PathBuf::from(&action.entry);
    let code = read_to_string(&entry)
        .with_context(|| format!("Failed to read action.entry at {:?}", entry))?;
    Ok(code)
}

/* ---------------- validation ---------------- */

fn validate_target_minimum(t: &CicdTarget, force: bool) -> Result<()> {
    if t.workflow_id.trim().is_empty() {
        bail!("cicd.yaml target.workflow_id must be set");
    }

    // Selector requirements (always required)
    if t.selector.selector_type.trim().is_empty() || t.selector.value.trim().is_empty() {
        bail!("cicd.yaml target.selector.type and target.selector.value must be set");
    }
    if t.selector.selector_type != "secret" {
        bail!(
            "Unsupported selector type '{}'. Only 'secret' is supported currently.",
            t.selector.selector_type
        );
    }

    // In non-force mode, require safety gates to be consistent if set
    if !force {
        if let Some(s) = &t.safety {
            if let Some(true) = s.require_clean_tests {
                // enforced by last-test.json ok
            }
            if let Some(true) = s.require_snapshot_match {
                // enforced by last-test.json snapshots_ok
            }
        }
    }

    Ok(())
}

fn enforce_last_test(last: &LastTestResult, t: &CicdTarget) -> Result<()> {
    let safety = t.safety.as_ref();

    // Defaults: if safety is absent, we still require passing tests + snapshots for promotion.
    // This keeps the feature "test-gated" by default.
    let require_clean = safety.and_then(|s| s.require_clean_tests).unwrap_or(true);

    let require_snapshots = safety
        .and_then(|s| s.require_snapshot_match)
        .unwrap_or(true);

    if require_clean && !last.ok {
        bail!(
            "Promotion blocked by safety gate.\n\
 \n\
 Last test run FAILED.\n\
 Run time: {}\n\
 \n\
 Fix the failing tests and re-run:\n\
   hsemulate test\n\
 \n\
 Or bypass safety checks explicitly:\n\
   hsemulate promote <target> --force",
            last.run_at
        );
    }

    if require_snapshots && !last.snapshots_ok {
        bail!(
            "Refusing to promote: snapshot mismatches detected (last-test.json snapshots_ok=false, run_at={})",
            last.run_at
        );
    }

    if let Some(max_ms) = safety.and_then(|s| s.max_duration_ms) {
        if let Some(actual) = last.max_duration_ms {
            if actual > (max_ms as u128) {
                bail!(
                    "Refusing to promote: duration {}ms exceeds safety max_duration_ms {}ms",
                    actual,
                    max_ms
                );
            }
        }
    }

    if let Some(max_mb) = safety.and_then(|s| s.max_memory_mb) {
        if let Some(actual_kb) = last.max_memory_kb {
            let max_kb = max_mb * 1024;
            if actual_kb > max_kb {
                bail!(
                    "Refusing to promote: memory {}MB exceeds safety max_memory_mb {}MB",
                    actual_kb / 1024,
                    max_mb
                );
            }
        }
    }

    Ok(())
}

/* ---------------- HubSpot HTTP ---------------- */

fn hubspot_headers(token: &str) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();

    let auth_val = format!("Bearer {}", token);
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth_val)
            .context("Invalid HUBSPOT_TOKEN for Authorization header")?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Ok(headers)
}

async fn hubspot_get_flow(
    client: &reqwest::Client,
    headers: &HeaderMap,
    workflow_id: &str,
) -> Result<JsonValue> {
    let url = format!("{}/automation/v4/flows/{}", HUBSPOT_BASE_URL, workflow_id);
    let resp = client
        .get(url)
        .headers(headers.clone())
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .context("Failed to call HubSpot GET flow")?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        bail!("HubSpot GET flow failed: {} {}", status, text);
    }

    let flow: JsonValue =
        serde_json::from_str(&text).context("HubSpot GET flow returned invalid JSON")?;
    Ok(flow)
}

async fn hubspot_put_flow(
    client: &reqwest::Client,
    headers: &HeaderMap,
    workflow_id: &str,
    payload: &JsonValue,
) -> Result<JsonValue> {
    let url = format!("{}/automation/v4/flows/{}", HUBSPOT_BASE_URL, workflow_id);
    let resp = client
        .put(url)
        .headers(headers.clone())
        .json(payload)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .context("Failed to call HubSpot PUT flow")?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        bail!("HubSpot PUT flow failed: {} {}", status, text);
    }

    let out: JsonValue =
        serde_json::from_str(&text).context("HubSpot PUT flow returned invalid JSON")?;
    Ok(out)
}

/* ---------------- action selection ---------------- */

fn find_target_action_index(flow: &JsonValue, selector: &CicdSelector) -> Result<usize> {
    let actions = flow
        .get("actions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Workflow JSON missing 'actions' array"))?;

    let mut matches: Vec<usize> = Vec::new();
    for (idx, a) in actions.iter().enumerate() {
        let typ_ok = a.get("type").and_then(|v| v.as_str()) == Some("CUSTOM_CODE");
        if !typ_ok {
            continue;
        }

        let empty: Vec<JsonValue> = Vec::new();

        let secret_names = a
            .get("secretNames")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty)
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>();

        if secret_names.iter().any(|s| *s == selector.value.as_str()) {
            matches.push(idx);
        }
    }

    if matches.is_empty() {
        bail!(
            "No CUSTOM_CODE action found with secretNames containing '{}'",
            selector.value
        );
    }

    let require_unique = selector.require_unique.unwrap_or(true);
    if require_unique && matches.len() != 1 {
        bail!(
            "Selector '{}' matched {} actions (require_unique=true). Refusing to proceed.",
            selector.value,
            matches.len()
        );
    }

    // If require_unique=false, we still refuse for now (safety-first).
    if matches.len() != 1 {
        bail!(
            "Selector '{}' matched {} actions. Refusing to proceed (must be exactly 1).",
            selector.value,
            matches.len()
        );
    }

    Ok(matches[0])
}

fn get_action_source_code(flow: &JsonValue, action_index: usize) -> Result<String> {
    let actions = flow
        .get("actions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Workflow JSON missing 'actions' array"))?;

    let a = actions
        .get(action_index)
        .ok_or_else(|| anyhow::anyhow!("Action index {} out of bounds", action_index))?;

    let source = a
        .get("sourceCode")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Target action missing 'sourceCode'"))?;

    Ok(source.to_string())
}

/* ---------------- payload build ---------------- */

fn build_updated_flow_payload(
    flow: &JsonValue,
    action_index: usize,
    new_source: &str,
    runtime_override: Option<&str>,
) -> Result<JsonValue> {
    // Clone full flow first (we will sanitize afterward)
    let mut flow_mut = flow.clone();

    // Update the action within actions[]
    {
        let actions = flow_mut
            .get_mut("actions")
            .and_then(|v| v.as_array_mut())
            .ok_or_else(|| anyhow::anyhow!("Workflow JSON missing 'actions' array"))?;

        let a = actions
            .get_mut(action_index)
            .ok_or_else(|| anyhow::anyhow!("Action index {} out of bounds", action_index))?;

        // Replace source code
        if let Some(obj) = a.as_object_mut() {
            obj.insert(
                "sourceCode".to_string(),
                JsonValue::String(new_source.to_string()),
            );

            // Only set runtime if specified in cicd.yaml (optional in force mode)
            if let Some(rt) = runtime_override {
                obj.insert("runtime".to_string(), JsonValue::String(rt.to_string()));
            }
        } else {
            bail!("Target action is not an object");
        }
    }

    // Now build sanitized payload (revision-safe)
    let mut payload = serde_json::Map::new();

    // Required fields
    for key in [
        "revisionId",
        "type",
        "name",
        "isEnabled",
        "actions",
        "startActionId",
    ] {
        let v = flow
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("Workflow JSON missing required field '{}'", key))?;
        payload.insert(key.to_string(), v.clone());
    }

    // Optional allowlist (same idea as your python example)
    let optional_fields = [
        "enrollmentCriteria",
        "enrollmentSchedule",
        "goalFilterBranch",
        "suppressionListIds",
        "timeWindows",
        "blockedDates",
        "unEnrollmentSetting",
        "customProperties",
        "canEnrollFromSalesforce",
        "description",
    ];

    for field in optional_fields {
        if let Some(v) = flow.get(field) {
            payload.insert(field.to_string(), v.clone());
        }
    }

    // BUT: actions must come from mutated flow
    payload.insert(
        "actions".to_string(),
        flow_mut
            .get("actions")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Mutated flow missing 'actions'"))?,
    );

    Ok(JsonValue::Object(payload))
}

/* ---------------- hashing marker ---------------- */

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    hex::encode(out)
}

/// Inserts a hash marker at the top of the file.
/// Uses `#` for python, `//` for js by best-effort detection.
fn inject_hash_marker(source: &str, hash: &str) -> String {
    // crude but effective: if it looks like python file, use '#', else JS style
    let is_pythonish = source.contains("def main(") || source.contains("import ");
    let comment = if is_pythonish {
        format!("# hsemulator-sha: {}\n", hash)
    } else {
        format!("// hsemulator-sha: {}\n", hash)
    };

    // If a marker already exists at top, replace it
    if let Some(existing) = extract_hash_marker(source) {
        if existing == hash {
            // already marked with same hash
            return source.to_string();
        }
        return replace_hash_marker(source, &comment);
    }

    format!("{}{}", comment, source)
}

fn extract_hash_marker(source: &str) -> Option<String> {
    for line in source.lines().take(10) {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("# hsemulator-sha: ") {
            return Some(rest.trim().to_string());
        }
        if let Some(rest) = line.strip_prefix("// hsemulator-sha: ") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn strip_hash_marker(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let t = line.trim();
            !t.starts_with("# hsemulator-sha: ") && !t.starts_with("// hsemulator-sha: ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn replace_hash_marker(source: &str, new_marker_line: &str) -> String {
    let mut out = String::new();
    let mut replaced = false;

    for (i, line) in source.lines().enumerate() {
        if !replaced && i < 10 {
            let t = line.trim();
            if t.starts_with("# hsemulator-sha: ") || t.starts_with("// hsemulator-sha: ") {
                out.push_str(new_marker_line.trim_end());
                out.push('\n');
                replaced = true;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }

    out.trim_end().to_string()
}
