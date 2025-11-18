//! Transform document into a token vector
use my_string::MyString;
use phf::phf_map;

use crate::types::AstPoint;

/// Position in document
#[derive(Debug, PartialEq, Clone)]
pub struct PositionToken {
    /// Data
    pub token: Token,
    /// line in file
    pub line: usize,
    /// column in file
    pub col: usize,
}
impl PositionToken {
    /// Start point of Token
    pub fn start_point(&self) -> AstPoint {
        AstPoint {
            line: self.line,
            col: self.col,
        }
    }
    /// End point of Token
    pub fn end_point(&self) -> AstPoint {
        AstPoint {
            line: self.line,
            col: self.col + self.token.len(),
        }
    }
}

impl Token {
    /// Length of token
    pub fn len(&self) -> usize {
        match self {
            Token::Identifier(i) => i.len(),
            Token::StringLiteral(i) => i.len(),
            Token::CharLiteral(i) => i.len(),
            Token::Number(n) => n.to_string().len(),
            Token::LeftParen
            | Token::RightParen
            | Token::Plus
            | Token::Dash
            | Token::Star
            | Token::Dot
            | Token::Colon
            | Token::Semicolon
            | Token::Percent
            | Token::Ampersand
            | Token::VerticalBar
            | Token::LeftParenCurly
            | Token::RightParenCurly
            | Self::LeftParenSquare
            | Self::RightParenSquare
            | Token::Comma
            | Token::Slash
            | Token::BackSlash
            | Token::At
            | Token::Lt
            | Token::Gt
            | Token::Equal
            | Token::ExclamationMark
            | Token::Underscore
            | Token::Caret
            | Token::Tilde
            | Token::SingleQuote => 1,
            Token::EqualDouble | Token::Le | Token::Ge | Token::Ne | Token::Arrow => 2,
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
            | Token::Final
            | Token::Default
            | Token::Else
            | Token::For
            | Token::Break
            | Token::Continue
            | Token::Switch
            | Token::Case
            | Token::Do
            | Token::Try
            | Token::Catch
            | Token::Finally
            | Token::Throw
            | Token::Yield
            | Token::Var
            | Token::This
            | Token::Abstract
            | Token::Record
            | Token::Synchronized
            | Token::InstanceOf
            | Token::Volatile
            | Token::Transient
            | Token::Native
            | Token::Sealed
            | Token::Non
            | Token::Permits
            | Token::If => KEYWORDS
                .entries()
                .find(|i| i.1 == self)
                .map(|i| i.0.len())
                .unwrap_or(0),
        }
    }

    /// string version of token
    pub fn to_string(&self) -> MyString {
        match self {
            Token::Identifier(smol_str) => smol_str.clone(),
            Token::StringLiteral(smol_str) => smol_str.clone(),
            Token::CharLiteral(smol_str) => smol_str.clone(),
            Token::Number(num) => num.to_string().into(),
            Token::LeftParen => MyString::from("("),
            Token::RightParen => MyString::from(")"),
            Token::Plus => MyString::from("+"),
            Token::Dash => MyString::from("-"),
            Token::Star => MyString::from("*"),
            Token::Dot => MyString::from("."),
            Token::Semicolon => MyString::from(";"),
            Token::Colon => MyString::from(":"),
            Token::Percent => MyString::from("%"),
            Token::Ampersand => MyString::from("&"),
            Token::VerticalBar => MyString::from("|"),
            Token::LeftParenCurly => MyString::from("{"),
            Token::RightParenCurly => MyString::from("}"),
            Token::LeftParenSquare => MyString::from("["),
            Token::RightParenSquare => MyString::from("]"),
            Token::Comma => MyString::from(","),
            Token::If => MyString::from("if"),
            Token::While => MyString::from("while"),
            Token::Package => MyString::from("package"),
            Token::Import => MyString::from("import"),
            Token::Public => MyString::from("public"),
            Token::Private => MyString::from("private"),
            Token::Protected => MyString::from("protedted"),
            Token::Class => MyString::from("class"),
            Token::Interface => MyString::from("interface"),
            Token::Enum => MyString::from("enum"),
            Token::Void => MyString::from("void"),
            Token::Throws => MyString::from("throws"),
            Token::Int => MyString::from("int"),
            Token::Double => MyString::from("double"),
            Token::Float => MyString::from("float"),
            Token::Slash => MyString::from("/"),
            Token::BackSlash => MyString::from("\\"),
            Token::At => MyString::from("@"),
            Token::Le => MyString::from("<="),
            Token::Lt => MyString::from("<"),
            Token::Ge => MyString::from(">="),
            Token::Gt => MyString::from(">"),
            Token::Extends => MyString::from("extends"),
            Token::Implements => MyString::from("implements"),
            Token::True => MyString::from("true"),
            Token::False => MyString::from("false"),
            Token::EqualDouble => MyString::from("=="),
            Token::Equal => MyString::from("="),
            Token::Ne => MyString::from("!="),
            Token::ExclamationMark => MyString::from("!"),
            Token::SingleQuote => MyString::from("'"),
            Token::New => MyString::from("new"),
            Token::Return => MyString::from("return"),
            Token::QuestionMark => MyString::from("?"),
            Token::Char => MyString::from("char"),
            Token::Boolean => MyString::from("boolean"),
            Token::Byte => MyString::from("byte"),
            Token::Short => MyString::from("short"),
            Token::Long => MyString::from("long"),
            Token::Static => MyString::from("static"),
            Token::Final => MyString::from("final"),
            Token::Default => MyString::from("default"),
            Token::Else => MyString::from("else"),
            Token::For => MyString::from("for"),
            Token::Break => MyString::from("break"),
            Token::Continue => MyString::from("continue"),
            Token::Switch => MyString::from("swtich"),
            Token::Case => MyString::from("case"),
            Token::Do => MyString::from("do"),
            Token::Try => MyString::from("try"),
            Token::Catch => MyString::from("catch"),
            Token::Finally => MyString::from("finally"),
            Token::Throw => MyString::from("throw"),
            Token::Yield => MyString::from("yield"),
            Token::Var => MyString::from("var"),
            Token::This => MyString::from("this"),
            Token::Underscore => MyString::from("_"),
            Token::Abstract => MyString::from("abstract"),
            Token::Record => MyString::from("record"),
            Token::Synchronized => MyString::from("synchronized"),
            Token::InstanceOf => MyString::from("instanceof"),
            Token::Volatile => MyString::from("volatile"),
            Token::Transient => MyString::from("transient"),
            Token::Native => MyString::from("native"),
            Token::Caret => MyString::from("^"),
            Token::Tilde => MyString::from("~"),
            Token::Sealed => MyString::from("sealed"),
            Token::Non => MyString::from("non"),
            Token::Permits => MyString::from("permits"),
            Token::Arrow => MyString::from("->"),
        }
    }

    #[must_use]
    /// if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Tokens of document
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    /// Data
    Identifier(MyString),
    /// Data
    StringLiteral(MyString),
    /// \r
    CharLiteral(MyString),
    /// 123
    Number(MyString),
    /// (
    LeftParen,
    /// )
    RightParen,
    /// +
    Plus,
    /// -
    Dash,
    /// *
    Star,
    /// .
    Dot,
    /// ;
    Semicolon,
    /// :
    Colon,
    /// %
    Percent,
    /// &
    Ampersand,
    /// |
    VerticalBar,
    /// {
    LeftParenCurly,
    /// }
    RightParenCurly,
    /// ,
    Comma,
    /// if
    If,
    /// while
    While,
    /// for
    For,
    /// package
    Package,
    /// import
    Import,
    /// public
    Public,
    /// private
    Private,
    /// protected
    Protected,
    /// class
    Class,
    /// interface
    Interface,
    /// enum
    Enum,
    /// void
    Void,
    /// throws
    Throws,
    /// int
    Int,
    /// double
    Double,
    /// float
    Float,
    /// /
    Slash,
    /// \
    BackSlash,
    /// @
    At,
    /// <=
    Le,
    /// <
    Lt,
    /// >=
    Ge,
    /// >
    Gt,
    /// extends
    Extends,
    /// implements
    Implements,
    /// true
    True,
    /// false
    False,
    /// ==
    EqualDouble,
    /// =
    Equal,
    /// !=
    Ne,
    /// !
    ExclamationMark,
    /// '
    SingleQuote,
    /// new
    New,
    /// return
    Return,
    /// ?
    QuestionMark,
    /// char
    Char,
    /// boolean
    Boolean,
    /// byte
    Byte,
    /// short
    Short,
    /// long
    Long,
    /// static
    Static,
    /// final
    Final,
    /// defautl
    Default,
    /// [
    LeftParenSquare,
    /// ]
    RightParenSquare,
    /// else
    Else,
    /// break
    Break,
    /// continue
    Continue,
    /// switch
    Switch,
    /// case
    Case,
    /// do
    Do,
    /// try
    Try,
    /// catch
    Catch,
    /// finally
    Finally,
    /// throw
    Throw,
    /// yield
    Yield,
    /// var
    Var,
    /// this
    This,
    /// _
    Underscore,
    /// abstract
    Abstract,
    /// record
    Record,
    /// synchronized
    Synchronized,
    /// instanceof
    InstanceOf,
    /// volatile
    Volatile,
    /// transient
    Transient,
    /// native
    Native,
    /// `^`
    Caret,
    /// `~`
    Tilde,
    /// sealed
    Sealed,
    /// non (used in non-sealed)
    Non,
    /// permits
    Permits,
    /// ->
    Arrow,
}

/// Error during lex function
#[derive(Debug, PartialEq)]
pub enum LexerError {
    /// Not implmented
    UnknwonChar(char),
}

static KEYWORDS: phf::Map<&'static str, Token> = phf_map! {
    "if" => Token::If,
    "true" => Token::True,
    "false" => Token::False,
    "while" => Token::While,
    "for" => Token::For,
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
    "final" => Token::Final,
    "default" => Token::Default,
    "else" => Token::Else,
    "break" => Token::Break,
    "continue" => Token::Continue,
    "switch" => Token::Switch,
    "case" => Token::Case,
    "do" => Token::Do,
    "try" => Token::Try,
    "catch" => Token::Catch,
    "finally" => Token::Finally,
    "throw" => Token::Throw,
    "yield" => Token::Yield,
    "var" => Token::Var,
    "this" => Token::This,
    "abstract" => Token::Abstract,
    "record" => Token::Record,
    "synchronized" => Token::Synchronized,
    "instanceof" => Token::InstanceOf,
    "volatile" => Token::Volatile,
    "transient" => Token::Transient,
    "native" => Token::Native,
    "sealed" => Token::Sealed,
    "non" => Token::Non,
    "permits" => Token::Permits,
};

/// Output token vec for document
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
            '[' => {
                tokens.push(PositionToken {
                    token: Token::LeftParenSquare,
                    line,
                    col,
                });
                col += 1;
            }
            ']' => {
                tokens.push(PositionToken {
                    token: Token::RightParenSquare,
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
                if let Some('-') = chars.get(index + 1) {
                    tokens.push(PositionToken {
                        token: Token::Dash,
                        line,
                        col,
                    });
                    tokens.push(PositionToken {
                        token: Token::Dash,
                        line,
                        col,
                    });
                    index += 1;
                    col += 2;
                } else if let Some('>') = chars.get(index + 1) {
                    tokens.push(PositionToken {
                        token: Token::Arrow,
                        line,
                        col,
                    });
                    index += 1;
                    col += 2;
                } else {
                    tokens.push(PositionToken {
                        token: Token::Dash,
                        line,
                        col,
                    });
                    col += 1;
                }
            }
            '*' => {
                tokens.push(PositionToken {
                    token: Token::Star,
                    line,
                    col,
                });
                col += 1;
            }
            '^' => {
                tokens.push(PositionToken {
                    token: Token::Caret,
                    line,
                    col,
                });
                col += 1;
            }
            '~' => {
                tokens.push(PositionToken {
                    token: Token::Tilde,
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
            ':' => {
                tokens.push(PositionToken {
                    token: Token::Colon,
                    line,
                    col,
                });
                col += 1;
            }
            '%' => {
                tokens.push(PositionToken {
                    token: Token::Percent,
                    line,
                    col,
                });
                col += 1;
            }
            '&' => {
                tokens.push(PositionToken {
                    token: Token::Ampersand,
                    line,
                    col,
                });
                col += 1;
            }
            '|' => {
                tokens.push(PositionToken {
                    token: Token::VerticalBar,
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
                index += 1;
                let mut str = String::new();
                let mut multi_line = false;
                if let Some('"') = chars.get(index)
                    && let Some('"') = chars.get(index + 1)
                {
                    multi_line = true;
                    index += 2;
                }
                'string_literal: loop {
                    let Some(ch) = chars.get(index) else {
                        break;
                    };
                    if *ch == '\\' {
                        let Some(peek) = chars.get(index + 1) else {
                            break;
                        };
                        if *peek == '\\' {
                            str.push('\\');
                            str.push('\\');
                            col += 2;
                            index += 2;
                            continue;
                        } else if *peek == '"' {
                            str.push('\\');
                            str.push('\"');
                            col += 2;
                            index += 2;
                            continue;
                        }
                    }
                    if *ch == '"' {
                        if !multi_line {
                            col += 1;
                            break 'string_literal;
                        } else if let Some('"') = chars.get(index + 1)
                            && let Some('"') = chars.get(index + 2)
                        {
                            index += 2;
                            col += 2;
                            break 'string_literal;
                        }
                    }
                    str.push(*ch);
                    index += 1;
                    col += 1;
                }
                tokens.push(PositionToken {
                    token: Token::StringLiteral(str),
                    line,
                    col,
                });
                col += 1;
            }
            '\'' => {
                index += 1;
                let mut char = MyString::new();
                'char_literal: loop {
                    let Some(ch) = chars.get(index) else {
                        break;
                    };
                    if *ch == '\\' {
                        let Some(peek) = chars.get(index + 1) else {
                            break;
                        };
                        if *peek == '\\' {
                            char.push('\\');
                            char.push('\\');
                            col += 2;
                            index += 2;
                            continue;
                        } else if *peek == '\'' {
                            char.push('\\');
                            char.push('\'');
                            col += 2;
                            index += 2;
                            continue;
                        }
                    }
                    if *ch == '\'' {
                        break 'char_literal;
                    }
                    char.push(*ch);
                    index += 1;
                    col += 1;
                }
                tokens.push(PositionToken {
                    token: Token::CharLiteral(char),
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
                    index += 1;
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
                    index += 1;
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
                    index += 1;
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
                let mut string = MyString::new();
                loop {
                    let Some(ch) = chars.get(index) else {
                        break;
                    };
                    if ch.is_ascii_digit() || ch == &'_' {
                        string.push(*ch);
                    } else {
                        break;
                    };
                    index += 1;
                }
                let string = string;

                col += string.len();
                tokens.push(PositionToken {
                    token: Token::Number(string),
                    line,
                    col,
                });
                continue;
            }
            'A'..='Z' | 'a'..='z' | '_' | '$' => {
                let mut ident = MyString::new();
                loop {
                    let Some(ch) = chars.get(index) else {
                        break;
                    };
                    if !ch.is_ascii_alphanumeric() && ch != &'_' && ch != &'$' {
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

/// tests
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
    #[test]
    fn escaped_double_quetes() {
        let content = r#"return "\"" + s + "\"";"#;
        let tokens = lexer::lex(content).unwrap();
        insta::assert_debug_snapshot!(tokens);
    }
    #[test]
    fn escaped_backslash() {
        let content = r#" "\\" "#;
        let tokens = lexer::lex(content).unwrap();
        insta::assert_debug_snapshot!(tokens);
    }
    #[test]
    fn escaped_others() {
        let content = r#" 
            '\b' + 
            '\t' + 
            '\n' + 
            '\f' + 
            '\r' + 
            '\"' +
            '\\' + 
         "#;
        let tokens = lexer::lex(content).unwrap();
        insta::assert_debug_snapshot!(tokens);
    }
}
