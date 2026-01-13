// src/cicd.rs

use crate::cli::{CicdCommand, CicdInitKind};
use crate::util::ensure_dir;

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Entry point for CI/CD commands.
pub fn handle(command: CicdCommand) -> Result<()> {
    match command {
        CicdCommand::Init {
            kind,
            runtime,
            branch,
        } => init(kind, runtime, branch),
    }
}

/* ---------------- cicd init ---------------- */

fn init(kind: Option<CicdInitKind>, runtime: String, branch: Option<String>) -> Result<()> {
    // Validate branch usage
    if branch.is_some() && !matches!(kind, Some(CicdInitKind::Action)) {
        bail!("--branch can only be used with `cicd init action`");
    }

    let runtime = match runtime.as_str() {
        "js" => "NODE20X",
        "python" => "PYTHON39",
        _ => bail!("Unsupported runtime: {}", runtime),
    };

    // Always create cicd.yaml
    create_cicd_yaml(runtime)?;

    // Optionally create GitHub Actions workflow
    if let Some(CicdInitKind::Action) = kind {
        let branch = branch.unwrap_or_else(|| "main".to_string());
        create_github_action(&branch)?;
    }

    Ok(())
}

/* ---------------- file creators ---------------- */

fn create_cicd_yaml(runtime: &str) -> Result<()> {
    let base = Path::new(".hsemulator");
    ensure_dir(base)?;

    let path = base.join("cicd.yaml");
    if path.exists() {
        bail!("{:?} already exists (refusing to overwrite)", path);
    }

    fs::write(&path, default_cicd_yaml(runtime))
        .with_context(|| format!("Failed to write {:?}", path))?;

    eprintln!("Created {:?}", path);
    Ok(())
}

fn create_github_action(branch: &str) -> Result<()> {
    let workflow_dir = PathBuf::from(".github/workflows");
    ensure_dir(&workflow_dir)?;

    let path = workflow_dir.join("hsemulator.yml");
    if path.exists() {
        bail!("{:?} already exists (refusing to overwrite)", path);
    }

    let content = default_github_action(branch);
    fs::write(&path, content).with_context(|| format!("Failed to write {:?}", path))?;

    eprintln!("Created {:?}", path);
    Ok(())
}

/* ---------------- templates ---------------- */

fn default_cicd_yaml(runtime: &str) -> String {
    format!(
        r#"
version: 1

hubspot:
  # Leave blank for local testing only.
  token: 'REPLACE_ME'

targets:
  production:
    portal: eu1
    workflow_id: "REPLACE_ME"

    selector:
      type: secret
      value: HS_ACTION__REPLACE_ME
      require_unique: true

    runtime: {runtime}

    safety:
      require_clean_tests: true
      require_snapshot_match: true
      max_duration_ms: 4000

    deploy:
      mode: full-flow-replace
      dry_run: false
"#,
        runtime = runtime
    )
}

fn default_github_action(branch: &str) -> String {
    format!(
        r#"
name: hsemulator

on:
  push:
    branches: [{branch}]

jobs:
  test-and-promote:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install hsemulator
        run: |
          curl -L https://github.com/morganzwest/hsemulator/releases/latest/download/hsemulator-linux \
          -o hsemulator
          chmod +x hsemulator

      - name: Run tests
        run: ./hsemulator test

      - name: Promote
        if: success()
        run: ./hsemulator promote production
        env:
          HUBSPOT_TOKEN: ${{{{ secrets.HUBSPOT_TOKEN }}}}
"#
    )
}
