use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Launch and manage Codex CLI workspaces.
#[derive(Debug, Parser)]
#[command(name = "codex-ws", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Commands supported by the `codex-ws` CLI.
#[derive(Debug, Subcommand)]
enum Command {
    /// Launch a Codex sandbox for a workspace manifest.
    Run(RunArgs),
}

/// Arguments used to launch a workspace sandbox.
#[derive(Debug, Parser)]
struct RunArgs {
    /// Provider configuration name to load from the local configuration database.
    #[arg(short, long)]
    provider: String,

    /// Path to the workspace manifest YAML file.
    #[arg(short, long, value_name = "PATH")]
    workspace: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Run(args) => {
            println!(
                "launching workspace '{}' with provider '{}'",
                args.workspace.display(),
                args.provider
            );
        }
    }
}
