#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::{num::TryFromIntError, str::FromStr, sync::Arc};

use ast::{
    error::{AstError, get_pos},
    lexer::{LexerError, PositionToken},
    types::{AstPoint, AstRange},
};
use common::TaskProgress;
use lsp_server::{Connection, Message};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, Position, ProgressParams, ProgressParamsValue, ProgressToken,
    PublishDiagnosticsParams, Range, ShowDocumentParams, Uri, WorkDoneProgress,
    WorkDoneProgressBegin, WorkDoneProgressEnd, WorkDoneProgressReport,
    notification::{Notification, Progress, PublishDiagnostics},
    request::{Request, ShowDocument},
};
use my_string::{NuVec, NuVecBuilder};

pub const SERVER_NAME: &str = "java_lsp";

#[derive(Debug)]
pub enum ToLspRangeError {
    Int(TryFromIntError),
}
pub fn to_lsp_position(point: AstPoint) -> Result<Position, ToLspRangeError> {
    let el = u32::try_from(point.line).map_err(ToLspRangeError::Int)?;
    let ec = u32::try_from(point.col).map_err(ToLspRangeError::Int)?;

    Ok(Position {
        line: el,
        character: ec,
    })
}
pub fn to_lsp_range(range: &AstRange) -> Result<Range, ToLspRangeError> {
    let sl = u32::try_from(range.start.line).map_err(ToLspRangeError::Int)?;
    let sc = u32::try_from(range.start.col).map_err(ToLspRangeError::Int)?;
    let el = u32::try_from(range.end.line).map_err(ToLspRangeError::Int)?;
    let ec = u32::try_from(range.end.col).map_err(ToLspRangeError::Int)?;

    Ok(Range {
        start: Position {
            line: sl,
            character: sc,
        },
        end: Position {
            line: el,
            character: ec,
        },
    })
}

#[must_use]
pub fn to_ast_point(position: lsp_types::Position) -> AstPoint {
    AstPoint::new(
        position.line.try_into().unwrap_or_default(),
        position.character.try_into().unwrap_or_default(),
    )
}

#[must_use]
pub fn lexer_error_to_diagnostic(error: &LexerError) -> Diagnostic {
    match error {
        LexerError::EOF(line, col) => {
            Diagnostic::new_simple(line_col_to_range(*line, *col), String::from("EOF"))
        }
    }
}

fn line_col_to_range(line: usize, col: usize) -> Range {
    Range {
        start: Position {
            line: u32::try_from(line).unwrap_or_default(),
            character: u32::try_from(col).unwrap_or_default(),
        },
        end: Position {
            line: u32::try_from(line).unwrap_or_default(),
            character: u32::try_from(col).unwrap_or_default(),
        },
    }
}

#[derive(Debug)]
pub enum SourceToUriError {
    UriInvalid { uri: String, error: String },
}

#[cfg(not(windows))]
pub fn source_to_uri(source: &NuVec) -> Result<Uri, SourceToUriError> {
    let source = path_without_subclass(source);
    let str_uri = format!("file://{source}");
    let uri = Uri::from_str(&str_uri);
    match uri {
        Ok(uri) => Ok(uri),
        Err(e) => Err(SourceToUriError::UriInvalid {
            uri: str_uri,
            error: format!("{e:?}"),
        }),
    }
}
#[cfg(windows)]
pub fn source_to_uri(source: &NuVec) -> Result<Uri, SourceToUriError> {
    #[cfg(windows)]
    let source = &source
        .trim_start_matches(b"\\\\?\\")
        .replace_byte(b'\\', b'/');
    let source = path_without_subclass(source);
    let str_uri = format!("file:///{source}");
    let uri = Uri::from_str(&str_uri);
    match uri {
        Ok(uri) => Ok(uri),
        Err(e) => Err(SourceToUriError::UriInvalid {
            uri: str_uri,
            error: format!("{e:?}"),
        }),
    }
}
fn path_without_subclass(source: &NuVec) -> NuVec {
    if let Some((path, file_name)) = source.rsplit_once_byte(b'/')
        && file_name.contains_byte(b'$')
        && let Some((name, extension)) = file_name.split_once_byte(b'.')
        && let Some((name, _)) = name.split_once_byte(b'$')
    {
        let mut out = NuVecBuilder::new();
        out.extend(&path);
        out.push(b'/');
        out.extend(&name);
        out.push(b'.');
        out.extend(&extension);
        return out.finish();
    }
    source.clone()
}

pub enum AstDiagnosticError {
    TokensModified,
    Empty,
    ChildrenNotFound,
}

pub fn ast_error_to_diagnostic(
    err: &AstError,
    tokens: &[PositionToken],
) -> Result<Diagnostic, AstDiagnosticError> {
    match err {
        AstError::ExpectedToken(expected_token) => {
            let found = tokens
                .get(expected_token.pos)
                .ok_or(AstDiagnosticError::TokensModified)?;
            Ok(diag(
                format!(
                    "Expected token {:?} found: {:?}",
                    expected_token.expected, found.token
                ),
                found,
            ))
        }
        AstError::UnexpectedEOF => {
            if let Some(last) = tokens.last() {
                return Ok(diag("Unexpected end of File".to_string(), last));
            }
            Err(AstDiagnosticError::Empty)
        }
        AstError::InvalidJtype(invalid_token) => {
            let found = tokens
                .get(invalid_token.0)
                .ok_or(AstDiagnosticError::TokensModified)?;
            Ok(diag(
                format!(
                    "Invalid Type token found: {:?} valid onese ar Int, String",
                    found.token
                ),
                found,
            ))
        }
        AstError::IdentifierEmpty(invalid_token) => {
            let found = tokens
                .get(invalid_token.0)
                .ok_or(AstDiagnosticError::TokensModified)?;
            Ok(diag(
                format!("Identifier empty found: {:?}", found.token),
                found,
            ))
        }
        AstError::InvalidName(invalid_token) => {
            let found = tokens
                .get(invalid_token.0)
                .ok_or(AstDiagnosticError::TokensModified)?;
            Ok(diag(
                format!("Token not allowed in name: {:?}", found.token),
                found,
            ))
        }
        AstError::InvalidNuget(invalid_token) => {
            let found = tokens
                .get(invalid_token.0)
                .ok_or(AstDiagnosticError::TokensModified)?;
            Ok(diag(
                format!("Token not allowed in nuget: {:?}", found.token),
                found,
            ))
        }
        AstError::InvalidString(invalid_token) => {
            let found = tokens
                .get(invalid_token.0)
                .ok_or(AstDiagnosticError::TokensModified)?;
            Ok(diag(
                format!("Not a string literal: {:?}", found.token),
                found,
            ))
        }
        AstError::AllChildrenFailed2 { parent: _, errors } => {
            if let Some(e) = errors
                .iter()
                .flatten()
                .map(|i| (get_pos(&i.1), i))
                .max_by(|a, b| a.0.0.cmp(&b.0.0))
            {
                // e.1.1.print_err(content, tokens);
                return ast_error_to_diagnostic(&e.1.1, tokens);
            }
            Err(AstDiagnosticError::ChildrenNotFound)
        }
        AstError::AllChildrenFailed3 { parent: _, errors } => {
            if let Some(e) = errors
                .iter()
                .flatten()
                .map(|i| (get_pos(&i.1), i))
                .max_by(|a, b| a.0.0.cmp(&b.0.0))
            {
                // e.1.1.print_err(content, tokens);
                return ast_error_to_diagnostic(&e.1.1, tokens);
            }
            Err(AstDiagnosticError::ChildrenNotFound)
        }
        AstError::AllChildrenFailed4 { parent: _, errors } => {
            if let Some(e) = errors
                .iter()
                .flatten()
                .map(|i| (get_pos(&i.1), i))
                .max_by(|a, b| a.0.0.cmp(&b.0.0))
            {
                // e.1.1.print_err(content, tokens);
                return ast_error_to_diagnostic(&e.1.1, tokens);
            }
            Err(AstDiagnosticError::ChildrenNotFound)
        }
        AstError::AllChildrenFailed5 { parent: _, errors } => {
            if let Some(e) = errors
                .iter()
                .flatten()
                .map(|i| (get_pos(&i.1), i))
                .max_by(|a, b| a.0.0.cmp(&b.0.0))
            {
                // e.1.1.print_err(content, tokens);
                return ast_error_to_diagnostic(&e.1.1, tokens);
            }
            Err(AstDiagnosticError::ChildrenNotFound)
        }
        AstError::AllChildrenFailed6 { parent: _, errors } => {
            if let Some(e) = errors
                .iter()
                .flatten()
                .map(|i| (get_pos(&i.1), i))
                .max_by(|a, b| a.0.0.cmp(&b.0.0))
            {
                // e.1.1.print_err(content, tokens);
                return ast_error_to_diagnostic(&e.1.1, tokens);
            }
            Err(AstDiagnosticError::ChildrenNotFound)
        }
        AstError::AllChildrenFailed7 { parent: _, errors } => {
            if let Some(e) = errors
                .iter()
                .flatten()
                .map(|i| (get_pos(&i.1), i))
                .max_by(|a, b| a.0.0.cmp(&b.0.0))
            {
                // e.1.1.print_err(content, tokens);
                return ast_error_to_diagnostic(&e.1.1, tokens);
            }
            Err(AstDiagnosticError::ChildrenNotFound)
        }
        AstError::AllChildrenFailed26 { parent: _, errors } => {
            if let Some(e) = errors
                .iter()
                .flatten()
                .map(|i| (get_pos(&i.1), i))
                .max_by(|a, b| a.0.0.cmp(&b.0.0))
            {
                // e.1.1.print_err(content, tokens);
                return ast_error_to_diagnostic(&e.1.1, tokens);
            }
            Err(AstDiagnosticError::ChildrenNotFound)
        }
        AstError::EmptyExpression(invalid_token) => {
            let found = tokens
                .get(invalid_token.0)
                .ok_or(AstDiagnosticError::TokensModified)?;
            Ok(diag(
                format!("Invalid expression it is empty {:?}", found.token),
                found,
            ))
        }
        AstError::FordbidenExpressionCall(invalid_token) => {
            let found = tokens
                .get(invalid_token.0)
                .ok_or(AstDiagnosticError::TokensModified)?;
            Ok(diag(
                format!("Method call now allowed {:?}", found.token),
                found,
            ))
        }
    }
}

fn diag(message: String, found: &PositionToken) -> Diagnostic {
    let p = Position {
        line: u32::try_from(found.line).unwrap_or_default(),
        character: u32::try_from(found.col).unwrap_or_default(),
    };
    let range = Range::new(p, p);
    Diagnostic::new(
        range,
        Some(DiagnosticSeverity::ERROR),
        None,
        None,
        message,
        None,
        None,
    )
}

pub fn progress_start_option_token(
    con: &Arc<Connection>,
    token: &Arc<Option<ProgressToken>>,
    title: &str,
) {
    if let Some(token) = token.as_ref() {
        progress_start_token(con, token, title);
        return;
    }
    progress_start(con, title);
}
fn progress_start_token(con: &Arc<Connection>, token: &ProgressToken, title: &str) {
    eprintln!("Start progress on: {title}");
    if let Ok(params) = serde_json::to_value(ProgressParams {
        token: token.to_owned(),
        value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(WorkDoneProgressBegin {
            title: title.to_owned(),
            cancellable: None,
            message: None,
            percentage: None,
        })),
    }) {
        let _ = con
            .sender
            .send(Message::Notification(lsp_server::Notification {
                method: Progress::METHOD.to_string(),
                params,
            }));
    }
}

pub fn progress_start(con: &Arc<Connection>, task: &str) {
    let token = ProgressToken::String(task.to_owned());
    progress_start_token(con, &token, task);
}
pub fn progress_update_percentage_option_token(
    con: &Arc<Connection>,
    token: &Arc<Option<ProgressToken>>,
    task: &str,
    message: String,
    percentage: u32,
) {
    if let Some(token) = token.as_ref() {
        progress_update_percentage_token(con, token, task, message, percentage);
        return;
    }
    progress_update_percentage(con, task, message, percentage);
}
pub fn progress_update_percentage(
    con: &Arc<Connection>,
    task: &str,
    message: String,
    percentage: u32,
) {
    let token = ProgressToken::String(task.to_owned());
    progress_update_percentage_token(con, &token, task, message, percentage);
}
pub fn progress_update_percentage_token(
    con: &Arc<Connection>,
    token: &ProgressToken,
    task: &str,
    message: String,
    percentage: u32,
) {
    eprintln!("Report progress on: {task} {percentage:?} status: {message}");
    if let Ok(params) = serde_json::to_value(ProgressParams {
        token: token.to_owned(),
        value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(WorkDoneProgressReport {
            cancellable: Some(false),
            message: Some(message),
            percentage: Some(percentage),
        })),
    }) {
        let _ = con
            .sender
            .try_send(Message::Notification(lsp_server::Notification {
                method: Progress::METHOD.to_string(),
                params,
            }));
    }
}
pub fn progress_end_option_token(
    con: &Arc<Connection>,
    token: &Arc<Option<ProgressToken>>,
    task: &str,
) {
    if let Some(token) = token.as_ref() {
        progress_end_token(con, token, task);
        return;
    }
    progress_end(con, task);
}
pub fn progress_end_token(con: &Arc<Connection>, token: &ProgressToken, task: &str) {
    eprintln!("End progress on: {task}");
    if let Ok(params) = serde_json::to_value(ProgressParams {
        token: token.to_owned(),
        value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
            message: None,
        })),
    }) {
        let _ = con
            .sender
            .send(Message::Notification(lsp_server::Notification {
                method: Progress::METHOD.to_string(),
                params,
            }));
    }
}
pub fn progress_end(con: &Arc<Connection>, task: &str) {
    let token = ProgressToken::String(task.to_owned());
    progress_end_token(con, &token, task);
}
pub fn open_log(con: &Connection, path: &NuVec) {
    if let Ok(uri) = source_to_uri(path)
        && let Ok(params) = serde_json::to_value(ShowDocumentParams {
            uri,
            external: None,
            take_focus: Some(true),
            selection: None,
        })
    {
        let _ = con.sender.send(Message::Request(lsp_server::Request {
            id: 1.into(),
            method: ShowDocument::METHOD.to_string(),
            params,
        }));
    }
}

pub fn send_diagnostic(con: &Arc<Connection>, uri: Uri, diagnostics: Vec<Diagnostic>) {
    if let Ok(params) = serde_json::to_value(PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    }) {
        let _ = con
            .sender
            .send(Message::Notification(lsp_server::Notification {
                method: PublishDiagnostics::METHOD.to_string(),
                params,
            }));
    }
}

pub async fn read_forward(
    mut rx: tokio::sync::watch::Receiver<TaskProgress>,
    con: Arc<Connection>,
    task: String,
    token: Arc<Option<ProgressToken>>,
) {
    loop {
        if rx.changed().await.is_err() {
            break;
        }
        let i = rx.borrow();
        progress_update_percentage_option_token(
            &con.clone(),
            &token,
            &task,
            i.message.clone(),
            i.percentage,
        );
    }
}
