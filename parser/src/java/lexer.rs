use std::iter::{self, from_fn};

use phf::phf_map;

#[derive(Debug, PartialEq, Clone)]
pub struct PositionToken {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Identifier(String),
    Number(i64),
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
};

pub fn lex(input: &str) -> Result<Vec<PositionToken>, LexerError> {
    let mut tokens = Vec::new();
    let mut iter = input.chars().peekable();
    let mut line = 0;
    let mut char = 0;

    while let Some(ch) = iter.next() {
        match ch {
            '\n' | '\r' => {
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
    use pretty_assertions::assert_eq;

    use crate::java::lexer::{self, Token};

    #[test]
    fn local_variable_table() {
        let content = include_str!("../../test/LocalVariableTable.java");
        let tokens = lexer::lex(content).unwrap();
        let tokens: Vec<Token> = tokens.iter().map(|i| i.token.clone()).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Package,
                Token::Identifier("ch".to_string()),
                Token::Dot,
                Token::Identifier("emilycares".to_string()),
                Token::Semicolon,
                Token::Import,
                Token::Identifier("java".to_string()),
                Token::Dot,
                Token::Identifier("util".to_string()),
                Token::Dot,
                Token::Star,
                Token::Semicolon,
                Token::Public,
                Token::Class,
                Token::Identifier("LocalVariableTable".to_string()),
                Token::LeftParenCurly,
                Token::Private,
                Token::Identifier("HashSet".to_string()),
                Token::Lt,
                Token::Identifier("String".to_string()),
                Token::Gt,
                Token::Identifier("a".to_string()),
                Token::Equal,
                Token::New,
                Token::Identifier("HashSet".to_string()),
                Token::Lt,
                Token::Gt,
                Token::LeftParen,
                Token::RightParen,
                Token::Semicolon,
                Token::Public,
                Token::Void,
                Token::Identifier("hereIsCode".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::LeftParenCurly,
                Token::Identifier("HashMap".to_string()),
                Token::Lt,
                Token::Identifier("Integer".to_string()),
                Token::Comma,
                Token::Identifier("String".to_string()),
                Token::Gt,
                Token::Identifier("a".to_string()),
                Token::Equal,
                Token::New,
                Token::Identifier("HashMap".to_string()),
                Token::Lt,
                Token::Gt,
                Token::LeftParen,
                Token::RightParen,
                Token::Semicolon,
                Token::Identifier("a".to_string()),
                Token::Dot,
                Token::Identifier("put".to_string()),
                Token::LeftParen,
                Token::Number(1,),
                Token::Comma,
                Token::DoubleQuote,
                Token::DoubleQuote,
                Token::RightParen,
                Token::Semicolon,
                Token::RightParenCurly,
                Token::Public,
                Token::Int,
                Token::Identifier("hereIsCode".to_string()),
                Token::LeftParen,
                Token::Int,
                Token::Identifier("a".to_string()),
                Token::Comma,
                Token::Int,
                Token::Identifier("b".to_string()),
                Token::RightParen,
                Token::LeftParenCurly,
                Token::Int,
                Token::Identifier("o".to_string()),
                Token::Equal,
                Token::Identifier("a".to_string()),
                Token::Plus,
                Token::Identifier("b".to_string()),
                Token::Semicolon,
                Token::Return,
                Token::Identifier("o".to_string()),
                Token::Dash,
                Token::Number(1,),
                Token::Semicolon,
                Token::RightParenCurly,
                Token::RightParenCurly,
                Token::EOF,
            ]
        )
    }

    #[test]
    fn super_interface() {
        let content = include_str!("../../test/SuperInterface.java");
        let tokens = lexer::lex(content).unwrap();
        let tokens: Vec<Token> = tokens.iter().map(|i| i.token.clone()).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Package,
                Token::Identifier("ch".to_string()),
                Token::Dot,
                Token::Identifier("emilycares".to_string()),
                Token::Semicolon,
                Token::Import,
                Token::Identifier("java".to_string()),
                Token::Dot,
                Token::Identifier("util".to_string()),
                Token::Dot,
                Token::Identifier("Collection".to_string()),
                Token::Semicolon,
                Token::Import,
                Token::Identifier("java".to_string()),
                Token::Dot,
                Token::Identifier("util".to_string()),
                Token::Dot,
                Token::Identifier("List".to_string()),
                Token::Semicolon,
                Token::Import,
                Token::Identifier("java".to_string()),
                Token::Dot,
                Token::Identifier("util".to_string()),
                Token::Dot,
                Token::Identifier("stream".to_string()),
                Token::Dot,
                Token::Identifier("Stream".to_string()),
                Token::Semicolon,
                Token::Import,
                Token::Identifier("java".to_string()),
                Token::Dot,
                Token::Identifier("util".to_string()),
                Token::Dot,
                Token::Identifier("stream".to_string()),
                Token::Dot,
                Token::Identifier("StreamSupport".to_string()),
                Token::Semicolon,
                Token::Public,
                Token::Interface,
                Token::Identifier("SuperInterface".to_string()),
                Token::Lt,
                Token::Identifier("E".to_string()),
                Token::Gt,
                Token::Extends,
                Token::Identifier("Collection".to_string()),
                Token::Comma,
                Token::Identifier("List".to_string()),
                Token::LeftParenCurly,
                Token::Identifier("default".to_string()),
                Token::Identifier("Stream".to_string()),
                Token::Lt,
                Token::Identifier("E".to_string()),
                Token::Gt,
                Token::Identifier("stream".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::LeftParenCurly,
                Token::Return,
                Token::Identifier("StreamSupport".to_string()),
                Token::Dot,
                Token::Identifier("stream".to_string()),
                Token::LeftParen,
                Token::Identifier("spliterator".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::Comma,
                Token::False,
                Token::RightParen,
                Token::Semicolon,
                Token::RightParenCurly,
                Token::RightParenCurly,
                Token::EOF,
            ]
        )
    }

    #[test]
    fn everything() {
        let content = include_str!("../../test/Everything.java");
        let tokens = lexer::lex(content).unwrap();
        let tokens: Vec<Token> = tokens.iter().map(|i| i.token.clone()).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Package,
                Token::Identifier("ch".to_string()),
                Token::Dot,
                Token::Identifier("emilycares".to_string()),
                Token::Semicolon,
                Token::Public,
                Token::Class,
                Token::Identifier("Everything".to_string()),
                Token::LeftParenCurly,
                Token::Public,
                Token::Identifier("Everything".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::LeftParenCurly,
                Token::RightParenCurly,
                Token::Int,
                Token::Identifier("noprop".to_string()),
                Token::Semicolon,
                Token::Public,
                Token::Int,
                Token::Identifier("publicproperty".to_string()),
                Token::Semicolon,
                Token::Private,
                Token::Int,
                Token::Identifier("privateproperty".to_string()),
                Token::Semicolon,
                Token::Void,
                Token::Identifier("method".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::LeftParenCurly,
                Token::RightParenCurly,
                Token::Public,
                Token::Void,
                Token::Identifier("public_method".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::LeftParenCurly,
                Token::RightParenCurly,
                Token::Private,
                Token::Void,
                Token::Identifier("private_method".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::LeftParenCurly,
                Token::RightParenCurly,
                Token::Int,
                Token::Identifier("out".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::LeftParenCurly,
                Token::Return,
                Token::Number(0,),
                Token::Semicolon,
                Token::RightParenCurly,
                Token::Slash,
                Token::Star,
                Token::Star,
                Token::Star,
                Token::Identifier("Documentation".to_string()),
                Token::Star,
                Token::At,
                Token::Identifier("param".to_string()),
                Token::Identifier("a".to_string()),
                Token::Star,
                Token::At,
                Token::Identifier("param".to_string()),
                Token::Identifier("b".to_string()),
                Token::Star,
                Token::At,
                Token::Return,
                Token::Star,
                Token::Slash,
                Token::Int,
                Token::Identifier("add".to_string()),
                Token::LeftParen,
                Token::Int,
                Token::Identifier("a".to_string()),
                Token::Comma,
                Token::Int,
                Token::Identifier("b".to_string()),
                Token::RightParen,
                Token::LeftParenCurly,
                Token::Return,
                Token::Identifier("a".to_string()),
                Token::Plus,
                Token::Identifier("b".to_string()),
                Token::Semicolon,
                Token::RightParenCurly,
                Token::Identifier("static".to_string()),
                Token::Int,
                Token::Identifier("sadd".to_string()),
                Token::LeftParen,
                Token::Int,
                Token::Identifier("a".to_string()),
                Token::Comma,
                Token::Int,
                Token::Identifier("b".to_string()),
                Token::RightParen,
                Token::LeftParenCurly,
                Token::Return,
                Token::Identifier("a".to_string()),
                Token::Plus,
                Token::Identifier("b".to_string()),
                Token::Semicolon,
                Token::RightParenCurly,
                Token::RightParenCurly,
                Token::EOF,
            ]
        )
    }

    #[test]
    fn thrower() {
        let content = include_str!("../../test/Thrower.java");
        let tokens = lexer::lex(content).unwrap();
        let tokens: Vec<Token> = tokens.iter().map(|i| i.token.clone()).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Package,
                Token::Identifier("ch".to_string()),
                Token::Dot,
                Token::Identifier("emilycares".to_string()),
                Token::Semicolon,
                Token::Import,
                Token::Identifier("java".to_string()),
                Token::Dot,
                Token::Identifier("io".to_string()),
                Token::Dot,
                Token::Identifier("IOException".to_string()),
                Token::Semicolon,
                Token::Public,
                Token::Class,
                Token::Identifier("Thrower".to_string()),
                Token::LeftParenCurly,
                Token::Public,
                Token::Void,
                Token::Identifier("ioThrower".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::Throws,
                Token::Identifier("IOException".to_string()),
                Token::LeftParenCurly,
                Token::RightParenCurly,
                Token::Public,
                Token::Void,
                Token::Identifier("ioThrower".to_string()),
                Token::LeftParen,
                Token::Int,
                Token::Identifier("a".to_string()),
                Token::RightParen,
                Token::Throws,
                Token::Identifier("IOException".to_string()),
                Token::Comma,
                Token::Identifier("IOException".to_string()),
                Token::LeftParenCurly,
                Token::RightParenCurly,
                Token::RightParenCurly,
                Token::EOF,
            ]
        )
    }
}
