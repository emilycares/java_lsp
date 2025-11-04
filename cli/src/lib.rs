use std::{fs::canonicalize, path::PathBuf, time::Instant};

use ast::error::PrintErr;
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
    // #[arg(long, default_value = "", required_if_eq("cmd", "ast-check"))]
    // pub file: PathBuf,
}
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the lsp server over stdio
    Server,
    /// Check for errors in file
    AstCheck { file: PathBuf },
    /// Recusivly check a directory for ast for java files
    AstCheckDir { folder: PathBuf },
    /// Check jdk in path
    AstCheckJdk,
}

pub fn ast_check(file: PathBuf) {
    match std::fs::read_to_string(&file) {
        Ok(text) => {
            let before_lex = Instant::now();
            match ast::lexer::lex(&text) {
                Ok(tokens) => {
                    let lex_time = before_lex.elapsed();
                    eprintln!("Here: {:?}", file);

                    print!("Timings: [lexer: {:.2?}]", lex_time);
                    let before_ast = Instant::now();
                    let ast = ast::parse_file(&tokens);
                    let ast_time = before_ast.elapsed();
                    println!("[ast: {:.2?}]", ast_time);
                    if ast.is_err() {
                        eprintln!("Here: {:?}", file);
                        ast.print_err(&text);
                        std::process::exit(3);
                    }
                }
                Err(e) => {
                    eprintln!("Here: {:?}", file);
                    eprintln!("Lexer error: {:?}", e);
                    std::process::exit(2);
                }
            }
        }
        Err(e) => {
            eprintln!("Here: {:?}", file);
            eprintln!("unable to open file: {:?}", e);
            std::process::exit(1);
        }
    }
}
pub fn ast_check_dir(folder: PathBuf) {
    jwalk::WalkDir::new(canonicalize(folder).unwrap())
        // Check in the same order always
        .sort(true)
        .into_iter()
        .filter_map(|a| a.ok())
        .filter(|e| !e.file_type().is_dir())
        .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
        .filter(|e| e.ends_with(".java"))
        .filter(|e| !e.ends_with("module-info.java"))
        .filter(|e| !e.ends_with("package-info.java"))
        .for_each(|i| {
            ast_check(i.into());
        });
    println!("Checked all files.")
}
