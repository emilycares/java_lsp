#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
use std::time::Instant;
use std::{fs::canonicalize, path::PathBuf};

use ast::error::PrintErr;
#[cfg(not(target_os = "windows"))]
use ast::lexer::PositionToken;
use clap::{Parser, Subcommand};
#[cfg(target_os = "windows")]
use std::sync::Arc;
use std::sync::mpsc;

#[derive(Debug)]
pub enum CheckError {
    IO(std::io::Error),
    ChannelSend(mpsc::SendError<PathBuf>),
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

#[cfg(target_os = "windows")]
pub fn ast_check(path: &PathBuf) {
    use std::{fs::File, str::from_utf8};

    match File::open(path) {
        Ok(file) => {
            let mmap = unsafe { memmap2::Mmap::map(&file) };
            match mmap {
                Ok(mmap) => {
                    #[cfg(unix)]
                    mmap.advise(memmap2::Advice::Sequential)
                        .expect("memmap advice to be accepted");
                    match from_utf8(&mmap[..]) {
                        Ok(text) => {
                            lex_and_ast(path, text);
                        }
                        Err(e) => {
                            eprintln!("invalid utf8: {:?}", e);
                            std::process::exit(3);
                        }
                    };
                }
                Err(e) => {
                    eprintln!("unable to memmap: {:?}", e);
                    std::process::exit(2);
                }
            };
        }
        Err(e) => {
            eprintln!("unable to open file: {:?}", e);
            std::process::exit(1);
        }
    };
}
#[cfg(not(target_os = "windows"))]
pub fn ast_check(path: &PathBuf, num: usize, tokens: &mut Vec<PositionToken>) {
    use std::fs::File;

    match File::open(path) {
        Ok(file) => {
            let mmap = unsafe { memmap2::Mmap::map(&file) };
            match mmap {
                Ok(mmap) => {
                    #[cfg(unix)]
                    mmap.advise(memmap2::Advice::Sequential)
                        .expect("memmap advice to be accepted");
                    lex_and_ast(path, &mmap, num, tokens);
                }
                Err(e) => {
                    eprintln!("unable to memmap: {:?}", e);
                    std::process::exit(2);
                }
            };
        }
        Err(e) => {
            eprintln!("unable to open file: {:?}", e);
            std::process::exit(1);
        }
    };
}

#[cfg(not(target_os = "windows"))]
fn lex_and_ast(file: &PathBuf, text: &[u8], num: usize, tokens: &mut Vec<PositionToken>) {
    // eprintln!("[{num}]Here: {:?}", file);
    match ast::lexer::lex_mut(text, tokens) {
        Ok(_) => {
            let ast = ast::parse_file(tokens);
            if ast.is_err() {
                eprintln!("[{num}]Here: {:?}", file);
                if let Ok(text) = str::from_utf8(text) {
                    ast.print_err(text, tokens);
                }
                std::process::exit(3);
            }
        }
        Err(e) => {
            eprintln!("[{num}]Lexer error: {:?}", e);
            std::process::exit(2);
        }
    }
}
#[cfg(target_os = "windows")]
fn lex_and_ast(file: &PathBuf, text: &str) {
    // eprintln!("Here: {:?}", file);
    match ast::lexer::lex(text) {
        Ok(tokens) => {
            let ast = ast::parse_file(&tokens);
            if ast.is_err() {
                eprintln!("Here: {:?}", file);
                ast.print_err(text, &tokens);
                std::process::exit(3);
            }
        }
        Err(e) => {
            eprintln!("Lexer error: {:?}", e);
            std::process::exit(2);
        }
    }
}
#[cfg(not(target_os = "windows"))]
pub async fn ast_check_dir(folder: PathBuf) -> Result<(), CheckError> {
    let mut count = 0;
    let time = Instant::now();
    let mut tokens = Vec::new();
    for i in jwalk::WalkDir::new(canonicalize(folder).expect("Canonicalize fail"))
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
        .enumerate()
    {
        count += 1;
        ast_check(&i.1, i.0, &mut tokens);
    }
    println!("Checked all files. {count}, in: {:.2?}", time.elapsed());
    Ok(())
}
#[cfg(not(target_os = "windows"))]
pub async fn ast_check_dir_ignore(folder: PathBuf, ignore: Vec<String>) -> Result<(), CheckError> {
    let mut count = 0;
    let time = Instant::now();
    let mut tokens = Vec::new();
    for i in jwalk::WalkDir::new(canonicalize(folder).expect("Canonicalize fail"))
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
        .enumerate()
    {
        count += 1;
        ast_check(&i.1, i.0, &mut tokens);
    }
    println!("Checked all files. {count}, in: {:.2?}", time.elapsed());
    Ok(())
}

#[cfg(target_os = "windows")]
fn visit_java_fies(
    dir: &PathBuf,
    tx: Arc<mpsc::Sender<PathBuf>>,
    cb: impl Fn(&PathBuf),
) -> Result<(), CheckError> {
    let read_dir = std::fs::read_dir(dir)
        .map_err(CheckError::IO)?
        .map(|res| res.map(|e| e.path()))
        .filter_map(|a| a.ok());
    for entry in read_dir {
        if entry.is_dir() {
            tx.send(entry).map_err(CheckError::ChannelSend)?;
        } else if let Some(e) = entry.extension()
            && e == "java"
        {
            cb(&entry);
        }
    }
    Ok(())
}
#[cfg(target_os = "windows")]
pub async fn ast_check_dir(folder: PathBuf) -> Result<(), CheckError> {
    use tokio::task::JoinSet;

    use std::time::Duration;

    let time = Instant::now();
    let dir = canonicalize(folder).map_err(CheckError::IO)?;
    let (tx, rx) = mpsc::channel();
    tx.send(dir).map_err(CheckError::ChannelSend)?;
    let tx = Arc::new(tx);
    let mut handles = JoinSet::new();
    while let Ok(dir) = rx.recv_timeout(Duration::from_millis(300)) {
        let tx = tx.clone();
        handles.spawn(async move { visit_java_fies(&dir, tx, ast_check) });
    }
    let _ = handles.join_all().await;

    println!("Checked all files. in: {:.2?}", time.elapsed());
    Ok(())
}
#[cfg(target_os = "windows")]
pub async fn ast_check_dir_ignore(folder: PathBuf, ignore: Vec<String>) -> Result<(), CheckError> {
    use std::time::Duration;

    use tokio::task::JoinSet;

    let time = Instant::now();
    let (tx, rx) = mpsc::channel();
    let dir = canonicalize(folder).map_err(CheckError::IO)?;
    tx.send(dir).map_err(CheckError::ChannelSend)?;
    let tx = Arc::new(tx);
    let mut handles = JoinSet::new();
    let ignore = Arc::new(ignore);
    while let Ok(dir) = rx.recv_timeout(Duration::from_millis(300)) {
        let tx = tx.clone();
        let ignore = ignore.clone();
        handles.spawn(async move {
            visit_java_fies(&dir, tx, |i| {
                if let Some(s) = i.to_str() {
                    for ig in ignore.iter() {
                        if s.contains(ig) {
                            return;
                        }
                    }
                }
                ast_check(i)
            })
        });
    }
    let _ = handles.join_all().await;
    println!("Checked all files. in: {:.2?}", time.elapsed());
    Ok(())
}

pub fn lex(file: PathBuf) {
    let bytes = std::fs::read(&file).expect("File should exist");
    let tokens = ast::lexer::lex(&bytes).expect("Ok to cratch if fail");
    eprintln!("{:?}", tokens);
}
pub fn lex_pos(file: PathBuf, pos: usize) {
    let bytes = std::fs::read(&file).expect("File should exist");
    let tokens = ast::lexer::lex(&bytes).expect("Ok to cratch if fail");
    eprintln!("{:?}", tokens[pos]);
}
