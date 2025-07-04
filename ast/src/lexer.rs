use std::iter::{self, from_fn};

use phf::phf_map;

use crate::types::AstPoint;

#[derive(Debug, PartialEq, Clone)]
pub struct PositionToken {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}
impl PositionToken {
    pub fn start_point(&self) -> AstPoint {
        AstPoint {
            line: self.line,
            col: self.col,
        }
    }
    pub fn end_point(&self) -> AstPoint {
        AstPoint {
            line: self.line,
            col: self.col + self.token.len(),
        }
    }
}

impl Token {
    pub fn len(&self) -> usize {
        match self {
            Token::Identifier(i) => i.len(),
            Token::Number(n) => n.to_string().len(),
            Token::NewLine => 1,
            Token::LeftParen => 1,
            Token::RightParen => 1,
            Token::Plus => 1,
            Token::Dash => 1,
            Token::Star => 1,
            Token::EOF => 0,
            Token::Dot => 1,
            Token::Semicolon => 1,
            Token::LeftParenCurly => 1,
            Token::RightParenCurly => 1,
            Token::Comma => 1,
            Token::Slash => 1,
            Token::BackSlash => 1,
            Token::At => 1,
            Token::Le => 2,
            Token::Lt => 1,
            Token::Ge => 2,
            Token::Gt => 1,
            Token::EqualDouble => 2,
            Token::Equal => 1,
            Token::Ne => 2,
            Token::ExclamationMark => 1,
            Token::DoubleQuote => 1,
            Token::SingleQuote => 1,
            Token::While
            | Token::Package
            | Token::Import
            | Token::Public
            | Token::Private
            | Token::Protected
            | Token::Class
            | Token::Interface
            | Token::Enum
            | Token::Void
            | Token::Throws
            | Token::Int
            | Token::Double
            | Token::Extends
            | Token::Implements
            | Token::True
            | Token::False
            | Token::Float
            | Token::New
            | Token::Return
            | Token::QuestionMark
            | Token::Char
            | Token::Boolean
            | Token::Byte
            | Token::Short
            | Token::Long
            | Token::Static
            | Token::If => KEYWORDS
                .entries()
                .find(|i| i.1 == self)
                .map(|i| i.0.len())
                .unwrap_or(0),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Identifier(String),
    Number(i64),
    NewLine,
    LeftParen,
    RightParen,
    Plus,
    Dash,
    Star,
    EOF,
    Dot,
    Semicolon,
    LeftParenCurly,
    RightParenCurly,
    Comma,
    If,
    While,
    Package,
    Import,
    Public,
    Private,
    Protected,
    Class,
    Interface,
    Enum,
    Void,
    Throws,
    Int,
    Double,
    Float,
    Slash,
    BackSlash,
    At,
    Le,
    Lt,
    Ge,
    Gt,
    Extends,
    Implements,
    True,
    False,
    EqualDouble,
    Equal,
    Ne,
    ExclamationMark,
    DoubleQuote,
    SingleQuote,
    New,
    Return,
    QuestionMark,
    Char,
    Boolean,
    Byte,
    Short,
    Long,
    Static,
}

#[derive(Debug, PartialEq)]
pub enum LexerError {
    UnknwonChar(char),
}

static KEYWORDS: phf::Map<&'static str, Token> = phf_map! {
    "if" => Token::If,
    "true" => Token::True,
    "false" => Token::False,
    "while" => Token::While,
    "package" => Token::Package,
    "import" => Token::Import,
    "public" => Token::Public,
    "private" => Token::Private,
    "protected" => Token::Protected,
    "class" => Token::Class,
    "interface" => Token::Interface,
    "enum" => Token::Enum,
    "void" => Token::Void,
    "throws" => Token::Throws,
    "int" => Token::Int,
    "double" => Token::Double,
    "float" => Token::Float,
    "extends" => Token::Extends,
    "implements" => Token::Implements,
    "new" => Token::New,
    "return" => Token::Return,
    "char" => Token::Char,
    "boolean" => Token::Boolean,
    "byte" => Token::Byte,
    "short" => Token::Short,
    "long" => Token::Long,
    "static" => Token::Static,
};

pub fn lex(input: &str) -> Result<Vec<PositionToken>, LexerError> {
    let mut tokens = Vec::new();
    let mut iter = input.chars().peekable();
    let mut line = 0;
    let mut char = 0;

    while let Some(ch) = iter.next() {
        match ch {
            '\n' => {
                tokens.push(PositionToken {
                    token: Token::NewLine,
                    line,
                    col: char,
                });
                line += 1;
                char = 0;
            }
            ch if ch.is_whitespace() => {
                char += 1;
                continue;
            }
            '(' => {
                tokens.push(PositionToken {
                    token: Token::LeftParen,
                    line,
                    col: char,
                });
                char += 1;
            }
            ')' => {
                tokens.push(PositionToken {
                    token: Token::RightParen,
                    line,
                    col: char,
                });
                char += 1;
            }
            '{' => {
                tokens.push(PositionToken {
                    token: Token::LeftParenCurly,
                    line,
                    col: char,
                });
                char += 1;
            }
            '}' => {
                tokens.push(PositionToken {
                    token: Token::RightParenCurly,
                    line,
                    col: char,
                });
                char += 1;
            }
            '+' => {
                tokens.push(PositionToken {
                    token: Token::Plus,
                    line,
                    col: char,
                });
                char += 1;
            }
            '-' => {
                tokens.push(PositionToken {
                    token: Token::Dash,
                    line,
                    col: char,
                });
                char += 1;
            }
            '*' => {
                tokens.push(PositionToken {
                    token: Token::Star,
                    line,
                    col: char,
                });
                char += 1;
            }
            '@' => {
                tokens.push(PositionToken {
                    token: Token::At,
                    line,
                    col: char,
                });
                char += 1;
            }
            '.' => {
                tokens.push(PositionToken {
                    token: Token::Dot,
                    line,
                    col: char,
                });
                char += 1;
            }
            ',' => {
                tokens.push(PositionToken {
                    token: Token::Comma,
                    line,
                    col: char,
                });
                char += 1;
            }
            ';' => {
                tokens.push(PositionToken {
                    token: Token::Semicolon,
                    line,
                    col: char,
                });
                char += 1;
            }
            '?' => {
                tokens.push(PositionToken {
                    token: Token::QuestionMark,
                    line,
                    col: char,
                });
                char += 1;
            }
            '/' => {
                tokens.push(PositionToken {
                    token: Token::Slash,
                    line,
                    col: char,
                });
                char += 1;
            }
            '\\' => {
                tokens.push(PositionToken {
                    token: Token::BackSlash,
                    line,
                    col: char,
                });
                char += 1;
            }
            '"' => tokens.push(PositionToken {
                token: Token::DoubleQuote,
                line,
                col: char,
            }),
            '\'' => {
                tokens.push(PositionToken {
                    token: Token::SingleQuote,
                    line,
                    col: char,
                });
                char += 1;
            }
            '=' => {
                if let Some('=') = iter.peek() {
                    tokens.push(PositionToken {
                        token: Token::EqualDouble,
                        line,
                        col: char,
                    });
                    iter.next();
                    char += 2;
                } else {
                    char += 1;
                    tokens.push(PositionToken {
                        token: Token::Equal,
                        line,
                        col: char,
                    });
                };
            }
            '!' => {
                if let Some('=') = iter.peek() {
                    tokens.push(PositionToken {
                        token: Token::Ne,
                        line,
                        col: char,
                    });
                    char += 2;
                    iter.next();
                } else {
                    tokens.push(PositionToken {
                        token: Token::ExclamationMark,
                        line,
                        col: char,
                    });
                    char += 1;
                };
            }
            '<' => {
                if let Some('=') = iter.peek() {
                    tokens.push(PositionToken {
                        token: Token::Le,
                        line,
                        col: char,
                    });
                    char += 2;
                    iter.next();
                } else {
                    tokens.push(PositionToken {
                        token: Token::Lt,
                        line,
                        col: char,
                    });
                    char += 1;
                };
            }
            '>' => {
                if let Some('=') = iter.peek() {
                    tokens.push(PositionToken {
                        token: Token::Ge,
                        line,
                        col: char,
                    });
                    char += 2;
                    iter.next();
                } else {
                    tokens.push(PositionToken {
                        token: Token::Gt,
                        line,
                        col: char,
                    });
                    char += 1;
                }
            }
            '0'..='9' => {
                let string = iter::once(ch)
                    .chain(from_fn(|| iter.by_ref().next_if(|s| s.is_ascii_digit())))
                    .collect::<String>();
                let n: i64 = string.parse().unwrap();

                tokens.push(PositionToken {
                    token: Token::Number(n),
                    line,
                    col: char,
                });
                char += string.len();
            }

            'A'..='Z' | 'a'..='z' => {
                let ident = iter::once(ch)
                    .chain(from_fn(|| {
                        iter.by_ref()
                            .next_if(|s| s.is_ascii_alphabetic() || s == &'_')
                    }))
                    .collect::<String>();
                let len = ident.len();
                match KEYWORDS.get(&ident) {
                    Some(t) => tokens.push(PositionToken {
                        token: t.to_owned(),
                        line,
                        col: char,
                    }),
                    None => tokens.push(PositionToken {
                        token: Token::Identifier(ident),
                        line,
                        col: char,
                    }),
                }
                char += len;
            }
            _ => return Err(LexerError::UnknwonChar(ch)),
        }
    }

    tokens.push(PositionToken {
        token: Token::EOF,
        line,
        col: char,
    });
    Ok(tokens)
}

#[cfg(test)]
pub mod tests {
    use crate::lexer::{self, Token};

    #[test]
    fn local_variable_table() {
        let content = include_str!("../../parser/test/LocalVariableTable.java");
        let tokens = lexer::lex(content).unwrap();
        let tokens: Vec<Token> = tokens.iter().map(|i| i.token.clone()).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn supere() {
        let content = include_str!("../../parser/test/Super.java");
        let tokens = lexer::lex(content).unwrap();
        let tokens: Vec<Token> = tokens.iter().map(|i| i.token.clone()).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn super_interface() {
        let content = include_str!("../../parser/test/SuperInterface.java");
        let tokens = lexer::lex(content).unwrap();
        let tokens: Vec<Token> = tokens.iter().map(|i| i.token.clone()).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn everything() {
        let content = include_str!("../../parser/test/Everything.java");
        let tokens = lexer::lex(content).unwrap();
        let tokens: Vec<Token> = tokens.iter().map(|i| i.token.clone()).collect();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn thrower() {
        let content = include_str!("../../parser/test/Thrower.java");
        let tokens = lexer::lex(content).unwrap();
        let tokens: Vec<Token> = tokens.iter().map(|i| i.token.clone()).collect();
        insta::assert_debug_snapshot!(tokens);
    }
}
