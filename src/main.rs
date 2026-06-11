use anyhow::Result;
use clap::Parser;
use codex_ws::app::{RunConfig, run_workspace};
use codex_ws::cli::{Cli, Command, WorkspaceCommand};
use codex_ws::workspace::{add_workspace, expand_home_path, list_workspaces};

fn main() -> Result<std::process::ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Command::Run(args) => {
            let config = RunConfig::from_args(args)?;
            run_workspace(&config)
        }
        Command::Workspace(args) => match args.command {
            WorkspaceCommand::Ls(args) => {
                let sessions_root = expand_home_path(args.sessions_root);
                for workspace in list_workspaces(&sessions_root)? {
                    println!("{}\t{}", workspace.name(), workspace.path().display());
                }
                Ok(std::process::ExitCode::SUCCESS)
            }
            WorkspaceCommand::Add(args) => {
                let sessions_root = expand_home_path(args.sessions_root);
                let path = add_workspace(&sessions_root, &args.workspace_name)?;
                println!("{}", path.display());
                Ok(std::process::ExitCode::SUCCESS)
            }
        },
    }
}
