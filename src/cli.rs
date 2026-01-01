// src/cli.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Local HubSpot Custom Code runner (JavaScript / Python).
///
/// This CLI is intentionally simple:
/// - `init` scaffolds a runnable project
/// - `run` executes a HubSpot custom code action locally
///
/// You do NOT need to know Rust to use this tool.
/// You only interact with it via the CLI.
#[derive(Parser, Debug)]
#[command(
    name = "hsemulate",
    version,
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Subcommand to execute (`run` or `init`)
    #[command(subcommand)]
    pub command: Command,
}

/// All supported CLI commands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run a HubSpot custom code action locally.
    ///
    /// This runs the SAME file you paste into HubSpot, using:
    /// - a fixture JSON as the `event` payload
    /// - Node or Python locally
    /// - optional assertions, snapshots, budgets, and flaky detection
    Run {
        /// Path to the action file (.js/.mjs/.cjs or .py)
        ///
        /// Example:
        /// hsemulate run actions/action.js -c config.yaml
        file: PathBuf,

        /// Path to config.yaml
        ///
        /// This controls fixtures, env vars, runtimes, and budgets.
        #[arg(short, long)]
        config: PathBuf,

        /// Override fixture filename.
        ///
        /// This is resolved relative to `fixtures.dir` in config.yaml.
        /// If omitted, `fixtures.default` is used.
        #[arg(long)]
        fixture: Option<String>,

        /// Assertion file (JSON).
        ///
        /// Format:
        /// {
        ///   "callback.outputFields.success": true
        /// }
        #[arg(long)]
        assert: Option<PathBuf>,

        /// Enable snapshot testing.
        ///
        /// - First run creates a snapshot.
        /// - Subsequent runs must match exactly.
        #[arg(long)]
        snapshot: bool,

        /// Repeat the same run N times to detect flaky behaviour.
        ///
        /// Example:
        /// --repeat 10
        #[arg(long, default_value_t = 1)]
        repeat: u32,

        /// Override duration budget in milliseconds.
        ///
        /// If set, the action must complete within this time.
        #[arg(long)]
        budget_time: Option<u64>,

        /// Override memory budget in MB (peak RSS).
        ///
        /// If set, the action must not exceed this memory usage.
        #[arg(long)]
        budget_mem: Option<u64>,
    },

    /// Initialise a project scaffold.
    ///
    /// Creates:
    /// - config.yaml
    /// - fixtures/event.json
    ///
    /// Optionally also creates a starter action file:
    /// - hsemulate init js
    /// - hsemulate init python
    Init {
        /// Optional action template language: "js" or "python"
        #[arg(value_parser = ["js", "python"])]
        language: Option<String>,
    },
}
