mod cli;
mod ok;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();

    let res = match args.cmd {
        cli::TdxCommand::Ok => ok::run_all_checks(),
    };

    if let Err(ref e) = res {
        eprintln!("Error: {}", e);
    }

    res
}
