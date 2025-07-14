//! Error type and helper
use std::panic::Location;

use smol_str::{SmolStr, format_smolstr};

use super::lexer::{PositionToken, Token};

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
    /// Invalid token in Availabilty
    InvalidAvailability(InvalidToken),
    /// Invalid token in JType
    InvalidJtype(InvalidToken),
    /// End of file reached
    UnexpectedEOF(SmolStr, u32, u32),
    /// Invalid token in Identifier
    IdentifierEmpty(InvalidToken),
    /// Invalid token in Name
    InvalidName(InvalidToken),
    /// Invalid token in Nuget
    InvalidNuget(InvalidToken),
    /// All children errored
    AllChildrenFailed {
        /// Description
        parent: SmolStr,
        /// Related errors
        errors: Vec<(SmolStr, AstError)>,
    },
    /// Invalid token in Expression
    InvalidExpression(InvalidToken),
    /// Invalid token in Double
    InvalidDouble(i64, i64),
    /// Invalid token in Boolean
    InvalidBoolean(InvalidToken),
    /// Invalid string literal
    InvalidString(InvalidToken),
}

impl PrintErr for AstError {
    fn print_err(&self, content: &str) {
        match self {
            AstError::ExpectedToken(expected_token) => {
                print_helper(
                    content,
                    expected_token.line,
                    expected_token.col,
                    format_smolstr!(
                        "Expected token {:?} found: {:?}",
                        expected_token.expected,
                        expected_token.found
                    ),
                );
            }
            AstError::InvalidAvailability(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    format_smolstr!(
                        "Invalid Availability token found: {:?} valid onese ar public, private, protected",
                        invalid_token.found
                    ),
                );
            }
            AstError::UnexpectedEOF(file, line, col) => {
                eprintln!("Unexpected end of File: {file}:{line}:{col}")
            }
            AstError::InvalidJtype(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    format_smolstr!(
                        "Invalid Type token found: {:?} valid onese ar Int, String",
                        invalid_token.found
                    ),
                );
            }
            AstError::IdentifierEmpty(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    format_smolstr!("Identifier empty found: {:?}", invalid_token.found),
                );
            }
            AstError::InvalidName(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    format_smolstr!("Token not allowed in name: {:?}", invalid_token.found),
                );
            }
            AstError::InvalidNuget(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    format_smolstr!("Token not allowed in nuget: {:?}", invalid_token.found),
                );
            }
            AstError::InvalidExpression(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    format_smolstr!("Token not allowed in expression: {:?}", invalid_token.found),
                );
            }
            AstError::InvalidDouble(a, b) => {
                eprintln!("Invalid double {a}.{b}");
            }
            AstError::InvalidBoolean(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    format_smolstr!("Token not allowed in boolean: {:?}", invalid_token.found),
                );
            }
            AstError::InvalidString(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    format_smolstr!("Not a string literal: {:?}", invalid_token.found),
                );
            }

            AstError::AllChildrenFailed { parent, errors } => {
                if false {
                    eprintln!("{}", parent);
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
        };
    }
}

#[allow(unused)]
fn sort_helper_error(a: &(SmolStr, AstError)) -> usize {
    match &a.1 {
        AstError::ExpectedToken(expected_token) => expected_token.pos,
        AstError::InvalidAvailability(invalid_token) => invalid_token.pos,
        AstError::InvalidJtype(invalid_token) => invalid_token.pos,
        AstError::UnexpectedEOF(_, _, _) => 1000,
        AstError::IdentifierEmpty(invalid_token) => invalid_token.pos,
        AstError::InvalidName(invalid_token) => invalid_token.pos,
        AstError::InvalidNuget(invalid_token) => invalid_token.pos,
        AstError::AllChildrenFailed {
            parent: _,
            errors: _,
        } => 1000,
        AstError::InvalidExpression(invalid_token) => invalid_token.pos,
        AstError::InvalidDouble(_, _) => 1000,
        AstError::InvalidBoolean(invalid_token) => invalid_token.pos,
        AstError::InvalidString(invalid_token) => invalid_token.pos,
    }
}
impl AstError {
    /// Generate eof error
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
            todo!("No errors in errors")
        }
        AstError::InvalidDouble(_, _) => todo!(),
        AstError::InvalidAvailability(invalid_token)
        | AstError::InvalidJtype(invalid_token)
        | AstError::IdentifierEmpty(invalid_token)
        | AstError::InvalidName(invalid_token)
        | AstError::InvalidNuget(invalid_token)
        | AstError::InvalidExpression(invalid_token)
        | AstError::InvalidBoolean(invalid_token)
        | AstError::InvalidString(invalid_token) => (invalid_token.pos, invalid_token.pos),
    }
}

fn print_helper(content: &str, line: usize, col: usize, msg: SmolStr) {
    let is_zero = line == 0;
    let mut lines = match is_zero {
        false => content.lines().enumerate().skip(line - 1),
        true => content.lines().enumerate().skip(line),
    };
    if !is_zero && let Some((number, line)) = lines.next() {
        eprintln!("{number} {line}");
    }
    if let Some((number, line)) = lines.next() {
        eprintln!("{number} \x1b[93m{line}\x1b[0m");
    }
    let line_digit_len: usize = line.checked_ilog10().unwrap_or(0).try_into().unwrap_or(0);
    let spaces = " ".repeat(col + line_digit_len);
    eprintln!("  {spaces}^");
    eprintln!("  {spaces}| {msg}");
    if let Some((number, line)) = lines.next() {
        eprintln!("{number}{line}");
    }
}

/// Error if token is not as expected
#[track_caller]
pub fn assert_token(
    tokens: &[PositionToken],
    pos: usize,
    expected: Token,
) -> Result<usize, AstError> {
    let t = tokens.get(pos).ok_or(AstError::eof())?;
    if t.token != expected {
        return Err(AstError::ExpectedToken(ExpectedToken::from(
            t, pos, expected,
        )));
    }
    Ok(pos + 1)
}

/// Optional semiolon
#[track_caller]
pub fn assert_semicolon(tokens: &[PositionToken], pos: usize) -> Result<usize, AstError> {
    let mut pos = pos;
    match assert_token(tokens, pos, Token::Semicolon) {
        Ok(npos) => {
            pos = npos;
        }
        Err(e) => {
            if let AstError::UnexpectedEOF(_, _, _) = e {
                return Err(e);
            }
        }
    }
    Ok(pos)
}

/// Error for expected token
#[derive(Debug, PartialEq, Clone)]
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
#[derive(Debug, PartialEq, Clone)]
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
    pub fn from(token: &PositionToken, pos: usize) -> Self {
        Self {
            found: token.token.clone(),
            pos,
            line: token.line,
            col: token.col,
        }
    }
}
