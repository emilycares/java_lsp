//! Transform document into a token vector
use core::fmt;

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
            Token::HexLiteral(n) => n.to_string().len() + 2,
            Token::BinaryLiteral(n) => n.to_string().len() + 2,
            Token::AtInterface => 11,
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
            | Token::Super
            | Token::StrictFp
            | Token::Module
            | Token::Exports
            | Token::To
            | Token::Uses
            | Token::Assert
            | Token::Provides
            | Token::With
            | Token::Requires
            | Token::Transitive
            | Token::Opens
            | Token::Open
            | Token::If => KEYWORDS
                .entries()
                .find(|i| i.1 == self)
                .map(|i| i.0.len())
                .unwrap_or(0),
        }
    }

    #[must_use]
    /// if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
impl fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Identifier(s) => write!(f, "{}", s),
            Token::StringLiteral(s) => write!(f, "{}", s),
            Token::CharLiteral(s) => write!(f, "{}", s),
            Token::Number(num) => write!(f, "{}", num),
            Token::HexLiteral(num) => write!(f, "0x{}", num),
            Token::BinaryLiteral(num) => write!(f, "0b{}", num),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::Plus => write!(f, "+"),
            Token::Dash => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Dot => write!(f, "."),
            Token::Semicolon => write!(f, ";"),
            Token::Colon => write!(f, ":"),
            Token::Percent => write!(f, "%"),
            Token::Ampersand => write!(f, "&"),
            Token::VerticalBar => write!(f, "|"),
            Token::LeftParenCurly => write!(f, "{{"),
            Token::RightParenCurly => write!(f, "}}"),
            Token::LeftParenSquare => write!(f, "["),
            Token::RightParenSquare => write!(f, "]"),
            Token::Comma => write!(f, ","),
            Token::If => write!(f, "if"),
            Token::While => write!(f, "while"),
            Token::Package => write!(f, "package"),
            Token::Import => write!(f, "import"),
            Token::Public => write!(f, "public"),
            Token::Private => write!(f, "private"),
            Token::Protected => write!(f, "protedted"),
            Token::Class => write!(f, "class"),
            Token::Interface => write!(f, "interface"),
            Token::Enum => write!(f, "enum"),
            Token::Void => write!(f, "void"),
            Token::Throws => write!(f, "throws"),
            Token::Int => write!(f, "int"),
            Token::Double => write!(f, "double"),
            Token::Float => write!(f, "float"),
            Token::Slash => write!(f, "/"),
            Token::BackSlash => write!(f, "\\"),
            Token::At => write!(f, "@"),
            Token::Le => write!(f, "<="),
            Token::Lt => write!(f, "<"),
            Token::Ge => write!(f, ">="),
            Token::Gt => write!(f, ">"),
            Token::Extends => write!(f, "extends"),
            Token::Implements => write!(f, "implements"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::EqualDouble => write!(f, "=="),
            Token::Equal => write!(f, "="),
            Token::Ne => write!(f, "!="),
            Token::ExclamationMark => write!(f, "!"),
            Token::SingleQuote => write!(f, "'"),
            Token::New => write!(f, "new"),
            Token::Return => write!(f, "return"),
            Token::QuestionMark => write!(f, "?"),
            Token::Char => write!(f, "char"),
            Token::Boolean => write!(f, "boolean"),
            Token::Byte => write!(f, "byte"),
            Token::Short => write!(f, "short"),
            Token::Long => write!(f, "long"),
            Token::Static => write!(f, "static"),
            Token::Final => write!(f, "final"),
            Token::Default => write!(f, "default"),
            Token::Else => write!(f, "else"),
            Token::For => write!(f, "for"),
            Token::Break => write!(f, "break"),
            Token::Continue => write!(f, "continue"),
            Token::Switch => write!(f, "swtich"),
            Token::Case => write!(f, "case"),
            Token::Do => write!(f, "do"),
            Token::Try => write!(f, "try"),
            Token::Catch => write!(f, "catch"),
            Token::Finally => write!(f, "finally"),
            Token::Throw => write!(f, "throw"),
            Token::Yield => write!(f, "yield"),
            Token::Var => write!(f, "var"),
            Token::This => write!(f, "this"),
            Token::Underscore => write!(f, "_"),
            Token::Abstract => write!(f, "abstract"),
            Token::Record => write!(f, "record"),
            Token::Synchronized => write!(f, "synchronized"),
            Token::InstanceOf => write!(f, "instanceof"),
            Token::Volatile => write!(f, "volatile"),
            Token::Transient => write!(f, "transient"),
            Token::Native => write!(f, "native"),
            Token::Caret => write!(f, "^"),
            Token::Tilde => write!(f, "~"),
            Token::Sealed => write!(f, "sealed"),
            Token::Non => write!(f, "non"),
            Token::Permits => write!(f, "permits"),
            Token::Arrow => write!(f, "->"),
            Token::Super => write!(f, "super"),
            Token::StrictFp => write!(f, "staticfp"),
            Token::AtInterface => write!(f, "@interface"),
            Token::Module => write!(f, "module"),
            Token::Exports => write!(f, "exports"),
            Token::To => write!(f, "to"),
            Token::Uses => write!(f, "uses"),
            Token::Assert => write!(f, "assert"),
            Token::Provides => write!(f, "provides"),
            Token::With => write!(f, "with"),
            Token::Requires => write!(f, "requires"),
            Token::Transitive => write!(f, "transitive"),
            Token::Opens => write!(f, "opens"),
            Token::Open => write!(f, "open"),
        }
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
    /// `0xFFFFFF`
    HexLiteral(MyString),
    /// `0b101`
    BinaryLiteral(MyString),
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
    /// super
    Super,
    /// staticfp
    StrictFp,
    /// @interface
    AtInterface,
    /// module
    Module,
    /// exports
    Exports,
    /// to
    To,
    /// Uses
    Uses,
    /// assert
    Assert,
    /// provides
    Provides,
    /// with
    With,
    /// requires
    Requires,
    /// transitive
    Transitive,
    /// opens
    Opens,
    /// open
    Open,
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
    "super" => Token::Super,
    "strictfp" => Token::StrictFp,
    "module" => Token::Module,
    "exports" => Token::Exports,
    "to" => Token::To,
    "uses" => Token::Uses,
    "assert" => Token::Assert,
    "provides" => Token::Provides,
    "with" => Token::With,
    "requires" => Token::Requires,
    "transitive" => Token::Transitive,
    "opens" => Token::Opens,
    "open" => Token::Open,
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
                let interface = &chars[index + 1..index + 10];
                let interface: String = interface.iter().collect();
                if interface == "interface" {
                    tokens.push(PositionToken {
                        token: Token::AtInterface,
                        line,
                        col,
                    });
                    col += 10;
                    index += 10;
                } else {
                    tokens.push(PositionToken {
                        token: Token::At,
                        line,
                        col,
                    });
                    col += 1;
                }
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
                if let Some('0') = chars.get(index) {
                    match chars.get(index + 1) {
                        Some('x') | Some('X') => {
                            index += 2;
                            let mut string = MyString::new();
                            loop {
                                let Some(ch) = chars.get(index) else {
                                    break;
                                };
                                if ch.is_ascii_hexdigit()
                                    || ch == &'_'
                                    || ch == &'.'
                                    || ch == &'p'
                                    || ch == &'-'
                                {
                                    string.push(*ch);
                                    index += 1;
                                } else {
                                    break;
                                };
                            }
                            col += string.len();
                            tokens.push(PositionToken {
                                token: Token::HexLiteral(string),
                                line,
                                col,
                            });
                            continue;
                        }
                        Some('b') | Some('B') => {
                            index += 2;
                            let mut string = MyString::new();
                            loop {
                                let Some(ch) = chars.get(index) else {
                                    break;
                                };
                                if ch == &'_' || ch == &'0' || ch == &'1' {
                                    string.push(*ch);
                                    index += 1;
                                } else {
                                    break;
                                };
                            }
                            col += string.len();
                            tokens.push(PositionToken {
                                token: Token::BinaryLiteral(string),
                                line,
                                col,
                            });
                            continue;
                        }
                        _ => (),
                    }
                }
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
        let tokens = lexer::lex(content).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn supere() {
        let content = include_str!("../../parser/test/Super.java");
        let tokens = lexer::lex(content).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn super_interface() {
        let content = include_str!("../../parser/test/SuperInterface.java");
        let tokens = lexer::lex(content).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn everything() {
        let content = include_str!("../../parser/test/Everything.java");
        let tokens = lexer::lex(content).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn thrower() {
        let content = include_str!("../../parser/test/Thrower.java");
        let tokens = lexer::lex(content).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }
    #[test]
    fn escaped_double_quetes() {
        let content = r#"return "\"" + s + "\"";"#;
        let tokens = lexer::lex(content).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }
    #[test]
    fn escaped_backslash() {
        let content = r#" "\\" "#;
        let tokens = lexer::lex(content).expect("Test");
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
        let tokens = lexer::lex(content).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }
}
