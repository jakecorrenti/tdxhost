mod cli;
mod ok;

use clap::Parser;

fn main() {
    let args = cli::Cli::parse();

    match args.cmd {
        cli::TdxCommand::Ok => ok::run_all_checks(),
    }
}
