// src/main.rs

//! hsemulate
//!
//! Entry point for the hsemulate CLI.
//!
//! This binary provides a local runner for HubSpot Workflow Custom Code Actions
//! (JavaScript and Python). It delegates all real work to the `runner` module.
//!
//! Responsibilities of this file:
//! - Parse CLI arguments
//! - Initialise the async runtime
//! - Hand off execution to the runner
//!
//! There is intentionally *no business logic* here.

mod checks;
mod cli;
mod cicd;
mod config;
mod metrics;
mod runner;
mod shim;
mod snapshot;
mod util;
mod promote;

use anyhow::Result;
use clap::Parser;

/// Program entry point.
///
/// Uses Tokio because the runner spawns and waits on child processes
/// asynchronously (Node / Python runtimes).
#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments (run / init / flags)
    let cli = cli::Cli::parse();

    // Delegate execution to the runner
    runner::run(cli).await
}
