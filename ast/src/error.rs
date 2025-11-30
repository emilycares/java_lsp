//! Error type and helper
use my_string::MyString;

use super::lexer::{PositionToken, Token};
use crate::BlockEntryOptions;
use std::panic::Location;

const PRINT_ALL_ERRORS: bool = false;

/// Fancy log ast error
pub trait PrintErr {
    /// impl
    fn print_err(&self, content: &str);
}

impl<T> PrintErr for Result<T, AstError> {
    fn print_err(&self, content: &str) {
        match self {
            Ok(_) => (),
            Err(e) => e.print_err(content),
        }
    }
}

/// All ast errors
#[derive(Debug, PartialEq, Clone)]
pub enum AstError {
    /// Other token found than expected
    ExpectedToken(ExpectedToken),
    /// Invalid token in `JType`
    InvalidJtype(InvalidToken),
    /// Expression was empty
    EmptyExpression(InvalidToken),
    /// End of file reached
    UnexpectedEOF(MyString, u32, u32),
    /// Invalid token in Identifier
    IdentifierEmpty(InvalidToken),
    /// Invalid token in Name
    InvalidName(InvalidToken),
    /// Invalid token in Nuget
    InvalidNuget(InvalidToken),
    /// All children errored
    AllChildrenFailed {
        /// Description
        parent: MyString,
        /// Related errors
        errors: Vec<(MyString, AstError)>,
    },
    /// Invalid token in Boolean
    InvalidBoolean(InvalidToken),
    /// Invalid string literal
    InvalidString(InvalidToken),
}

impl PrintErr for AstError {
    fn print_err(&self, content: &str) {
        match self {
            Self::ExpectedToken(expected_token) => {
                print_helper(
                    content,
                    expected_token.line,
                    expected_token.col,
                    &format!(
                        "Expected token {:?} found: {:?}",
                        expected_token.expected, expected_token.found
                    ),
                );
            }
            Self::UnexpectedEOF(file, line, col) => {
                eprintln!("Unexpected end of File: {file}:{line}:{col}");
            }
            Self::InvalidJtype(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    &format!(
                        "Invalid Type token found: {:?} valid onese ar Int, String",
                        invalid_token.found
                    ),
                );
            }
            Self::IdentifierEmpty(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    &format!("Identifier empty found: {:?}", invalid_token.found),
                );
            }
            Self::InvalidName(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    &format!("Token not allowed in name: {:?}", invalid_token.found),
                );
            }
            Self::InvalidNuget(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    &format!("Token not allowed in nuget: {:?}", invalid_token.found),
                );
            }
            Self::InvalidBoolean(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    &format!("Token not allowed in boolean: {:?}", invalid_token.found),
                );
            }
            Self::InvalidString(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    &format!("Not a string literal: {:?}", invalid_token.found),
                );
            }
            Self::AllChildrenFailed { parent, errors } => {
                if PRINT_ALL_ERRORS {
                    eprintln!("{parent}");
                    for e in errors {
                        eprintln!("{}", e.0);
                        e.1.print_err(content);
                    }
                } else if let Some(e) = errors
                    .iter()
                    .map(|i| (get_pos(&i.1), i))
                    .max_by(|a, b| a.0.0.cmp(&b.0.0))
                {
                    e.1.1.print_err(content);
                }
            }
            Self::EmptyExpression(invalid_token) => print_helper(
                content,
                invalid_token.line,
                invalid_token.col,
                &format!(
                    "Invalid Type token found: {:?} valid onese ar Int, String",
                    invalid_token.found
                ),
            ),
        }
    }
}

#[allow(unused)]
const fn sort_helper_error(a: &(MyString, AstError)) -> usize {
    match &a.1 {
        AstError::ExpectedToken(expected_token) => expected_token.pos,
        AstError::InvalidJtype(invalid_token)
        | AstError::EmptyExpression(invalid_token)
        | AstError::IdentifierEmpty(invalid_token)
        | AstError::InvalidName(invalid_token)
        | AstError::InvalidNuget(invalid_token)
        | AstError::InvalidBoolean(invalid_token)
        | AstError::InvalidString(invalid_token) => invalid_token.pos,
        AstError::UnexpectedEOF(_, _, _)
        | AstError::AllChildrenFailed {
            parent: _,
            errors: _,
        } => 1000,
    }
}
impl AstError {
    /// Generate eof error
    #[must_use]
    #[track_caller]
    pub fn eof() -> Self {
        let loc = Location::caller();
        Self::UnexpectedEOF(loc.file().into(), loc.line(), loc.column())
    }
}
fn get_pos(e: &AstError) -> (usize, usize) {
    match e {
        AstError::ExpectedToken(expected_token) => (expected_token.pos, expected_token.pos),
        AstError::UnexpectedEOF(_, _, _) => (10_000_000, 10_000_000),
        AstError::AllChildrenFailed { parent: _, errors } => {
            let poses = errors.iter().map(|i| &i.1).map(get_pos);
            if let Some(min) = poses.clone().min()
                && let Some(max) = poses.max()
            {
                return (min.0, max.1);
            }
            (0, 0)
        }
        AstError::InvalidJtype(invalid_token)
        | AstError::EmptyExpression(invalid_token)
        | AstError::IdentifierEmpty(invalid_token)
        | AstError::InvalidName(invalid_token)
        | AstError::InvalidNuget(invalid_token)
        | AstError::InvalidBoolean(invalid_token)
        | AstError::InvalidString(invalid_token) => (invalid_token.pos, invalid_token.pos),
    }
}

fn print_helper(content: &str, line: usize, col: usize, msg: &str) {
    let is_zero = line == 0;
    let mut lines = if is_zero {
        content.lines().enumerate().skip(line)
    } else {
        content.lines().enumerate().skip(line - 1)
    };
    if !is_zero && let Some((number, line)) = lines.next() {
        let number = number + 1;
        eprintln!("{number} {line}");
    }
    if let Some((number, line)) = lines.next() {
        let number = number + 1;
        eprintln!("{number} \x1b[93m{line}\x1b[0m");
    }
    let line_digit_len: usize = line.checked_ilog10().unwrap_or(0).try_into().unwrap_or(0);
    let spaces = " ".repeat(col + line_digit_len);
    eprintln!("  {spaces}^");
    eprintln!("  {spaces}| {msg}");
    if let Some((number, line)) = lines.next() {
        let number = number + 1;
        eprintln!("{number} {line}");
    }
}

/// Error if token is not as expected
///
/// # Errors
/// When token does not match expected
#[track_caller]
pub fn assert_token(
    tokens: &[PositionToken],
    pos: usize,
    expected: Token,
) -> Result<usize, AstError> {
    if pos > tokens.len() {
        return Err(AstError::eof());
    }
    let t = tokens.get(pos).ok_or_else(AstError::eof)?;
    if t.token != expected {
        return Err(AstError::ExpectedToken(ExpectedToken::from(
            t, pos, expected,
        )));
    }
    Ok(pos + 1)
}

/// Optional semiolon
#[track_caller]
pub fn assert_semicolon_options(
    tokens: &[PositionToken],
    pos: usize,
    block_entry_options: &BlockEntryOptions,
) -> Result<usize, AstError> {
    if block_entry_options == &BlockEntryOptions::NoSemicolon {
        return Ok(pos);
    }
    assert_semicolon(tokens, pos)
}
/// Optional multiple semiolon
#[track_caller]
pub fn assert_semicolon(tokens: &[PositionToken], pos: usize) -> Result<usize, AstError> {
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::Semicolon) {
        pos = npos;
    }
    Ok(pos)
}

/// Error for expected token
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ExpectedToken {
    /// The expected token
    pub expected: Token,
    /// But found
    pub found: Token,
    /// At pos
    pub pos: usize,
    /// In line
    pub line: usize,
    /// In column
    pub col: usize,
}

impl ExpectedToken {
    /// constructor
    #[must_use]
    pub fn from(position_token: &PositionToken, pos: usize, expected: Token) -> Self {
        Self {
            expected,
            pos,
            found: position_token.token.clone(),
            line: position_token.line,
            col: position_token.col,
        }
    }
}

/// Token is invalid
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InvalidToken {
    /// But was
    pub found: Token,
    /// At pos
    pub pos: usize,
    /// In line
    pub line: usize,
    /// In column
    pub col: usize,
}

impl InvalidToken {
    /// constructor
    #[must_use]
    pub fn from(token: &PositionToken, pos: usize) -> Self {
        Self {
            found: token.token.clone(),
            pos,
            line: token.line,
            col: token.col,
        }
    }
}

/// Get Start and End `PositionToken`
pub trait GetStartEnd {
    /// Get start `PositionToken`
    #[track_caller]
    fn start(&self, pos: usize) -> Result<&PositionToken, AstError>;
    /// Get end `PositionToken`
    #[track_caller]
    fn end(&self, pos: usize) -> Result<&PositionToken, AstError>;
}

impl GetStartEnd for [PositionToken] {
    fn start(&self, pos: usize) -> Result<&PositionToken, AstError> {
        self.get(pos).ok_or_else(AstError::eof)
    }
    fn end(&self, pos: usize) -> Result<&PositionToken, AstError> {
        self.get(pos.saturating_sub(1)).ok_or_else(AstError::eof)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn config() {
        assert!(!PRINT_ALL_ERRORS);
    }
}
