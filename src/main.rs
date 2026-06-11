use anyhow::Result;
use clap::Parser;
use codex_ws::app::{RunConfig, run_workspace};
use codex_ws::cli::{Cli, Command};

fn main() -> Result<std::process::ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Command::Run(args) => {
            let config = RunConfig::from_args(args);
            run_workspace(&config)
        }
    }
}
