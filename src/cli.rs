use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Launch and manage Codex CLI workspaces.
#[derive(Debug, Parser)]
#[command(name = "codex-ws", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Commands supported by the `codex-ws` CLI.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Launch a Codex sandbox for a workspace manifest.
    Run(RunArgs),
}

/// Arguments used to launch a workspace sandbox.
#[derive(Debug, Parser)]
pub struct RunArgs {
    /// Provider configuration name to load from the local configuration database.
    #[arg(short, long)]
    pub provider: String,

    /// Path to the workspace manifest YAML file.
    #[arg(short, long, value_name = "PATH")]
    pub workspace: PathBuf,
}
