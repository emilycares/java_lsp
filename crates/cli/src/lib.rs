#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unused_async)]
use std::io::{Read, Write};
use std::path::Path;
use std::time::Instant;
use std::{fs::canonicalize, path::PathBuf};

use ast::error::PrintErr;
use ast::lexer::PositionToken;
use editorconfig::EditorConfigFilled;
use jdk::{test_load_jdk_jmod, test_load_jdk_modules_executable, test_load_jdk_modules_own};

pub fn print_help() {
    println!(
        "
java-lsp

--help : Shows this help

no arg : Starts lsp over stdio
server-tcp <port> : Open tcp lsp socket

reload-deps : Reload dependencies of current project
update-deps : Update dependencies of current project

format <path> : Format java file or all files in directory
format -p : Format java file from stdin and print to stdout

lex <file path to java file> : Print tokens from file
lex-pos <file path to java file> <index> : Print token at position
ast-check <file path to java file> : Check for ast errors in java file
ast-check-dir <directory path> <Optional ignore pattern> : Check for ast errors in directory
ast-check-jdk : Check for ast errors in current jdk in path
index-jdk <variant> : Index jdk in path with variant jimage-own/jimage-executable/jmod
"
    );
}

pub fn parse(args: &[String]) -> Option<Command> {
    let args = &args[1..];
    match args.first().map(std::string::String::as_str) {
        None => Some(Command::Server),
        Some("server-tcp") => parse_server_tcp(&args[1..]),
        Some("reload-deps") => Some(Command::ReloadDependencies),
        Some("update-deps") => Some(Command::UpdateDependencies),
        Some("format") => parse_format(&args[1..]),
        Some("lex") => parse_lex(&args[1..]),
        Some("lex-pos") => parse_lex_pos(&args[1..]),
        Some("ast-check") => parse_ast_check(&args[1..]),
        Some("ast-check-dir") => parse_ast_check_dir(&args[1..]),
        Some("ast-check-jdk") => Some(Command::AstCheckJdk),
        Some("index-jdk") => parse_index_jdk(&args[1..]),
        Some("--help") => Some(Command::Help),
        // for vscode
        Some("--stdio") => None,
        Some(u) => {
            println!("unknown option {u}");
            None
        }
    }
}

fn parse_index_jdk(args: &[String]) -> Option<Command> {
    match args.first().map(std::string::String::as_str) {
        Some("jimage-own") => Some(Command::IndexJdk {
            variant: IndexJdkOptions::JimageOwn,
        }),
        Some("jimage-executable") => Some(Command::IndexJdk {
            variant: IndexJdkOptions::JimageExecutable,
        }),
        Some("jmod") => Some(Command::IndexJdk {
            variant: IndexJdkOptions::Jmod,
        }),
        None | Some(_) => {
            println!("must provide variant jimage-own/jimage-executable/jmod");
            None
        }
    }
}

fn parse_ast_check(args: &[String]) -> Option<Command> {
    args.first().map_or_else(
        || {
            println!("Expected file path");
            None
        },
        |path| {
            let path = PathBuf::from(path);
            Some(Command::AstCheck { file: path })
        },
    )
}

fn parse_ast_check_dir(args: &[String]) -> Option<Command> {
    args.first().map_or_else(
        || {
            println!("Expected directory path");
            None
        },
        |path| {
            let path = PathBuf::from(path);
            if let Some(ignore) = args.get(1) {
                Some(Command::AstCheckDir {
                    folder: path,
                    ignore: Some(ignore.clone()),
                })
            } else {
                Some(Command::AstCheckDir {
                    folder: path,
                    ignore: None,
                })
            }
        },
    )
}

fn parse_lex(args: &[String]) -> Option<Command> {
    args.first().map_or_else(
        || {
            println!("Expected file path");
            None
        },
        |path| {
            let path = PathBuf::from(path);
            Some(Command::Lex { file: path })
        },
    )
}

fn parse_format(args: &[String]) -> Option<Command> {
    args.first().map_or_else(
        || {
            println!("Expected file path or -p");
            None
        },
        |path| {
            if path == "-p" {
                return Some(Command::FormatFilePiped);
            }
            let path = PathBuf::from(path);
            if path.is_dir() {
                return Some(Command::FormatDir(path));
            }
            Some(Command::FormatFile(path))
        },
    )
}

fn parse_lex_pos(args: &[String]) -> Option<Command> {
    args.first().map_or_else(
        || {
            println!("Expected file path");
            None
        },
        |path| {
            args.get(1).map_or_else(
                || {
                    println!("Expected position");
                    None
                },
                |index| {
                    let path = PathBuf::from(path);
                    Some(Command::LexPos {
                        file: path,
                        pos: index.parse().unwrap_or_default(),
                    })
                },
            )
        },
    )
}

fn parse_server_tcp(args: &[String]) -> Option<Command> {
    args.first().map_or_else(
        || {
            println!("Expected tcp port");
            None
        },
        |port| {
            port.parse().map_or_else(
                |_| {
                    println!("Port must be a number");
                    None
                },
                |port| Some(Command::ServerTcp { port }),
            )
        },
    )
}

#[derive(Debug)]
pub enum Command {
    // Print help
    Help,
    /// Start the lsp server over stdio
    Server,
    /// Start the lsp server tcp with specified port
    ServerTcp {
        port: u16,
    },
    /// Reloads the dependencies of project
    ReloadDependencies,
    /// Update the dependencies of project
    UpdateDependencies,
    /// Get tokens from file
    Lex {
        file: PathBuf,
    },
    /// Get tokens from file at pos
    LexPos {
        file: PathBuf,
        pos: usize,
    },
    /// Check for errors in file
    AstCheck {
        file: PathBuf,
    },
    /// Recusivly check a directory for ast for java files
    AstCheckDir {
        folder: PathBuf,
        ignore: Option<String>,
    },
    /// Check jdk in path
    AstCheckJdk,
    IndexJdk {
        variant: IndexJdkOptions,
    },
    FormatDir(PathBuf),
    FormatFile(PathBuf),
    FormatFilePiped,
}

#[derive(Clone, Debug)]
pub enum IndexJdkOptions {
    JimageExecutable,
    JimageOwn,
    Jmod,
}

pub fn ast_check(path: &PathBuf, tokens: &mut Vec<PositionToken>) {
    match std::fs::read(path) {
        Ok(data) => lex_and_ast(path, &data, tokens),
        Err(e) => {
            eprintln!("unable to open file: {e:?}");
            std::process::exit(1);
        }
    }
}

fn lex_and_ast(file: &Path, text: &[u8], tokens: &mut Vec<PositionToken>) {
    // eprintln!("Here: {:?}", file);
    match ast::lexer::lex_mut::<false>(text, tokens) {
        Ok(()) => {
            let ast = ast::parse_file(tokens);
            if ast.is_err() {
                eprintln!("Here: {}", file.display());
                ast.print_err(text, tokens);
            }
        }
        Err(e) => {
            eprintln!("Here: {}", file.display());
            eprintln!("Lexer error: {e:?}");
            std::process::exit(2);
        }
    }
}

fn visit_java_fies(
    dir: &PathBuf,
    tokens: &mut Vec<PositionToken>,
    dirs: &mut std::collections::VecDeque<PathBuf>,
    cb: impl Fn(&PathBuf, &mut Vec<PositionToken>),
) -> Result<(), std::io::Error> {
    let read_dir = std::fs::read_dir(dir)?
        .map(|res| res.map(|e| e.path()))
        .filter_map(Result::ok);
    for entry in read_dir {
        if entry.is_dir() {
            dirs.push_back(entry);
        } else if let Some(e) = entry.extension()
            && e == "java"
        {
            cb(&entry, tokens);
        }
    }
    Ok(())
}
pub fn ast_check_dir(folder: PathBuf) -> Result<(), std::io::Error> {
    let time = Instant::now();
    let dir = canonicalize(folder)?;
    let mut dirs = std::collections::VecDeque::new();
    let mut tokens = Vec::with_capacity(8192);
    dirs.push_back(dir);
    while let Some(dir) = dirs.pop_front() {
        visit_java_fies(&dir, &mut tokens, &mut dirs, ast_check)?;
    }
    println!("Checked all files. in: {:.2?}", time.elapsed());
    Ok(())
}
pub fn ast_check_dir_ignore(folder: PathBuf, ignore: &[String]) -> Result<(), std::io::Error> {
    let time = Instant::now();
    let dir = canonicalize(folder)?;
    let mut tokens = Vec::new();
    let mut dirs = std::collections::VecDeque::new();
    dirs.push_back(dir);
    while let Some(dir) = dirs.pop_front() {
        visit_java_fies(&dir, &mut tokens, &mut dirs, |i, tokens| {
            if let Some(s) = i.to_str() {
                for ig in ignore {
                    if s.contains(ig) {
                        return;
                    }
                }
            }
            ast_check(i, tokens);
        })?;
    }
    println!("Checked all files. in: {:.2?}", time.elapsed());
    Ok(())
}

/// # Panics
/// When lexer fails or file issue
pub fn lex(file: &PathBuf) {
    let bytes = std::fs::read(file).expect("File should exist");
    let tokens = ast::lexer::lex(&bytes).expect("Ok to cratch if fail");
    eprintln!("{tokens:?}");
}
/// # Panics
/// When lexer fails or file issue
pub fn lex_pos(file: &PathBuf, pos: usize) {
    let bytes = std::fs::read(file).expect("File should exist");
    let tokens = ast::lexer::lex(&bytes).expect("Ok to cratch if fail");
    eprintln!("{:?}", tokens[pos]);
}

pub async fn index_jdk(variant: IndexJdkOptions) {
    match variant {
        IndexJdkOptions::JimageOwn => test_load_jdk_modules_own().await,
        IndexJdkOptions::JimageExecutable => test_load_jdk_modules_executable().await,
        IndexJdkOptions::Jmod => test_load_jdk_jmod().await,
    }
}

pub fn format_file(
    p: &PathBuf,
    tokens: &mut Vec<PositionToken>,
    exit: bool,
    editorconfig: &EditorConfigFilled,
) {
    match std::fs::read(p) {
        Ok(data) => match ast::lexer::lex_mut::<false>(&data, tokens) {
            Ok(()) => match ast::parse_file(tokens) {
                Ok(ast) => {
                    match formatter::format(
                        &config::FormatterConfig::Internal,
                        &ast,
                        &data,
                        editorconfig,
                    ) {
                        Ok(o) => {
                            if let Err(e) = std::fs::write(p, o) {
                                eprintln!("Here: {}", p.display());
                                eprintln!("write error: {e:?}");
                                if exit {
                                    std::process::exit(4);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Here: {}", p.display());
                            eprintln!("Format error: {e:?}");
                            if exit {
                                std::process::exit(3);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Here: {}", p.display());
                    e.print_err(&data, tokens);
                    std::process::exit(3);
                }
            },
            Err(e) => {
                eprintln!("Here: {}", p.display());
                eprintln!("Lexer error: {e:?}");
                if exit {
                    std::process::exit(2);
                }
            }
        },
        Err(e) => {
            eprintln!("unable to open file: {e:?}");
            if exit {
                std::process::exit(1);
            }
        }
    }
}

pub fn format_dir(p: &PathBuf) {
    let editorconfig = editorconfig::load_editor_config_or_default();
    let editorconfig = editorconfig.to_filled();

    let time = Instant::now();
    let Ok(dir) = canonicalize(p) else {
        return;
    };
    let mut dirs = std::collections::VecDeque::new();
    let mut tokens = Vec::with_capacity(8192);
    dirs.push_back(dir);
    while let Some(dir) = dirs.pop_front() {
        if let Err(e) = visit_java_fies(&dir, &mut tokens, &mut dirs, |i, tokens| {
            format_file(i, tokens, false, &editorconfig);
        }) {
            eprintln!("Error walking files: {e:?}");
        }
    }
    println!("Formatted files. in: {:.2?}", time.elapsed());
}

pub fn format_piped() {
    let mut data = Vec::new();
    if std::io::stdin().read_to_end(&mut data).is_err() {
        eprintln!("Unable to read stdin");
        std::process::exit(2);
    }

    let editorconfig = editorconfig::load_editor_config_or_default();
    let editorconfig = editorconfig.to_filled();
    let mut tokens = Vec::new();
    match ast::lexer::lex_mut::<false>(&data, &mut tokens) {
        Ok(()) => match ast::parse_file(&tokens) {
            Ok(ast) => {
                match formatter::format(
                    &config::FormatterConfig::Internal,
                    &ast,
                    &data,
                    &editorconfig,
                ) {
                    Ok(o) => {
                        if std::io::stdout().write_all(&o).is_err() {
                            std::process::exit(4);
                        }
                    }
                    Err(e) => {
                        eprintln!("Format error: {e:?}");
                        std::process::exit(3);
                    }
                }
            }
            Err(e) => {
                eprintln!("Ast error: {e:?}");
            }
        },
        Err(e) => {
            eprintln!("Lexer error: {e:?}");
            std::process::exit(2);
        }
    }
}
