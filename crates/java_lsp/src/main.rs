#![deny(clippy::redundant_clone)]
use cli::Command;
use command::{reload_dependencies_cli, update_dependencies_cli};
use tokio::runtime::LocalOptions;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let args = cli::parse(&args);
    match args {
        Some(Command::Help) => cli::print_help(),
        Some(Command::Server) | None => {
            unsafe {
                std::env::set_var("RUST_BACKTRACE", "1");
                // std::env::set_var("RUST_LOG=lsp_server", "debug");
            };
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let _ = server::stdio();
                })
        }
        Some(Command::ServerTcp { port }) => {
            unsafe {
                std::env::set_var("RUST_BACKTRACE", "1");
                // std::env::set_var("RUST_LOG=lsp_server", "debug");
            };
            let _ = server::listen(port);
        }
        Some(Command::ReloadDependencies) => tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { reload_dependencies_cli().await }),
        Some(Command::UpdateDependencies) => tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { update_dependencies_cli().await }),
        Some(Command::Lex { file }) => {
            cli::lex(&file);
        }
        Some(Command::LexPos { file, pos }) => {
            cli::lex_pos(&file, pos);
        }
        Some(Command::AstCheck { file }) => {
            let mut tokens = Vec::new();
            cli::ast_check(&file, &mut tokens);
        }
        Some(Command::AstCheckDir { folder, ignore }) => {
            if let Some(ignore) = ignore {
                let collect: Vec<String> = ignore.split(',').map(|i| i.to_string()).collect();
                cli::ast_check_dir_ignore(folder, &collect).unwrap();
            } else {
                cli::ast_check_dir(folder).unwrap();
            }
        }
        Some(Command::AstCheckJdk) => tokio::runtime::Builder::new_current_thread()
            .build_local(LocalOptions::default())
            .unwrap()
            .block_on(async {
                let Some(path) = std::env::var_os("PATH") else {
                    return;
                };
                let (java_path, op_dir) = jdk::get_work_dirs(&path).unwrap();
                jdk::extract_source_zip(&java_path, &op_dir).await.unwrap();
                cli::ast_check_dir(op_dir.join("src")).unwrap();
            }),
        Some(Command::IndexJdk { variant }) => tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                cli::index_jdk(variant).await;
            }),
        Some(Command::FormatFile(p)) => {
            let mut tokens = Vec::new();
            cli::format_file(
                &p,
                &mut tokens,
                true,
                &editorconfig::load_editor_config_or_default().to_filled(),
            )
        }
        Some(Command::FormatDir(p)) => cli::format_dir(&p),
    }
}
