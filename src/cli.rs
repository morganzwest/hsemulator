// src/cli.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Local HubSpot Custom Code runner (JavaScript / Python).
///
/// `config.yaml` is the primary source of truth.
/// CLI flags only override config values.
#[derive(Parser, Debug)]
#[command(
    name = "hsemulate",
    version,
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Command,
}

/// All supported CLI commands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run a HubSpot custom code action locally.
    ///
    /// If no arguments are provided, this defaults to:
    /// - config.yaml
    /// - action, fixtures, assertions defined inside it
    Run {
        /// Path to config file
        ///
        /// Defaults to ./config.yaml
        #[arg(short, long, default_value = "config.yaml")]
        config: PathBuf,

        /// Override action entry file
        ///
        /// Example:
        /// --action actions/action.js
        #[arg(long)]
        action: Option<PathBuf>,

        /// Override fixture (can be passed multiple times)
        ///
        /// Example:
        /// --fixture fixtures/create.json
        #[arg(long)]
        fixture: Vec<PathBuf>,

        /// Override assertions file (JSON)
        ///
        /// If provided, config.yaml assertions are ignored.
        #[arg(long)]
        assert: Option<PathBuf>,

        /// Enable snapshot testing
        ///
        /// Overrides config snapshots.enabled = true
        #[arg(long)]
        snapshot: bool,

        /// Enable watch mode
        ///
        /// Re-runs action when files change.
        #[arg(long)]
        watch: bool,

        /// Repeat execution N times (flaky detection)
        #[arg(long)]
        repeat: Option<u32>,

        /// Override duration budget in milliseconds
        #[arg(long)]
        budget_time: Option<u64>,

        /// Override memory budget in MB (peak RSS)
        #[arg(long)]
        budget_mem: Option<u64>,
    },

    /// CI-first execution mode.
    ///
    /// Equivalent to:
    /// - mode = ci
    /// - snapshots enabled
    /// - no watch
    /// - strict failure handling
    Test {
        /// Path to config file
        ///
        /// Defaults to ./config.yaml
        #[arg(short, long, default_value = "config.yaml")]
        config: PathBuf,
    },

    /// Initialise a project scaffold.
    ///
    /// Creates:
    /// - config.yaml
    /// - fixtures/event.json
    /// - optional starter action file
    Init {
        /// Optional action template language
        ///
        /// Allowed values: js | python
        #[arg(value_parser = ["js", "python"])]
        language: Option<String>,
    },
}
