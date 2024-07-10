use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: TdxCommand,
}

/// Utilities for managing the host TDX environment
#[derive(Subcommand, Debug)]
pub enum TdxCommand {
    /// Probe system for TDX support
    Ok,
}
