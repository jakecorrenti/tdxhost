// SPDX-License-Identifier: Apache-2.0

use clap::{Command, CommandFactory};
use std::{io::Error, path::Path};

use clap_complete::{generate, shells};

#[path = "src/cli.rs"]
mod cli;

fn generate_shell_completion(cmd: &mut Command, out_dir: &Path) -> Result<(), Error> {
    let mut buffer: Vec<u8> = Default::default();
    generate(shells::Zsh, cmd, "tdxhost", &mut buffer);
    std::fs::write(out_dir.join("tdxhost-completion-zsh.1"), &buffer)?;
    buffer.clear();

    generate(shells::Bash, cmd, "tdxhost", &mut buffer);
    std::fs::write(out_dir.join("tdxhost-completion-bash.1"), &buffer)?;
    buffer.clear();

    generate(shells::Fish, cmd, "tdxhost", &mut buffer);
    std::fs::write(out_dir.join("tdxhost-completion-fish.1"), &buffer)?;
    buffer.clear();

    generate(shells::PowerShell, cmd, "tdxhost", &mut buffer);
    std::fs::write(out_dir.join("tdxhost-completion-powershell.1"), &buffer)?;
    buffer.clear();

    generate(shells::Elvish, cmd, "tdxhost", &mut buffer);
    std::fs::write(out_dir.join("tdxhost-completion-elvish.1"), buffer)?;
    Ok(())
}

fn generate_manpages(cmd: &Command, out_dir: &Path) -> Result<(), Error> {
    let man = clap_mangen::Man::new(cmd.clone());
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    std::fs::write(out_dir.join("tdxhost.1"), buffer)?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let out_dir =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").ok_or(std::io::ErrorKind::NotFound)?);
    let mut cmd = cli::Cli::command();

    generate_manpages(&cmd, &out_dir)?;
    generate_shell_completion(&mut cmd, &out_dir)?;

    Ok(())
}
