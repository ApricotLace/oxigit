use std::env;

use anyhow::Context;
use clap::Parser;
use cmd::Commands;
use repository::Repository;

mod cmd;
pub mod lockfile;
pub mod oid;
mod repository;

fn main() -> Result<(), anyhow::Error> {
    let cli = cmd::Cli::parse();

    match &cli.command {
        Commands::Commit {} => {
            Repository::open(
                env::current_dir().with_context(|| "Can't get current working directory")?,
            )
            .commit()?;
        }
        Commands::Add { paths } => {
            Repository::open(
                env::current_dir().with_context(|| "Can't get current working directory")?,
            )
            .add(paths)?;
        }
        Commands::Init { root_path } => {
            let root = match root_path {
                Some(root) => root.to_path_buf(),
                None => {
                    env::current_dir().with_context(|| "Can't get current working directory")?
                }
            };
            Repository::open(root).init()?;
        }
    }

    Ok(())
}
