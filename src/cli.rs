// src/cli.rs

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Local HubSpot Custom Code runner (JavaScript / Python).
///
/// `config.yaml` is the source of truth.
/// CLI flags override configuration values explicitly.
///
/// Designed for:
/// - Local development
/// - Deterministic testing
/// - CI/CD promotion workflows
#[derive(Parser, Debug)]
#[command(
    name = "hsemulate",
    version,
    disable_help_subcommand = true,
    arg_required_else_help = true
)]
pub struct Cli {
    /// Command to execute
    #[command(subcommand)]
    pub command: Command,
}

/// All supported CLI commands.
///
/// Ordered by typical usage flow.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialise a new hsemulate project.
    ///
    /// Creates:
    /// - config.yaml
    /// - fixtures/event.json
    /// - Optional starter action file
    ///
    /// Example:
    ///   hsemulate init js
    Init {
        /// Action runtime language
        ///
        /// Allowed values: js | python
        #[arg(value_parser = ["js", "python"])]
        language: Option<String>,
    },

    /// Validate configuration and exit.
    ///
    /// This performs **static validation only**:
    /// - Schema correctness
    /// - Required fields
    /// - File existence
    /// - Internal consistency
    ///
    /// No code is executed.
    ///
    /// Intended to be run:
    /// - Before `run`
    /// - Before `test`
    /// - Automatically by UIs or CI pipelines
    ///
    /// Example:
    ///   hsemulate validate
    ///   hsemulate validate --config ./config.yaml
    Validate {
        /// Path to config file
        ///
        /// Defaults to ./config.yaml
        #[arg(short, long, default_value = "config.yaml")]
        config: PathBuf,
    },

    /// Run a HubSpot custom code action locally.
    ///
    /// By default this uses:
    /// - config.yaml
    /// - action, fixtures, assertions defined within it
    ///
    /// CLI flags override config values explicitly.
    ///
    /// Example:
    ///   hsemulate run
    ///   hsemulate run --watch
    ///   hsemulate run --repeat 25
    Run {
        /// Path to config file
        ///
        /// Defaults to ./config.yaml
        #[arg(short, long, default_value = "config.yaml")]
        config: PathBuf,

        /// Override action entry file
        ///
        /// Example:
        ///   --action actions/action.js
        #[arg(long)]
        action: Option<PathBuf>,

        /// Override fixture file (repeatable)
        ///
        /// Example:
        ///   --fixture fixtures/create.json
        ///   --fixture fixtures/update.json
        #[arg(long)]
        fixture: Vec<PathBuf>,

        /// Override assertions file (JSON)
        ///
        /// When provided, assertions in config.yaml are ignored.
        #[arg(long)]
        assert: Option<PathBuf>,

        /// Enable snapshot testing
        ///
        /// Forces snapshots.enabled = true
        #[arg(long)]
        snapshot: bool,

        /// Enable watch mode
        ///
        /// Re-runs when source files change.
        #[arg(long)]
        watch: bool,

        /// Repeat execution N times (flaky detection)
        #[arg(long)]
        repeat: Option<u32>,

        /// Override execution time budget (milliseconds)
        #[arg(long)]
        budget_time: Option<u64>,

        /// Override memory budget (MB, peak RSS)
        #[arg(long)]
        budget_mem: Option<u64>,
    },

    /// CI-first execution mode.
    ///
    /// This is equivalent to:
    /// - mode = ci
    /// - snapshots enabled
    /// - watch disabled
    /// - strict failure handling
    ///
    /// Intended for:
    /// - CI pipelines
    /// - Pre-promotion gates
    ///
    /// Example:
    ///   hsemulate test
    Test {
        /// Path to config file
        ///
        /// Defaults to ./config.yaml
        #[arg(short, long, default_value = "config.yaml")]
        config: PathBuf,
    },

    /// Start the HTTP runtime server.
    ///
    /// Exposes endpoints for:
    /// - Validation
    /// - Execution
    ///
    /// Intended for:
    /// - Remote execution
    /// - Control-plane orchestration
    /// - Managed runners
    ///
    /// Example:
    ///   hsemulate runtime --listen 0.0.0.0:8080
    Runtime {
        /// Address to listen on
        #[arg(long, default_value = "127.0.0.1:8080")]
        listen: String,
    },

    /// CI/CD related commands.
    ///
    /// Used to scaffold and manage promotion workflows.
    Cicd {
        #[command(subcommand)]
        command: CicdCommand,
    },

    /// Promote tested code into an existing HubSpot workflow action.
    ///
    /// Promotion is gated by test results unless `--force` is used.
    ///
    /// Assumes:
    /// - Workflow and action already exist
    /// - CI/CD configuration is present
    ///
    /// Example:
    ///   hsemulate promote production
    Promote {
        /// Promotion target name from .hsemulator/cicd.yaml
        ///
        /// Example: "production"
        target: String,

        /// Force promotion (skip test gates)
        #[arg(long)]
        force: bool,

        /// Path to action config file
        ///
        /// Defaults to ./config.yaml
        #[arg(short, long, default_value = "config.yaml")]
        config: PathBuf,
    },
}

/// CI/CD subcommands.
#[derive(Subcommand, Debug)]
pub enum CicdCommand {
    /// Initialise CI/CD configuration.
    ///
    /// Creates:
    /// - .hsemulator/cicd.yaml
    ///
    /// Optional:
    /// - GitHub Actions workflow
    ///
    /// Examples:
    ///   hsemulate cicd init js
    ///   hsemulate cicd init js action --branch main
    Init {
        /// Runtime language for the action
        ///
        /// Required: js | python
        #[arg(value_parser = ["js", "python"])]
        runtime: String,

        /// Optional CI/CD init type
        ///
        /// Supported:
        /// - action (GitHub Actions)
        #[arg(value_enum)]
        kind: Option<CicdInitKind>,

        /// Git branch to trigger CI/CD on
        ///
        /// Only valid when kind = action
        #[arg(long)]
        branch: Option<String>,
    },
}

/// Supported CI/CD init types.
#[derive(ValueEnum, Debug, Clone)]
pub enum CicdInitKind {
    /// Initialise GitHub Actions workflow
    Action,
}
