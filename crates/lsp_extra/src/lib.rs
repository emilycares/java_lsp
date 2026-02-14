#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::{num::TryFromIntError, str::FromStr};

use ast::{
    error::{AstError, get_pos},
    lexer::{LexerError, PositionToken},
    types::{AstPoint, AstRange},
};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Uri};

pub const SERVER_NAME: &str = "java_lsp";

#[derive(Debug)]
pub enum ToLspRangeError {
    Int(TryFromIntError),
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
        LexerError::UnknownChar(char, line, col) => Diagnostic::new_simple(
            Range {
                start: Position {
                    line: u32::try_from(*line).unwrap_or_default(),
                    character: u32::try_from(*col).unwrap_or_default(),
                },
                end: Position {
                    line: u32::try_from(*line).unwrap_or_default(),
                    character: u32::try_from(*col).unwrap_or_default(),
                },
            },
            format!("Unexpected char: '{char}'"),
        ),
    }
}

#[derive(Debug)]
pub enum SourceToUriError {
    UriInvalid { uri: String, error: String },
}
#[cfg(not(windows))]
pub fn source_to_uri(source: &str) -> Result<Uri, SourceToUriError> {
    #[cfg(windows)]
    let source = &source.trim_start_matches("\\\\?\\").replace('\\', "/");
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
pub fn source_to_uri(source: &str) -> Result<Uri, SourceToUriError> {
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
fn path_without_subclass(source: &str) -> String {
    if let Some((path, file_name)) = source.rsplit_once('/')
        && file_name.contains('$')
        && let Some((name, extension)) = file_name.split_once('.')
        && let Some((name, _)) = name.split_once('$')
    {
        return format!("{path}/{name}.{extension}");
    }
    source.to_owned()
}

#[must_use]
/// # Panics
/// When tokens vec has been mutated
pub fn ast_error_to_diagnostic(err: &AstError, tokens: &[PositionToken]) -> Option<Diagnostic> {
    match err {
        AstError::ExpectedToken(expected_token) => {
            let found = tokens
                .get(expected_token.pos)
                .expect("Tokens should not be changed");
            Some(diag(
                format!(
                    "Expected token {:?} found: {:?}",
                    expected_token.expected, found.token
                ),
                found,
            ))
        }
        AstError::UnexpectedEOF => {
            if let Some(last) = tokens.last() {
                return Some(diag("Unexpected end of File".to_string(), last));
            }
            None
        }
        AstError::InvalidJtype(invalid_token) => {
            let found = tokens
                .get(invalid_token.0)
                .expect("Tokens should not be changed");
            Some(diag(
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
                .expect("Tokens should not be changed");
            Some(diag(
                format!("Identifier empty found: {:?}", found.token),
                found,
            ))
        }
        AstError::InvalidName(invalid_token) => {
            let found = tokens
                .get(invalid_token.0)
                .expect("Tokens should not be changed");
            Some(diag(
                format!("Token not allowed in name: {:?}", found.token),
                found,
            ))
        }
        AstError::InvalidNuget(invalid_token) => {
            let found = tokens
                .get(invalid_token.0)
                .expect("Tokens should not be changed");
            Some(diag(
                format!("Token not allowed in nuget: {:?}", found.token),
                found,
            ))
        }
        AstError::InvalidBoolean(invalid_token) => {
            let found = tokens
                .get(invalid_token.0)
                .expect("Tokens should not be changed");
            Some(diag(
                format!("Token not allowed in boolean: {:?}", found.token),
                found,
            ))
        }
        AstError::InvalidString(invalid_token) => {
            let found = tokens
                .get(invalid_token.0)
                .expect("Tokens should not be changed");
            Some(diag(
                format!("Not a string literal: {:?}", found.token),
                found,
            ))
        }
        AstError::AllChildrenFailed { parent: _, errors } => {
            if let Some(e) = errors
                .iter()
                .map(|i| (get_pos(&i.1), i))
                .max_by(|a, b| a.0.0.cmp(&b.0.0))
            {
                // e.1.1.print_err(content, tokens);
                return ast_error_to_diagnostic(&e.1.1, tokens);
            }
            None
        }
        AstError::EmptyExpression(invalid_token) => {
            let found = tokens
                .get(invalid_token.0)
                .expect("Tokens should not be changed");
            Some(diag(
                format!("Invalid expression it is empty {:?}", found.token),
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
