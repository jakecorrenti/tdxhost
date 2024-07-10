// SPDX-License-Identifier: Apache-2.0

use clap::{Command, CommandFactory};
use std::{io::Error, path::Path};

#[path = "src/cli.rs"]
mod cli;

fn generate_manpages(cmd: &Command, out_dir: &Path) -> Result<(), Error> {
    let man = clap_mangen::Man::new(cmd.clone());
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    std::fs::write(out_dir.join("tdxhost.1"), buffer)?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let out_dir = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").ok_or_else(|| std::io::ErrorKind::NotFound)?,
    );
    let cmd = cli::Cli::command();

    generate_manpages(&cmd, &out_dir)?;

    Ok(())
}
