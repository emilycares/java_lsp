#![deny(clippy::redundant_clone)]
use cli::Command;
use common::TaskProgress;
use jdk::ForceLoader;
use server::command::{reload_dependencies_cli, update_dependencies_cli};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let args = cli::parse(&args);
    match args {
        Some(Command::Help) => cli::print_help(),
        Some(Command::Server) | None => {
            unsafe {
                std::env::set_var("RUST_BACKTRACE", "1");
                // std::env::set_var("RUST_LOG=lsp_server", "debug");
            };
            let _ = server::stdio();
        }
        Some(Command::ServerTcp { port }) => {
            unsafe {
                std::env::set_var("RUST_BACKTRACE", "1");
                // std::env::set_var("RUST_LOG=lsp_server", "debug");
            };
            let _ = server::listen(port);
        }
        Some(Command::ReloadDependencies) => reload_dependencies_cli().await,
        Some(Command::UpdateDependencies) => update_dependencies_cli().await,
        Some(Command::Lex { file }) => {
            cli::lex(&file);
        }
        Some(Command::LexPos { file, pos }) => {
            cli::lex_pos(&file, pos);
        }
        Some(Command::AstCheck { file }) => {
            cli::ast_check(&file);
        }
        Some(Command::AstCheckDir { folder, ignore }) => {
            if let Some(ignore) = ignore {
                let collect: Vec<String> = ignore.split(',').map(|i| i.to_string()).collect();
                cli::ast_check_dir_ignore(folder, &collect).await.unwrap();
            } else {
                cli::ast_check_dir(folder).unwrap();
            }
        }
        Some(Command::AstCheckJdk) => {
            let Some(path) = std::env::var_os("PATH") else {
                return;
            };
            let (java_path, op_dir) = jdk::get_work_dirs(&path).unwrap();
            let (sender, _) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress::default());
            jdk::load_jdk(&java_path, &op_dir, ForceLoader::None, sender)
                .await
                .unwrap();
            cli::ast_check_dir(op_dir.join("src")).unwrap();
        }
        Some(Command::IndexJdk { variant }) => {
            cli::index_jdk(variant).await;
        }
    }
}
