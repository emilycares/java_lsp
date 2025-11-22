#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
use clap::Parser;
use cli::{Args, Commands};
use common::TaskProgress;

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
        Some(Commands::Lex { file }) => {
            cli::lex(file);
        }
        Some(Commands::LexPos { file, pos }) => {
            cli::lex_pos(file, pos);
        }
        Some(Commands::AstCheck { file }) => {
            cli::ast_check(file);
        }
        Some(Commands::AstCheckDir { folder }) => {
            cli::ast_check_dir(folder);
        }
        Some(Commands::AstCheckJdk) => {
            let (java_path, op_dir) = jdk::get_work_dirs().await.unwrap();
            let (sender, _) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress::default());
            jdk::load_jdk(java_path, &op_dir, sender).await.unwrap();
            cli::ast_check_dir(op_dir);
        }
    }
}
