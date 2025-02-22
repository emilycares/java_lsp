use std::{
    fs::{self, write},
    io,
    path::PathBuf,
    process::{self},
};

use clap::{Parser, Subcommand};

/// A java lsp server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    cmd: Option<Commands>,
}
#[derive(Subcommand, Debug)]
enum Commands {
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.cmd {
        Some(Commands::Format { path: None }) => match io::read_to_string(io::stdin()) {
            Ok(input) => match format::format(input, format::Formatter::Topiary) {
                Some(output) => print!("{}", output),
                None => process::exit(1),
            },
            Err(e) => {
                eprintln!("There was an error with reading stdin: {}", e);
                process::exit(1);
            }
        },
        Some(Commands::Format { path: Some(path) }) => match fs::read_to_string(&path) {
            Ok(input) => match format::format(input, format::Formatter::Topiary) {
                Some(output) => match write(&path, output) {
                    Ok(_) => println!("Replaced file"),
                    Err(_) => (),
                },
                None => process::exit(1),
            },
            Err(e) => {
                eprintln!("There was an error with reading the file: {}", e);
                process::exit(1);
            }
        },
        Some(Commands::Server) | None => {
            std::env::set_var("RUST_BACKTRACE", "1");
            let _ = server::main().await;
        }
    }
}
