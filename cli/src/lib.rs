use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// A java lsp server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub cmd: Option<Commands>,

    /// Unused flag required by vscode.
    #[arg(short, long, default_value_t = true)]
    pub stdio: bool,
}
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the lsp server over stdio
    Server,
    /// Format a .java file.
    /// The default is over stdio
    Format {
        /// Path to read the .java file
        #[clap(short, long)]
        path: Option<PathBuf>,
    },
}
