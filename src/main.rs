use anyhow::Result;
use clap::Parser;
use codex_ws::app::{RunConfig, resolve_sessions_root, run_workspace};
use codex_ws::cli::{Cli, Command, ConfigCommand, WorkspaceCommand};
use codex_ws::config::{load_default_user_config, set_default_config_value};
use codex_ws::workspace::{add_workspace, list_workspaces};

fn main() -> Result<std::process::ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Command::Run(args) => {
            let config = RunConfig::from_args(args)?;
            run_workspace(&config)
        }
        Command::Config(args) => match args.command {
            ConfigCommand::Get(args) => {
                let config = load_default_user_config()?;
                if let Some(config_name) = args.config_name {
                    if let Some(entry) = config.get_value(&config_name)? {
                        println!("{}", entry.value().display());
                    }
                } else {
                    for entry in config.entries() {
                        println!("{}\t{}", entry.name(), entry.value().display());
                    }
                }
                Ok(std::process::ExitCode::SUCCESS)
            }
            ConfigCommand::Set(args) => {
                set_default_config_value(&args.config_name, args.config_value)?;
                Ok(std::process::ExitCode::SUCCESS)
            }
        },
        Command::Workspace(args) => match args.command {
            WorkspaceCommand::Ls(args) => {
                let sessions_root = resolve_sessions_root(args.sessions_root)?;
                for workspace in list_workspaces(&sessions_root)? {
                    println!("{}\t{}", workspace.name(), workspace.path().display());
                }
                Ok(std::process::ExitCode::SUCCESS)
            }
            WorkspaceCommand::Add(args) => {
                let sessions_root = resolve_sessions_root(args.sessions_root)?;
                let path = add_workspace(&sessions_root, &args.workspace_name)?;
                println!("{}", path.display());
                Ok(std::process::ExitCode::SUCCESS)
            }
        },
    }
}
