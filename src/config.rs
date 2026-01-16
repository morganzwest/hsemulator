// src/config.rs

use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

/// Root configuration loaded from `config.yaml`.
///
/// This file is the single source of truth for execution.
/// CLI flags may override fields at runtime.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Action configuration (required)
    #[serde(default)]
    pub action: Option<Action>,


    /// One or more fixture files
    #[serde(default)]
    pub fixtures: Vec<String>,

    /// Environment variables injected into the action process
    #[serde(default)]
    pub env: BTreeMap<String, String>,

    /// Runtime binaries (node / python)
    #[serde(default)]
    pub runtime: Runtime,

    /// Optional performance budgets
    #[serde(default)]
    pub budgets: Option<Budgets>,

    /// Assertions applied to the action output
    #[serde(default)]
    pub assertions: BTreeMap<String, Assertion>,

    /// Optional assertions JSON file path (overrides inline assertions)
    #[serde(default)]
    pub assertions_file: Option<String>,

    /// Snapshot configuration
    #[serde(default)]
    pub snapshots: SnapshotConfig,

    /// Output configuration
    #[serde(default)]
    pub output: OutputConfig,

    /// Watch files and re-run on change
    #[serde(default)]
    pub watch: bool,

    /// Number of times to repeat execution (flaky detection)
    #[serde(default = "default_repeat")]
    pub repeat: u32,

    /// Execution mode (normal | ci)
    #[serde(default)]
    pub mode: Mode,
}

/// Action definition.
#[derive(Debug, Deserialize)]
pub struct Action {
    /// js | python
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub action_type: ActionType,

    /// Path to the action file
    pub entry: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    Js,
    Python,
}

/// Snapshot configuration.
#[derive(Debug, Deserialize, Default)]
pub struct SnapshotConfig {
    #[serde(default)]
    pub enabled: bool,

    /// Paths to ignore when comparing snapshots
    #[serde(default)]
    #[allow(dead_code)]
    pub ignore: Vec<String>,
}

/// Assertion operators.
///
/// Values are parsed from YAML but represented as JSON for runtime comparison.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Assertion {
    Eq { eq: serde_json::Value },
    Gt { gt: serde_json::Value },
    Lt { lt: serde_json::Value },
    Exists { exists: bool },
    Regex { regex: String },
}

/// Output configuration.
#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_output_mode")]
    pub mode: OutputMode,

    /// Only used when mode = file
    #[serde(default)]
    pub file: Option<String>,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            mode: default_output_mode(),
            file: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    Stdout,
    Pretty,
    Simple,
    File,
}

fn default_output_mode() -> OutputMode {
    OutputMode::Simple
}

/// Runtime binary configuration.
#[derive(Debug, Deserialize)]
pub struct Runtime {
    #[serde(default = "default_node")]
    pub node: String,

    #[serde(default = "default_python")]
    pub python: String,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            node: default_node(),
            python: default_python(),
        }
    }
}

fn default_node() -> String {
    "node".to_string()
}

fn default_python() -> String {
    "python3".to_string()
}

/// Optional performance budgets.
#[derive(Debug, Deserialize, Clone)]
pub struct Budgets {
    pub duration_ms: Option<u64>,
    pub memory_mb: Option<u64>,
}

/// Execution mode.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Normal,
    Ci,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal
    }
}

fn default_repeat() -> u32 {
    1
}

impl Config {
    /// Load and parse `config.yaml` from disk.
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        let cfg: Config = serde_yaml::from_str(&raw).context("Failed to parse YAML config")?;

        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        let action = self.action.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Missing `action` configuration in config.yaml.\n\
                Uncomment and choose ONE:\n\
                \n\
                # JavaScript\n\
                action:\n\
                type: js\n\
                entry: actions/action.js\n\
                \n\
                # Python\n\
                action:\n\
                type: python\n\
                entry: actions/action.py"
            )
        })?;

        // ---------- action ----------
        let entry = action.entry.trim();
        if entry.is_empty() {
            anyhow::bail!(
                "Missing action.entry in config.yaml.\n\
                Set one of:\n\
                - JS:     action: {{ type: js,     entry: actions/action.js }}\n\
                - Python: action: {{ type: python, entry: actions/action.py }}"
            );
        }

        let ext = std::path::Path::new(entry)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let ext_ok = match action.action_type {
            ActionType::Js => matches!(ext.as_str(), "js" | "mjs" | "cjs"),
            ActionType::Python => ext == "py",
        };

        if !ext_ok {
            anyhow::bail!(
                "action.type does not match action.entry.\n\
                - action.type: {:?}\n\
                - action.entry: {}\n\
                Expected extensions:\n\
                - js: js | mjs | cjs\n\
                - python: py",
                action.action_type,
                entry
            );
        }

        // Optional but helpful: file existence check (clearer than later runtime errors)
        let entry_path = std::path::Path::new(entry);
        if !entry_path.exists() {
            anyhow::bail!(
                "action.entry file not found: {}\n\
                Ensure the path is correct relative to your working directory.",
                entry
            );
        }

        // ---------- fixtures ----------
        if self.fixtures.is_empty() {
            anyhow::bail!(
                "No fixtures configured.\n\
                Add at least one JSON fixture file, for example:\n\
                fixtures:\n\
                - fixtures/event.json"
            );
        }

        // Validate each fixture exists + is valid JSON
        for f in &self.fixtures {
            let f_trim = f.trim();
            if f_trim.is_empty() {
                anyhow::bail!("fixtures contains an empty path. Remove it or set a valid file path.");
            }

            let p = std::path::Path::new(f_trim);
            if !p.exists() {
                anyhow::bail!("Fixture file not found: {}", f_trim);
            }

            let raw = std::fs::read_to_string(p)
                .with_context(|| format!("Failed to read fixture file: {}", f_trim))?;

            serde_json::from_str::<serde_json::Value>(&raw)
                .with_context(|| format!("Fixture is not valid JSON: {}", f_trim))?;
        }

        // ---------- repeat ----------
        if self.repeat == 0 {
            anyhow::bail!("repeat must be >= 1");
        }

        // ---------- runtime ----------
        // Ensure runtime strings are not blank (actual resolution is handled at spawn time)
        if self.runtime.node.trim().is_empty() {
            anyhow::bail!("runtime.node must be set (e.g. 'node' or a full path to node).");
        }
        if self.runtime.python.trim().is_empty() {
            anyhow::bail!("runtime.python must be set (e.g. 'python' or a full path to python).");
        }

        // ---------- output ----------
        if matches!(self.output.mode, OutputMode::File) {
            let file = self
                .output
                .file
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());

            if file.is_none() {
                anyhow::bail!(
                    "output.mode is 'file' but output.file is not set.\n\
                    Example:\n\
                    output:\n\
                    mode: file\n\
                    file: results.json"
                );
            }
        }

        // ---------- budgets ----------
        if let Some(b) = &self.budgets {
            if let Some(ms) = b.duration_ms {
                if ms == 0 {
                    anyhow::bail!("budgets.duration_ms must be > 0 when set");
                }
            }
            if let Some(mb) = b.memory_mb {
                if mb == 0 {
                    anyhow::bail!("budgets.memory_mb must be > 0 when set");
                }
            }
        }

        // ---------- assertions ----------
        // Basic assertion key sanity so typos fail early
        for (k, v) in &self.assertions {
            let key = k.trim();
            if key.is_empty() {
                anyhow::bail!("assertions contains an empty key (remove it).");
            }
            // For regex assertions, fail fast if the pattern is invalid
            if let Assertion::Regex { regex } = v {
                let pat = regex.trim();
                if pat.is_empty() {
                    anyhow::bail!("Assertion '{}' has an empty regex pattern.", key);
                }
                regex::Regex::new(pat).with_context(|| {
                    format!("Assertion '{}' has an invalid regex pattern: {}", key, pat)
                })?;
            }
        }

        // Optional: prevent ambiguous dual assertion sources
        if self.assertions_file.is_some() && !self.assertions.is_empty() {
            anyhow::bail!(
                "Both assertions_file and inline assertions are set.\n\
                Choose one:\n\
                - use assertions_file (recommended for larger suites), OR\n\
                - keep inline assertions and remove assertions_file."
            );
        }

        Ok(())
    }

}
