use std::path::PathBuf;

use clap::{command, Parser, Subcommand};

#[derive(Parser)]
#[command(about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Init
    Init {
        /// Root path
        root_path: Option<PathBuf>,
    },

    /// Add
    Add {
        /// Paths to add
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },

    /// Commit
    Commit {},
}
