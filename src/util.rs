// src/util.rs

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Resolve a directory path relative to the location of `config.yaml`.
///
/// This is used for:
/// - fixtures directory
/// - any other config-relative paths
///
/// Example:
/// config.yaml at `/project/config.yaml`
/// fixtures.dir = "fixtures"
/// → resolves to `/project/fixtures`
#[allow(dead_code)]
pub fn resolve_dir_relative_to_config(
    config_path: &Path,
    rel_dir: &str,
) -> Result<PathBuf> {
    let base = config_path
        .parent()
        .context("Config path has no parent directory")?;
    Ok(base.join(rel_dir))
}

/// Read a UTF-8 file into a String with a clear error message.
///
/// This is mainly used for:
/// - fixture JSON
/// - assertion JSON
pub fn read_to_string(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file {:?}", path))
}

/// Ensure a directory exists (create it if missing).
///
/// This is used when:
/// - creating fixtures
/// - creating snapshots
/// - initialising project scaffolding
pub fn ensure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory {:?}", path))
}

/// Build a stable snapshot key from the action filename and fixture name.
///
/// This ensures snapshots are:
/// - deterministic
/// - readable
/// - unique per action + fixture combination
///
/// Example:
/// action_file = actions/my_action.js
/// fixture_name = booking_created.json
///
/// → my_action.booking_created
pub fn snapshot_key(action_file: &Path, fixture_name: &str) -> String {
    let action_stem = action_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("action");

    let fixture_stem = Path::new(fixture_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("fixture");

    format!("{}.{}", action_stem, fixture_stem)
}
