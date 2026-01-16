// src/cli.rs

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Local HubSpot Custom Code runner (JavaScript / Python).
///
/// `config.yaml` is the primary source of truth.
/// CLI flags only override config values.
#[derive(Parser, Debug)]
#[command(name = "hsemulate", version, disable_help_subcommand = true)]
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

    /// CI/CD related commands.
    Cicd {
        #[command(subcommand)]
        command: CicdCommand,
    },

    /// Config-related commands
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },

    /// Promote the currently tested code into a HubSpot workflow action.
    ///
    /// This is a gated promotion step. Use --force to bypass test gates.
    Promote {
        /// Promotion target name from .hsemulator/cicd.yaml (e.g. "production")
        target: String,

        /// Force promotion (skips test gating)
        #[arg(long)]
        force: bool,

        /// Path to action config file (defaults to ./config.yaml)
        #[arg(short, long, default_value = "config.yaml")]
        config: PathBuf,
    },
}

/// CI/CD subcommands.
#[derive(Subcommand, Debug)]
pub enum CicdCommand {
    /// Initialise CI/CD configuration.
    ///
    /// - `cicd init` → creates .hsemulator/cicd.yaml
    /// - `cicd init action` → also creates GitHub Actions workflow
    Init {
        /// Runtime language for the action
        ///
        /// Required: js | python
        #[arg(value_parser = ["js", "python"])]
        runtime: String,

        /// Optional init type (e.g. GitHub Actions)
        ///
        /// Currently supported:
        /// - action
        #[arg(value_enum)]
        kind: Option<CicdInitKind>,

        /// Git branch to trigger CI/CD on
        ///
        /// Only valid when `kind = action`
        #[arg(long)]
        branch: Option<String>,
    },
}

/// Supported CI/CD init types.
#[derive(ValueEnum, Debug, Clone)]
pub enum CicdInitKind {
    /// Initialise GitHub Actions workflow.
    Action,
}

#[derive(clap::Subcommand, Debug)]
pub enum ConfigCommand {
    /// Validate config.yaml and exit
    Validate {
        #[arg(default_value = "config.yaml")]
        config: PathBuf,
    },
}
