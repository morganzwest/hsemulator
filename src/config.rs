// src/config.rs

use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

/// Root configuration loaded from `config.yaml`.
///
/// This file controls:
/// - Where fixtures live
/// - Which environment variables are injected into the action
/// - Which Node/Python binaries to use
/// - Optional performance budgets
///
/// Colleagues using the tool only need to edit `config.yaml`,
/// not this Rust file.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Fixture configuration (directory + default file)
    pub fixtures: Fixtures,

    /// Environment variables injected into the action process
    ///
    /// Example:
    /// HUBSPOT_TOKEN, HUBSPOT_BASE_URL
    #[serde(default)]
    pub env: BTreeMap<String, String>,

    /// Runtime binaries (node / python)
    #[serde(default)]
    pub runtime: Runtime,

    /// Optional performance budgets
    ///
    /// These can be overridden by CLI flags.
    #[serde(default)]
    pub budgets: Option<Budgets>,

    /// Output configuration
    #[serde(default)]
    pub output: OutputConfig,
}

/// Fixture configuration section.
///
/// Example in config.yaml:
///
/// fixtures:
///   dir: fixtures
///   default: event.json
#[derive(Debug, Deserialize)]
pub struct Fixtures {
    /// Directory containing fixture JSON files.
    /// This path is resolved relative to the location of config.yaml.
    pub dir: String,

    /// Default fixture filename inside `fixtures.dir`.
    pub default: String,
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
    OutputMode::Stdout
}

/// Runtime binary configuration.
///
/// Example:
///
/// runtime:
///   node: node
///   python: python3
#[derive(Debug, Deserialize, Default)]
pub struct Runtime {
    #[serde(default = "default_node")]
    pub node: String,

    #[serde(default = "default_python")]
    pub python: String,
}

fn default_node() -> String {
    "node".to_string()
}

fn default_python() -> String {
    "python3".to_string()
}

/// Optional performance budgets.
///
/// Example:
///
/// budgets:
///   duration_ms: 500
///   memory_mb: 64
#[derive(Debug, Deserialize, Clone)]
pub struct Budgets {
    /// Maximum execution time in milliseconds
    pub duration_ms: Option<u64>,

    /// Maximum peak memory usage in megabytes (RSS)
    pub memory_mb: Option<u64>,
}

impl Config {
    /// Load and parse `config.yaml` from disk.
    ///
    /// This performs:
    /// - File read
    /// - YAML deserialization
    /// - Basic structural validation
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        let cfg: Config =
            serde_yaml::from_str(&raw).context("Failed to parse YAML config")?;

        Ok(cfg)
    }
}
