// src/snapshot.rs

//! Snapshot storage and comparison.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Build the full snapshot file path for a given snapshot key.
///
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

    Ok(normalize(parsed))
}

/// Write a snapshot file to disk.
pub fn write_snapshot(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create snapshot directory {:?}", parent))?;
    }

    let normalized = normalize(value.clone());

    let bytes = serde_json::to_vec_pretty(&normalized)
        .context("Failed to serialise snapshot JSON")?;

    std::fs::write(path, bytes)
        .with_context(|| format!("Failed to write snapshot file {:?}", path))?;

    Ok(())
}

/// Compare an expected snapshot with the actual output.
///
/// Fails with a readable diff when values differ.
pub fn compare_snapshot(expected: &Value, actual: &Value) -> Result<()> {
    let expected = normalize(expected.clone());
    let actual = normalize(actual.clone());

    if expected == actual {
        return Ok(());
    }

    let expected_str =
        serde_json::to_string_pretty(&expected).unwrap_or_else(|_| "<invalid json>".to_string());
    let actual_str =
        serde_json::to_string_pretty(&actual).unwrap_or_else(|_| "<invalid json>".to_string());

    bail!(
        "Snapshot mismatch\n\n--- expected\n{}\n\n+++ actual\n{}",
        expected_str,
        actual_str
    );
}

/* ---------------- helpers ---------------- */

/// Recursively normalise JSON values to ensure stable ordering.
fn normalize(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();

            let mut normalized = serde_json::Map::new();
            for k in keys {
                if let Some(v) = map.get(&k) {
                    normalized.insert(k, normalize(v.clone()));
                }
            }

            Value::Object(normalized)
        }

        Value::Array(arr) => {
            Value::Array(arr.into_iter().map(normalize).collect())
        }

        other => other,
    }
}
