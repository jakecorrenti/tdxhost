mod ok;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: TdxCommand,
}

/// Utilities for managing the host TDX environment
#[derive(Subcommand, Debug)]
enum TdxCommand {
    /// Probe system for TDX support
    Ok,
}

fn main() {
    let args = Args::parse();

    match args.cmd {
        TdxCommand::Ok => ok::run_all_checks(),
    }
}
