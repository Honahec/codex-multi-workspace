use anyhow::Result;
use clap::Parser;
use codex_ws::cli::{Cli, Command};

fn main() -> Result<()> {
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

    Ok(())
}
