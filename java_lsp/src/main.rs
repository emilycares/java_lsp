use clap::Parser;
use cli::{Args, Commands};

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.cmd {
        Some(Commands::Server) | None => {
            unsafe {
                std::env::set_var("RUST_BACKTRACE", "1");
            };
            let _ = server::main();
        }
    }
}
