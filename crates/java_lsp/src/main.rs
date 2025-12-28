#![deny(warnings)]
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
                // std::env::set_var("RUST_LOG=lsp_server", "debug");
            };
            let _ = server::stdio();
        }
        Some(Commands::Lex { file }) => {
            cli::lex(file);
        }
        Some(Commands::LexPos { file, pos }) => {
            cli::lex_pos(file, pos);
        }
        Some(Commands::AstCheck { file }) => {
            #[cfg(target_os = "windows")]
            cli::ast_check(&file);
            #[cfg(not(target_os = "windows"))]
            cli::ast_check(&file, 0, &mut Vec::new());
        }
        Some(Commands::AstCheckDir { folder, ignore }) => {
            if let Some(ignore) = ignore {
                let collect: Vec<String> = ignore.split(',').map(|i| i.to_string()).collect();
                cli::ast_check_dir_ignore(folder, collect).await.unwrap();
            } else {
                cli::ast_check_dir(folder).await.unwrap();
            }
        }
        Some(Commands::AstCheckJdk) => {
            let (java_path, op_dir) = jdk::get_work_dirs().await.unwrap();
            let (sender, _) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress::default());
            jdk::load_jdk(java_path, &op_dir, sender).await.unwrap();
            cli::ast_check_dir(op_dir.join("src")).await.unwrap();
        }
    }
}
