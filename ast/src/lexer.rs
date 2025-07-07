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
            Token::LeftParen
            | Token::RightParen
            | Token::Plus
            | Token::Dash
            | Token::Star
            | Token::Dot
            | Token::Semicolon
            | Token::LeftParenCurly
            | Token::RightParenCurly
            | Token::Comma
            | Token::Slash
            | Token::BackSlash
            | Token::At
            | Token::Lt
            | Token::Gt
            | Token::Equal
            | Token::ExclamationMark
            | Token::DoubleQuote
            | Token::SingleQuote => 1,
            Token::EqualDouble | Token::Le | Token::Ge | Token::Ne => 2,
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
    LeftParen,
    RightParen,
    Plus,
    Dash,
    Star,
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
    let chars: Vec<char> = input.chars().collect();
    let mut line = 0;
    let mut col = 0;
    let mut index = 0;

    loop {
        let ch = chars.get(index);
        let Some(ch) = ch else {
            break;
        };
        match ch {
            '\n' => {
                line += 1;
                col = 0;
                index += 1;
                continue;
            }
            ch if ch.is_whitespace() => {
                col += 1;
                index += 1;
                continue;
            }
            '(' => {
                tokens.push(PositionToken {
                    token: Token::LeftParen,
                    line,
                    col,
                });
                col += 1;
            }
            ')' => {
                tokens.push(PositionToken {
                    token: Token::RightParen,
                    line,
                    col,
                });
                col += 1;
            }
            '{' => {
                tokens.push(PositionToken {
                    token: Token::LeftParenCurly,
                    line,
                    col,
                });
                col += 1;
            }
            '}' => {
                tokens.push(PositionToken {
                    token: Token::RightParenCurly,
                    line,
                    col,
                });
                col += 1;
            }
            '+' => {
                tokens.push(PositionToken {
                    token: Token::Plus,
                    line,
                    col,
                });
                col += 1;
            }
            '-' => {
                tokens.push(PositionToken {
                    token: Token::Dash,
                    line,
                    col,
                });
                col += 1;
            }
            '*' => {
                tokens.push(PositionToken {
                    token: Token::Star,
                    line,
                    col,
                });
                col += 1;
            }
            '@' => {
                tokens.push(PositionToken {
                    token: Token::At,
                    line,
                    col,
                });
                col += 1;
            }
            '.' => {
                tokens.push(PositionToken {
                    token: Token::Dot,
                    line,
                    col,
                });
                col += 1;
            }
            ',' => {
                tokens.push(PositionToken {
                    token: Token::Comma,
                    line,
                    col,
                });
                col += 1;
            }
            ';' => {
                tokens.push(PositionToken {
                    token: Token::Semicolon,
                    line,
                    col,
                });
                col += 1;
            }
            '?' => {
                tokens.push(PositionToken {
                    token: Token::QuestionMark,
                    line,
                    col,
                });
                col += 1;
            }
            '/' => {
                let Some(peek) = chars.get(index + 1) else {
                    break;
                };
                if peek == &'/' {
                    // Inside line comment
                    loop {
                        let Some(ch) = chars.get(index) else {
                            break;
                        };
                        if ch == &'\n' {
                            line += 1;
                            col = 0;
                            break;
                        }
                        index += 1;
                    }
                } else if peek == &'*' {
                    // Inside multi line comment
                    loop {
                        let Some(ch) = chars.get(index) else {
                            break;
                        };
                        if ch == &'\n' {
                            line += 1;
                            col = 0;
                        }
                        if ch == &'*' {
                            let Some(ch) = chars.get(index + 1) else {
                                break;
                            };
                            if ch == &'/' {
                                col += 2;
                                index += 1;
                                break;
                            }
                        }
                        col += 1;
                        index += 1;
                    }
                } else {
                    tokens.push(PositionToken {
                        token: Token::Slash,
                        line,
                        col,
                    });
                    col += 1;
                }
            }
            '\\' => {
                tokens.push(PositionToken {
                    token: Token::BackSlash,
                    line,
                    col,
                });
                col += 1;
            }
            '"' => {
                tokens.push(PositionToken {
                    token: Token::DoubleQuote,
                    line,
                    col,
                });
                col += 1;
            }
            '\'' => {
                tokens.push(PositionToken {
                    token: Token::SingleQuote,
                    line,
                    col,
                });
                col += 1;
            }
            '=' => {
                if let Some('=') = chars.get(index + 1) {
                    tokens.push(PositionToken {
                        token: Token::EqualDouble,
                        line,
                        col,
                    });
                    col += 2;
                } else {
                    col += 1;
                    tokens.push(PositionToken {
                        token: Token::Equal,
                        line,
                        col,
                    });
                };
            }
            '!' => {
                if let Some('=') = chars.get(index + 1) {
                    tokens.push(PositionToken {
                        token: Token::Ne,
                        line,
                        col,
                    });
                    col += 2;
                } else {
                    tokens.push(PositionToken {
                        token: Token::ExclamationMark,
                        line,
                        col,
                    });
                    col += 1;
                };
            }
            '<' => {
                if let Some('=') = chars.get(index + 1) {
                    tokens.push(PositionToken {
                        token: Token::Le,
                        line,
                        col,
                    });
                    col += 2;
                } else {
                    tokens.push(PositionToken {
                        token: Token::Lt,
                        line,
                        col,
                    });
                    col += 1;
                };
            }
            '>' => {
                if let Some('=') = chars.get(index + 1) {
                    tokens.push(PositionToken {
                        token: Token::Ge,
                        line,
                        col,
                    });
                    col += 2;
                    index += 1;
                } else {
                    tokens.push(PositionToken {
                        token: Token::Gt,
                        line,
                        col,
                    });
                    col += 1;
                }
            }
            '0'..='9' => {
                let mut string = String::new();
                loop {
                    let Some(ch) = chars.get(index) else {
                        break;
                    };
                    if ch.is_ascii_digit() {
                        string.push(*ch);
                    } else {
                        break;
                    };
                    index += 1;
                }
                let n: i64 = string.parse().unwrap();

                tokens.push(PositionToken {
                    token: Token::Number(n),
                    line,
                    col,
                });
                col += string.len();
                continue;
            }
            'A'..='Z' | 'a'..='z' => {
                let mut ident = String::new();
                loop {
                    let Some(ch) = chars.get(index) else {
                        break;
                    };
                    if !ch.is_ascii_alphabetic() && ch != &'_' {
                        break;
                    }
                    ident.push(*ch);
                    index += 1;
                }
                let len = ident.len();
                match KEYWORDS.get(&ident) {
                    Some(t) => tokens.push(PositionToken {
                        token: t.to_owned(),
                        line,
                        col,
                    }),
                    None => tokens.push(PositionToken {
                        token: Token::Identifier(ident),
                        line,
                        col,
                    }),
                }
                col += len;
                continue;
            }
            _ => return Err(LexerError::UnknwonChar(*ch)),
        }
        index += 1;
    }

    Ok(tokens)
}

#[cfg(test)]
pub mod tests {
    use crate::lexer::{self};

    #[test]
    fn local_variable_table() {
        let content = include_str!("../../parser/test/LocalVariableTable.java");
        let tokens = lexer::lex(content).unwrap();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn supere() {
        let content = include_str!("../../parser/test/Super.java");
        let tokens = lexer::lex(content).unwrap();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn super_interface() {
        let content = include_str!("../../parser/test/SuperInterface.java");
        let tokens = lexer::lex(content).unwrap();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn everything() {
        let content = include_str!("../../parser/test/Everything.java");
        let tokens = lexer::lex(content).unwrap();
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn thrower() {
        let content = include_str!("../../parser/test/Thrower.java");
        let tokens = lexer::lex(content).unwrap();
        insta::assert_debug_snapshot!(tokens);
    }
}
