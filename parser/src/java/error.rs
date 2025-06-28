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
    UnexpectedEOF,
}
impl AstError {
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
            AstError::UnexpectedEOF => eprintln!("Unexpected end of File"),
        }
    }
}

fn print_helper(content: &str, line: usize, col: usize, msg: String) {
    let mut lines = content.lines().enumerate().skip(line - 1);
    let (number, line) = lines.next().unwrap();
    println!("{number} {line}");
    let (number, line) = lines.next().unwrap();
    println!("{number} \x1b[93m{line}\x1b[0m");
    let spaces = " ".repeat(col);
    println!("  {spaces}^");
    println!("  | {msg}");
    let (number, line) = lines.next().unwrap();
    println!("{number}{line}");
}

pub fn assert_token<'a>(
    mut iter: impl Iterator<Item = &'a PositionToken>,
    expected: Token,
) -> Result<(), AstError> {
    match iter.next() {
        Some(t) => {
            if t.token != expected {
                return Err(AstError::ExpectedToken(ExpectedToken::from(t, expected)));
            }
            return Ok(());
        }
        None => Err(AstError::UnexpectedEOF),
    }?
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
    pub line: usize,
    pub col: usize,
}

impl InvalidToken {
    pub fn from(pos: &PositionToken) -> Self {
        Self {
            found: pos.token.clone(),
            line: pos.line,
            col: pos.col,
        }
    }
}
