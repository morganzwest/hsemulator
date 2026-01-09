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
    pub action: Action,

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

        let cfg: Config =
            serde_yaml::from_str(&raw).context("Failed to parse YAML config")?;

        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        if self.fixtures.is_empty() {
            anyhow::bail!("At least one fixture must be defined in `fixtures`");
        }

        if self.repeat == 0 {
            anyhow::bail!("repeat must be >= 1");
        }

        Ok(())
    }
}
