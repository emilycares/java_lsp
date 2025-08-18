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
        Some(Commands::AstCheck { file }) => {
            cli::ast_check(file);
        }
        Some(Commands::AstCheckFolder { folder }) => {
            cli::ast_check_folder(folder);
        }
    }
}
