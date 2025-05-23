use std::process::{self};

use clap::Parser;
use cli::{Args, Commands};

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.cmd {
        Some(Commands::Format { path: None }) => todo!(),
        Some(Commands::Format { path: Some(path) }) => {
            match format::format_op(format::Formatter::Topiary { path }) {
                Ok(()) => {
                    println!("Replaced file");
                }
                Err(e) => {
                    eprintln!("There was an error with formatting: {e:?}");
                    process::exit(1)
                }
            }
        }
        Some(Commands::Server) | None => {
            unsafe {
                std::env::set_var("RUST_BACKTRACE", "1");
            };
            let _ = server::main();
        }
    }
}
