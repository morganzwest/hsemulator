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

mod auth;
mod checks;
mod cicd;
mod cli;
mod config;
mod engine;
mod execution_id;
mod metrics;
mod promote;
mod runner;
mod runtime;
mod shim;
mod snapshot;
mod util;
mod sinks; 

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Program entry point.
///
/// Uses Tokio because the runner spawns and waits on child processes
/// asynchronously (Node / Python runtimes).
#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hsemulate=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse CLI arguments (run / init / flags)
    let cli = cli::Cli::parse();

    // Delegate execution to the runner
    runner::run(cli).await
}
