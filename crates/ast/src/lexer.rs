//! Transform document into a token vector
use core::fmt;

use memchr::memchr;
use memchr::memchr_iter;
use memchr::memmem;
use my_string::MyString;
use phf::phf_map;

use crate::types::AstPoint;

/// Position in document
#[derive(Debug, PartialEq, Eq, Clone)]
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
    #[must_use]
    pub const fn start_point(&self) -> AstPoint {
        AstPoint {
            line: self.line,
            col: self.col,
        }
    }
    /// End point of Token
    #[must_use]
    pub fn end_point(&self) -> AstPoint {
        AstPoint {
            line: self.line,
            col: self.col + self.token.len(),
        }
    }
}

impl Token {
    /// Length of token
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Identifier(i) | Self::StringLiteral(i) | Self::CharLiteral(i) => i.len(),
            Self::Number(n) => n.len(),
            Self::HexLiteral(n) | Self::BinaryLiteral(n) => n.len() + 2,
            Self::AtInterface => 11,
            Self::LeftParen
            | Self::RightParen
            | Self::Plus
            | Self::Dash
            | Self::Star
            | Self::Dot
            | Self::Colon
            | Self::Semicolon
            | Self::Percent
            | Self::Ampersand
            | Self::VerticalBar
            | Self::LeftParenCurly
            | Self::RightParenCurly
            | Self::LeftParenSquare
            | Self::RightParenSquare
            | Self::Comma
            | Self::Slash
            | Self::BackSlash
            | Self::At
            | Self::Lt
            | Self::Gt
            | Self::Equal
            | Self::ExclamationMark
            | Self::Underscore
            | Self::Caret
            | Self::Tilde
            | Self::SingleQuote => 1,
            Self::EqualDouble | Self::Le | Self::Ge | Self::Ne | Self::Arrow => 2,
            Self::While
            | Self::Package
            | Self::Import
            | Self::Public
            | Self::Private
            | Self::Protected
            | Self::Class
            | Self::Interface
            | Self::Enum
            | Self::Void
            | Self::Throws
            | Self::Int
            | Self::Double
            | Self::Extends
            | Self::Implements
            | Self::True
            | Self::False
            | Self::Float
            | Self::New
            | Self::Return
            | Self::QuestionMark
            | Self::Char
            | Self::Boolean
            | Self::Byte
            | Self::Short
            | Self::Long
            | Self::Static
            | Self::Final
            | Self::Default
            | Self::Else
            | Self::For
            | Self::Break
            | Self::Continue
            | Self::Switch
            | Self::Case
            | Self::Do
            | Self::Try
            | Self::Catch
            | Self::Finally
            | Self::Throw
            | Self::Yield
            | Self::Var
            | Self::This
            | Self::Abstract
            | Self::Record
            | Self::Synchronized
            | Self::InstanceOf
            | Self::Volatile
            | Self::Transient
            | Self::Native
            | Self::Sealed
            | Self::Non
            | Self::Permits
            | Self::Super
            | Self::StrictFp
            | Self::Module
            | Self::Exports
            | Self::To
            | Self::Uses
            | Self::Assert
            | Self::Provides
            | Self::With
            | Self::Requires
            | Self::Transitive
            | Self::Opens
            | Self::Open
            | Self::If => KEYWORDS
                .entries()
                .find(|i| i.1 == self)
                .map_or(0, |i| i.0.len()),
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
            Self::Number(s)
            | Self::Identifier(s)
            | Self::StringLiteral(s)
            | Self::CharLiteral(s) => {
                write!(f, "{s}")
            }
            Self::HexLiteral(num) => write!(f, "0x{num}"),
            Self::BinaryLiteral(num) => write!(f, "0b{num}"),
            Self::LeftParen => write!(f, "("),
            Self::RightParen => write!(f, ")"),
            Self::Plus => write!(f, "+"),
            Self::Dash => write!(f, "-"),
            Self::Star => write!(f, "*"),
            Self::Dot => write!(f, "."),
            Self::Semicolon => write!(f, ";"),
            Self::Colon => write!(f, ":"),
            Self::Percent => write!(f, "%"),
            Self::Ampersand => write!(f, "&"),
            Self::VerticalBar => write!(f, "|"),
            Self::LeftParenCurly => write!(f, "{{"),
            Self::RightParenCurly => write!(f, "}}"),
            Self::LeftParenSquare => write!(f, "["),
            Self::RightParenSquare => write!(f, "]"),
            Self::Comma => write!(f, ","),
            Self::If => write!(f, "if"),
            Self::While => write!(f, "while"),
            Self::Package => write!(f, "package"),
            Self::Import => write!(f, "import"),
            Self::Public => write!(f, "public"),
            Self::Private => write!(f, "private"),
            Self::Protected => write!(f, "protedted"),
            Self::Class => write!(f, "class"),
            Self::Interface => write!(f, "interface"),
            Self::Enum => write!(f, "enum"),
            Self::Void => write!(f, "void"),
            Self::Throws => write!(f, "throws"),
            Self::Int => write!(f, "int"),
            Self::Double => write!(f, "double"),
            Self::Float => write!(f, "float"),
            Self::Slash => write!(f, "/"),
            Self::BackSlash => write!(f, "\\"),
            Self::At => write!(f, "@"),
            Self::Le => write!(f, "<="),
            Self::Lt => write!(f, "<"),
            Self::Ge => write!(f, ">="),
            Self::Gt => write!(f, ">"),
            Self::Extends => write!(f, "extends"),
            Self::Implements => write!(f, "implements"),
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
            Self::EqualDouble => write!(f, "=="),
            Self::Equal => write!(f, "="),
            Self::Ne => write!(f, "!="),
            Self::ExclamationMark => write!(f, "!"),
            Self::SingleQuote => write!(f, "'"),
            Self::New => write!(f, "new"),
            Self::Return => write!(f, "return"),
            Self::QuestionMark => write!(f, "?"),
            Self::Char => write!(f, "char"),
            Self::Boolean => write!(f, "boolean"),
            Self::Byte => write!(f, "byte"),
            Self::Short => write!(f, "short"),
            Self::Long => write!(f, "long"),
            Self::Static => write!(f, "static"),
            Self::Final => write!(f, "final"),
            Self::Default => write!(f, "default"),
            Self::Else => write!(f, "else"),
            Self::For => write!(f, "for"),
            Self::Break => write!(f, "break"),
            Self::Continue => write!(f, "continue"),
            Self::Switch => write!(f, "switch"),
            Self::Case => write!(f, "case"),
            Self::Do => write!(f, "do"),
            Self::Try => write!(f, "try"),
            Self::Catch => write!(f, "catch"),
            Self::Finally => write!(f, "finally"),
            Self::Throw => write!(f, "throw"),
            Self::Yield => write!(f, "yield"),
            Self::Var => write!(f, "var"),
            Self::This => write!(f, "this"),
            Self::Underscore => write!(f, "_"),
            Self::Abstract => write!(f, "abstract"),
            Self::Record => write!(f, "record"),
            Self::Synchronized => write!(f, "synchronized"),
            Self::InstanceOf => write!(f, "instanceof"),
            Self::Volatile => write!(f, "volatile"),
            Self::Transient => write!(f, "transient"),
            Self::Native => write!(f, "native"),
            Self::Caret => write!(f, "^"),
            Self::Tilde => write!(f, "~"),
            Self::Sealed => write!(f, "sealed"),
            Self::Non => write!(f, "non"),
            Self::Permits => write!(f, "permits"),
            Self::Arrow => write!(f, "->"),
            Self::Super => write!(f, "super"),
            Self::StrictFp => write!(f, "staticfp"),
            Self::AtInterface => write!(f, "@interface"),
            Self::Module => write!(f, "module"),
            Self::Exports => write!(f, "exports"),
            Self::To => write!(f, "to"),
            Self::Uses => write!(f, "uses"),
            Self::Assert => write!(f, "assert"),
            Self::Provides => write!(f, "provides"),
            Self::With => write!(f, "with"),
            Self::Requires => write!(f, "requires"),
            Self::Transitive => write!(f, "transitive"),
            Self::Opens => write!(f, "opens"),
            Self::Open => write!(f, "open"),
        }
    }
}

/// Tokens of document
#[derive(Debug, PartialEq, Eq, Clone)]
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
    /// default
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
#[derive(Debug, PartialEq, Eq)]
pub enum LexerError {
    /// Not implemented char
    UnknownChar(char, usize, usize),
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
pub fn lex(input: &[u8]) -> Result<Vec<PositionToken>, LexerError> {
    let mut tokens = Vec::new();

    lex_mut(input, &mut tokens)?;

    Ok(tokens)
}

/// Fill tokens vec with tokens for document
///
/// Use this instead of `lex` if there is already a vec of tokens that can be reused
pub fn lex_mut(input: &[u8], tokens: &mut Vec<PositionToken>) -> Result<(), LexerError> {
    tokens.clear();
    let mut line = 0;
    let mut col = 0;
    let mut index = 0;

    loop {
        let ch = input.get(index);
        let Some(ch) = ch else {
            break;
        };
        match ch {
            b'\n' => {
                line += 1;
                col = 0;
                index += 1;
                continue;
            }
            ch if is_whitespace(*ch) => {
                col += 1;
                index += 1;
                continue;
            }
            b'(' => {
                tokens.push(PositionToken {
                    token: Token::LeftParen,
                    line,
                    col,
                });
                col += 1;
            }
            b')' => {
                tokens.push(PositionToken {
                    token: Token::RightParen,
                    line,
                    col,
                });
                col += 1;
            }
            b'{' => {
                tokens.push(PositionToken {
                    token: Token::LeftParenCurly,
                    line,
                    col,
                });
                col += 1;
            }
            b'}' => {
                tokens.push(PositionToken {
                    token: Token::RightParenCurly,
                    line,
                    col,
                });
                col += 1;
            }
            b'[' => {
                tokens.push(PositionToken {
                    token: Token::LeftParenSquare,
                    line,
                    col,
                });
                col += 1;
            }
            b']' => {
                tokens.push(PositionToken {
                    token: Token::RightParenSquare,
                    line,
                    col,
                });
                col += 1;
            }
            b'+' => {
                tokens.push(PositionToken {
                    token: Token::Plus,
                    line,
                    col,
                });
                col += 1;
            }
            b'-' => {
                if matches!(input.get(index + 1), Some(b'-')) {
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
                } else if matches!(input.get(index + 1), Some(b'>')) {
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
            b'*' => {
                tokens.push(PositionToken {
                    token: Token::Star,
                    line,
                    col,
                });
                col += 1;
            }
            b'^' => {
                tokens.push(PositionToken {
                    token: Token::Caret,
                    line,
                    col,
                });
                col += 1;
            }
            b'~' => {
                tokens.push(PositionToken {
                    token: Token::Tilde,
                    line,
                    col,
                });
                col += 1;
            }
            b'@' => {
                let interface = &input[index + 1..index + 10];
                if interface == b"interface" {
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
            b'.' => {
                tokens.push(PositionToken {
                    token: Token::Dot,
                    line,
                    col,
                });
                col += 1;
            }
            b',' => {
                tokens.push(PositionToken {
                    token: Token::Comma,
                    line,
                    col,
                });
                col += 1;
            }
            b';' => {
                tokens.push(PositionToken {
                    token: Token::Semicolon,
                    line,
                    col,
                });
                col += 1;
            }
            b':' => {
                tokens.push(PositionToken {
                    token: Token::Colon,
                    line,
                    col,
                });
                col += 1;
            }
            b'%' => {
                tokens.push(PositionToken {
                    token: Token::Percent,
                    line,
                    col,
                });
                col += 1;
            }
            b'&' => {
                tokens.push(PositionToken {
                    token: Token::Ampersand,
                    line,
                    col,
                });
                col += 1;
            }
            b'|' => {
                tokens.push(PositionToken {
                    token: Token::VerticalBar,
                    line,
                    col,
                });
                col += 1;
            }
            b'?' => {
                tokens.push(PositionToken {
                    token: Token::QuestionMark,
                    line,
                    col,
                });
                col += 1;
            }
            b'/' => {
                let Some(peek) = input.get(index + 1) else {
                    break;
                };
                if peek == &b'/' {
                    let slice = &input[index + 2..];
                    let Some(m) = memchr(b'\n', slice) else {
                        break;
                    };
                    let length = m + 1;
                    // slice is offset my 2
                    index += length + 2;
                    line += 1;
                    col = 0;
                    continue;
                } else if peek == &b'*' {
                    // Inside multi line comment
                    let slice = &input[index + 2..];
                    let finder = memmem::Finder::new("*/");
                    let Some(m) = finder.find(slice) else {
                        break;
                    };
                    // Include the last two chars
                    let length = m + 2;
                    let for_ln_count = &slice[..m];
                    let mut ln = memchr_iter(b'\n', for_ln_count);
                    if let Some(last) = ln.next_back() {
                        // After last newline
                        // let char_count = &slice[last + 1..length];
                        // col += char_count.len();
                        // debug_assert_eq!(char_count.len(), length - (last + 1));
                        col += length - (last + 1);
                        let ln_count = ln.count() + 1;
                        line += ln_count;
                    } else {
                        // Full comment contains no newline
                        // let char_count = &input[index..index + length + 2];
                        // debug_assert_eq!(char_count.len(), length + 2);
                        col += length + 2;
                    }
                    // slice is offset my 2
                    index += length + 2;
                    continue;
                }
                tokens.push(PositionToken {
                    token: Token::Slash,
                    line,
                    col,
                });
                col += 1;
            }
            b'\\' => {
                tokens.push(PositionToken {
                    token: Token::BackSlash,
                    line,
                    col,
                });
                col += 1;
            }
            b'"' => {
                index += 1;
                let mut str = String::new();
                let mut multi_line = false;
                if matches!(input.get(index), Some(b'"'))
                    && matches!(input.get(index + 1), Some(b'"'))
                {
                    multi_line = true;
                    index += 2;
                }
                'string_literal: loop {
                    let Some(ch) = input.get(index) else {
                        break;
                    };
                    if *ch == b'\r' {
                        index += 1;
                        continue;
                    }
                    if *ch == b'\\' {
                        let Some(peek) = input.get(index + 1) else {
                            break;
                        };
                        if *peek == b'\\' {
                            str.push('\\');
                            str.push('\\');
                            col += 2;
                            index += 2;
                            continue;
                        } else if *peek == b'"' {
                            str.push('\\');
                            str.push('\"');
                            col += 2;
                            index += 2;
                            continue;
                        }
                    }
                    if *ch == b'"' {
                        if !multi_line {
                            col += 1;
                            break 'string_literal;
                        } else if matches!(input.get(index + 1), Some(b'"'))
                            && matches!(input.get(index + 2), Some(b'"'))
                        {
                            index += 2;
                            col += 2;
                            break 'string_literal;
                        }
                    }
                    str.push(*ch as char);
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
            b'\'' => {
                index += 1;
                let mut char = MyString::new();
                'char_literal: loop {
                    let Some(ch) = input.get(index) else {
                        break;
                    };
                    if *ch == b'\\' {
                        let Some(peek) = input.get(index + 1) else {
                            break;
                        };
                        if *peek == b'\\' {
                            char.push('\\');
                            char.push('\\');
                            col += 2;
                            index += 2;
                            continue;
                        } else if *peek == b'\'' {
                            char.push('\\');
                            char.push('\'');
                            col += 2;
                            index += 2;
                            continue;
                        }
                    }
                    if *ch == b'\'' {
                        break 'char_literal;
                    }
                    char.push(*ch as char);
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
            b'=' => {
                if matches!(input.get(index + 1), Some(b'=')) {
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
                }
            }
            b'!' => {
                if matches!(input.get(index + 1), Some(b'=')) {
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
                }
            }
            b'<' => {
                if matches!(input.get(index + 1), Some(b'=')) {
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
                }
            }
            b'>' => {
                if matches!(input.get(index + 1), Some(b'=')) {
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
            b'0'..=b'9' => {
                if matches!(input.get(index + 1), Some(b'0')) {
                    match input.get(index + 1) {
                        Some(b'x' | b'X') => {
                            index += 2;
                            let mut string = MyString::new();
                            loop {
                                let Some(ch) = input.get(index) else {
                                    break;
                                };
                                if ch.is_ascii_hexdigit()
                                    || ch == &b'_'
                                    || ch == &b'.'
                                    || ch == &b'p'
                                    || ch == &b'-'
                                {
                                    string.push(*ch as char);
                                    index += 1;
                                } else {
                                    break;
                                }
                            }
                            col += string.len();
                            tokens.push(PositionToken {
                                token: Token::HexLiteral(string),
                                line,
                                col,
                            });
                            continue;
                        }
                        Some(b'b' | b'B') => {
                            index += 2;
                            let mut string = MyString::new();
                            loop {
                                let Some(ch) = input.get(index) else {
                                    break;
                                };
                                if ch == &b'_' || ch == &b'0' || ch == &b'1' {
                                    string.push(*ch as char);
                                    index += 1;
                                } else {
                                    break;
                                }
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
                    let Some(ch) = input.get(index) else {
                        break;
                    };
                    if ch.is_ascii_digit() || ch == &b'_' {
                        string.push(*ch as char);
                    } else {
                        break;
                    }
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
            b'A'..=b'Z' | b'a'..=b'z' | b'_' | b'$' => {
                let mut ident = MyString::new();
                loop {
                    let Some(ch) = input.get(index) else {
                        break;
                    };
                    if !ch.is_ascii_alphanumeric() && ch != &b'_' && ch != &b'$' {
                        break;
                    }
                    ident.push(*ch as char);
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
            _ => return Err(LexerError::UnknownChar(*ch as char, line, col)),
        }
        index += 1;
    }

    Ok(())
}

fn is_whitespace(ch: u8) -> bool {
    ch == b' ' || (b'\x09'..b'\x0d').contains(&ch)
}

/// tests
#[cfg(test)]
pub mod tests {
    use crate::lexer::{self};

    #[test]
    fn local_variable_table() {
        let content = include_str!("../../parser/test/LocalVariableTable.java");
        let tokens = lexer::lex(content.as_bytes()).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn supere() {
        let content = include_str!("../../parser/test/Super.java");
        let tokens = lexer::lex(content.as_bytes()).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn super_interface() {
        let content = include_str!("../../parser/test/SuperInterface.java");
        let tokens = lexer::lex(content.as_bytes()).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn everything() {
        let content = include_str!("../../parser/test/Everything.java");
        let tokens = lexer::lex(content.as_bytes()).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }

    #[test]
    fn thrower() {
        let content = include_str!("../../parser/test/Thrower.java");
        let tokens = lexer::lex(content.as_bytes()).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }
    #[test]
    fn escaped_double_quetes() {
        let content = r#"return "\"" + s + "\"";"#;
        let tokens = lexer::lex(content.as_bytes()).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }
    #[test]
    fn escaped_backslash() {
        let content = r#" "\\" "#;
        let tokens = lexer::lex(content.as_bytes()).expect("Test");
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
        let tokens = lexer::lex(content.as_bytes()).expect("Test");
        insta::assert_debug_snapshot!(tokens);
    }
}
