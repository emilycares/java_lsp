#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
use std::time::Instant;
use std::{fs::canonicalize, path::PathBuf};

use ast::error::PrintErr;
use clap::{Parser, Subcommand};
use tokio::fs::read_to_string;

#[derive(Debug)]
pub enum CheckError {
    IO(std::io::Error),
}

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
    /// Get tokens from file
    Lex { file: PathBuf },
    /// Get tokens from file at pos
    LexPos { file: PathBuf, pos: usize },
    /// Check for errors in file
    AstCheck { file: PathBuf },
    /// Recusivly check a directory for ast for java files
    AstCheckDir {
        folder: PathBuf,
        #[arg(long)]
        ignore: Option<String>,
    },
    /// Check jdk in path
    AstCheckJdk,
}

pub async fn ast_check_async(file: PathBuf, num: usize) {
    match read_to_string(&file).await {
        Ok(text) => {
            // let before_lex = Instant::now();
            lex_and_ast(&file, text, num);
        }
        Err(e) => {
            eprintln!("unable to open file: {:?}", e);
            std::process::exit(1);
        }
    }
}
pub fn ast_check(file: &PathBuf, num: usize) {
    match std::fs::read_to_string(file) {
        Ok(text) => {
            // let before_lex = Instant::now();
            lex_and_ast(file, text, num);
        }
        Err(e) => {
            eprintln!("[{num}]Here: {:?}", file);
            eprintln!("unable to open file: {:?}", e);
            std::process::exit(1);
        }
    }
}

fn lex_and_ast(file: &PathBuf, text: String, num: usize) {
    eprintln!("[{num}]Here: {:?}", file);
    match ast::lexer::lex(&text) {
        Ok(tokens) => {
            // let lex_time = before_lex.elapsed();

            // print!(
            //     "[{num}]Timings: [lexer: {:.2?}, tokens_len: {}]",
            //     lex_time,
            //     tokens.len()
            // );
            // let before_ast = Instant::now();
            let ast = ast::parse_file(&tokens);
            // let ast_time = before_ast.elapsed();
            if ast.is_err() {
                eprintln!("[{num}]Here: {:?}", file);
                ast.print_err(&text);
                std::process::exit(3);
            }
        }
        Err(e) => {
            eprintln!("[{num}]Lexer error: {:?}", e);
            std::process::exit(2);
        }
    }
}
#[cfg(not(target_os = "windows"))]
pub async fn ast_check_dir(folder: PathBuf) -> Result<(), CheckError> {
    let mut count = 0;
    let time = Instant::now();
    for i in jwalk::WalkDir::new(canonicalize(folder).expect("Cannonicalize fail"))
        // Check in the same order always
        .sort(true)
        // .follow_links(true)
        .into_iter()
        .filter_map(|a| a.ok())
        .filter(|e| !e.file_type().is_dir())
        .map(|i| i.path())
        .filter(|i| {
            if let Some(e) = i.extension() {
                return e == "java";
            }
            false
        })
        // .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
        // .filter(|e| !e.ends_with("module-info.java"))
        // .filter(|e| !e.ends_with("package-info.java"))
        .enumerate()
    {
        count += 1;
        ast_check_async(i.1, i.0).await;
    }
    println!("Checked all files. {count}, in: {:.2?}", time.elapsed());
    Ok(())
}
#[cfg(not(target_os = "windows"))]
pub async fn ast_check_dir_ignore(folder: PathBuf, ignore: Vec<&str>) -> Result<(), CheckError> {
    let mut count = 0;
    let time = Instant::now();
    for i in jwalk::WalkDir::new(canonicalize(folder).expect("Cannonicalize fail"))
        // Check in the same order always
        .sort(true)
        // .follow_links(true)
        .into_iter()
        .filter_map(|a| a.ok())
        .filter(|e| !e.file_type().is_dir())
        .map(|i| i.path())
        .filter(|i| {
            if let Some(s) = i.to_str() {
                for ig in &ignore {
                    if s.contains(ig) {
                        return false;
                    }
                }
            }
            if let Some(e) = i.extension() {
                return e == "java";
            }
            false
        })
        // .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
        // .filter(|e| !e.ends_with("module-info.java"))
        // .filter(|e| !e.ends_with("package-info.java"))
        .enumerate()
    {
        count += 1;
        ast_check_async(i.1, i.0).await;
    }
    println!("Checked all files. {count}, in: {:.2?}", time.elapsed());
    Ok(())
}

#[cfg(target_os = "windows")]
fn visit_java_fies(
    dir: &std::path::Path,
    index: usize,
    cb: &dyn Fn(&PathBuf, usize),
) -> Result<usize, CheckError> {
    if dir.is_dir() {
        let mut read_dir: Vec<_> = std::fs::read_dir(dir)
            .map_err(CheckError::IO)?
            .map(|res| res.map(|e| e.path()))
            .filter_map(|a| a.ok())
            .collect();
        read_dir.sort();
        let mut index = index;
        for entry in read_dir.iter() {
            if entry.is_dir() {
                let nindex = visit_java_fies(&entry, index, cb)?;
                index = nindex;
            } else {
                if let Some(e) = entry.extension() {
                    if e == "java" {
                        index += 1;
                        cb(&entry, index);
                    }
                }
            }
        }
        return Ok(index);
    }
    Ok(0)
}
#[cfg(target_os = "windows")]
pub async fn ast_check_dir(folder: PathBuf) -> Result<(), CheckError> {
    let time = Instant::now();
    visit_java_fies(
        canonicalize(folder).expect("Cannonicalize fail").as_path(),
        0,
        &|i, index| ast_check(i, index),
    )?;
    println!("Checked all files. in: {:.2?}", time.elapsed());
    Ok(())
}
#[cfg(target_os = "windows")]
pub async fn ast_check_dir_ignore(folder: PathBuf, ignore: Vec<&str>) -> Result<(), CheckError> {
    let time = Instant::now();
    visit_java_fies(
        canonicalize(folder).expect("Cannonicalize fail").as_path(),
        0,
        &|i, index| {
            if let Some(s) = i.to_str() {
                for ig in &ignore {
                    if s.contains(ig) {
                        return;
                    }
                }
            }
            ast_check(i, index)
        },
    )?;
    println!("Checked all files. in: {:.2?}", time.elapsed());
    Ok(())
}

pub fn lex(file: PathBuf) {
    let text = std::fs::read_to_string(&file).expect("File shoul exist");
    let tokens = ast::lexer::lex(&text).expect("Ok to crach if fail");
    eprintln!("{:?}", tokens);
}
pub fn lex_pos(file: PathBuf, pos: usize) {
    let text = std::fs::read_to_string(&file).expect("File shoul exist");
    let tokens = ast::lexer::lex(&text).expect("Ok to crach if fail");
    eprintln!("{:?}", tokens[pos]);
}
