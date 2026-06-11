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

    /// Read or update codex-ws user configuration.
    Config(ConfigArgs),

    /// Manage saved workspace manifests.
    Workspace(WorkspaceArgs),
}

/// Arguments used to launch a workspace sandbox.
#[derive(Debug, Parser)]
pub struct RunArgs {
    /// Provider configuration name to load from the local configuration database.
    #[arg(short, long)]
    pub provider: String,

    /// Workspace name or path to a workspace manifest YAML file.
    #[arg(short, long, value_name = "WORKSPACE")]
    pub workspace: PathBuf,

    /// Path to the local provider configuration database.
    #[arg(long, value_name = "PATH")]
    pub config_db: Option<PathBuf>,

    /// Host directory used to store per-workspace Codex sessions.
    #[arg(long, value_name = "PATH")]
    pub sessions_root: Option<PathBuf>,

    /// Docker image containing the Codex CLI.
    #[arg(long, value_name = "IMAGE")]
    pub image: Option<String>,
}

/// Arguments used to read or update user configuration.
#[derive(Debug, Parser)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

/// User configuration commands.
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Read one config value, or all configured values when no name is provided.
    Get(ConfigGetArgs),

    /// Set a supported config value.
    Set(ConfigSetArgs),
}

/// Arguments used to read user configuration.
#[derive(Debug, Parser)]
pub struct ConfigGetArgs {
    /// Optional config name to read.
    pub config_name: Option<String>,
}

/// Arguments used to update user configuration.
#[derive(Debug, Parser)]
pub struct ConfigSetArgs {
    /// Supported config name.
    pub config_name: String,

    /// Config value to persist.
    pub config_value: PathBuf,
}

/// Arguments used to manage saved workspace manifests.
#[derive(Debug, Parser)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    pub command: WorkspaceCommand,
}

/// Workspace manifest management commands.
#[derive(Debug, Subcommand)]
pub enum WorkspaceCommand {
    /// List saved workspace manifests.
    Ls(WorkspaceLsArgs),

    /// Create or edit a saved workspace manifest.
    Add(WorkspaceAddArgs),
}

/// Arguments used to list saved workspace manifests.
#[derive(Debug, Parser)]
pub struct WorkspaceLsArgs {
    /// Host directory used to store codex-ws state and saved workspace manifests.
    #[arg(long, value_name = "PATH")]
    pub sessions_root: Option<PathBuf>,
}

/// Arguments used to add a saved workspace manifest.
#[derive(Debug, Parser)]
pub struct WorkspaceAddArgs {
    /// Workspace name used for the saved manifest file.
    pub workspace_name: String,

    /// Host directory used to store codex-ws state and saved workspace manifests.
    #[arg(long, value_name = "PATH")]
    pub sessions_root: Option<PathBuf>,
}
