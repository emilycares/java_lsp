use std::panic::Location;

use super::lexer::{PositionToken, Token};

pub trait PrintErr {
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

#[derive(Debug, PartialEq)]
pub enum AstError {
    ExpectedToken(ExpectedToken),
    InvalidAvailability(InvalidToken),
    InvalidJtype(InvalidToken),
    UnexpectedEOF(String, u32, u32),
    IdentifierEmpty(InvalidToken),
    InvalidName(InvalidToken),
    InvalidNuget(InvalidToken),
    AllChildrenFailed {
        parent: String,
        errors: Vec<(String, AstError)>,
    },
    InvalidExpression(InvalidToken),
    InvalidDouble(i64, i64),
}
impl AstError {
    #[track_caller]
    pub fn eof() -> Self {
        let loc = Location::caller();
        Self::UnexpectedEOF(loc.file().to_string(), loc.line(), loc.column())
    }
    pub fn print_err(&self, content: &str) {
        match self {
            AstError::ExpectedToken(expected_token) => {
                print_helper(
                    content,
                    expected_token.line,
                    expected_token.col,
                    format!(
                        "Expected token {:?} found: {:?}",
                        expected_token.expected, expected_token.found
                    ),
                );
            }
            AstError::InvalidAvailability(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    format!(
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
                    format!(
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
                    format!("Identifier empty found: {:?}", invalid_token.found),
                );
            }
            AstError::InvalidName(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    format!("Token not allowed in name: {:?}", invalid_token.found),
                );
            }
            AstError::InvalidNuget(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    format!("Token not allowed in nuget: {:?}", invalid_token.found),
                );
            }
            AstError::AllChildrenFailed { parent, errors } => {
                eprintln!("All chilren failed in {parent}");
                for (child, e) in errors {
                    eprintln!(" {child} --------------------------");
                    e.print_err(content);
                }
            }
            AstError::InvalidExpression(invalid_token) => {
                print_helper(
                    content,
                    invalid_token.line,
                    invalid_token.col,
                    format!("Token not allowed in expression: {:?}", invalid_token.found),
                );
            }
            AstError::InvalidDouble(a, b) => {
                eprintln!("Invalid double {a}.{b}");
            }
        }
    }
}

fn print_helper(content: &str, line: usize, col: usize, msg: String) {
    let is_zero = line == 0;
    let mut lines = match is_zero {
        false => content.lines().enumerate().skip(line - 1),
        true => content.lines().enumerate().skip(line),
    };
    if !is_zero {
        if let Some((number, line)) = lines.next() {
            eprintln!("{number} {line}");
        }
    }
    if let Some((number, line)) = lines.next() {
        eprintln!("{number} \x1b[93m{line}\x1b[0m");
    }
    let spaces = " ".repeat(col + 1);
    eprintln!("  {spaces}^");
    eprintln!("  {spaces}| {msg}");
    if let Some((number, line)) = lines.next() {
        eprintln!("{number}{line}");
    }
}

pub fn assert_token(
    tokens: &[PositionToken],
    pos: usize,
    expected: Token,
) -> Result<usize, AstError> {
    match tokens.get(pos) {
        Some(t) => {
            if t.token != expected {
                return Err(AstError::ExpectedToken(ExpectedToken::from(t, expected)));
            }
            return Ok(pos + 1);
        }
        None => Err(AstError::eof()),
    }?
}
pub fn assert_semicolon(tokens: &[PositionToken], pos: usize) -> usize {
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Semicolon) {
        pos = npos;
    }
    pos
}
#[derive(Debug, PartialEq)]
pub struct ExpectedToken {
    pub expected: Token,
    pub found: Token,
    pub line: usize,
    pub col: usize,
}

impl ExpectedToken {
    pub fn from(pos: &PositionToken, expected: Token) -> Self {
        Self {
            expected,
            found: pos.token.clone(),
            line: pos.line,
            col: pos.col,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct InvalidToken {
    pub found: Token,
    pub pos: usize,
    pub line: usize,
    pub col: usize,
}

impl InvalidToken {
    pub fn from(token: &PositionToken, pos: usize) -> Self {
        Self {
            found: token.token.clone(),
            pos,
            line: token.line,
            col: token.col,
        }
    }
}
