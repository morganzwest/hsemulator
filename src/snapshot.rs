// src/snapshot.rs

//! Snapshot storage and comparison.
//!
//! Snapshots capture the **final structured JSON output** of an action run
//! (the JSON printed by the runner shim).
//!
//! They deliberately do NOT include:
//! - execution time
//! - memory usage
//! - run metadata
//!
//! This ensures snapshots are stable and only fail when the *behavioural output*
//! of the action changes.
//!
//! Snapshots are opt-in via the `--snapshot` flag.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Build the full snapshot file path for a given snapshot key.
///
/// Snapshot files are stored as:
/// snapshots/<key>.snapshot.json
pub fn snapshot_path(base_dir: &Path, key: &str) -> PathBuf {
    base_dir.join(format!("{}.snapshot.json", key))
}

/// Load a snapshot file from disk.
pub fn load_snapshot(path: &Path) -> Result<Value> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read snapshot file {:?}", path))?;

    let parsed: Value =
        serde_json::from_str(&raw).context("Snapshot file is not valid JSON")?;

    Ok(parsed)
}

/// Write a snapshot file to disk.
///
/// The directory is created automatically if it does not exist.
pub fn write_snapshot(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create snapshot directory {:?}", parent))?;
    }

    let bytes = serde_json::to_vec_pretty(value)
        .context("Failed to serialise snapshot JSON")?;

    std::fs::write(path, bytes)
        .with_context(|| format!("Failed to write snapshot file {:?}", path))?;

    Ok(())
}

/// Compare an expected snapshot with the actual output.
///
/// Fails if the two JSON values are not exactly equal.
pub fn compare_snapshot(expected: &Value, actual: &Value) -> Result<()> {
    if expected != actual {
        bail!("Snapshot mismatch");
    }

    Ok(())
}
