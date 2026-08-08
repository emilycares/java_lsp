//! Transform document into a token vector
use core::fmt;

use memchr::memchr;
use memchr::memchr_iter;
use memchr::memmem;
use my_string::NuVec;
use my_string::NuVecBuilder;

use crate::types::AstPoint;

/// Position in document
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
            col: self.col.saturating_add(self.token.len()),
        }
    }
}

impl Token {
    /// Length of token
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Identifier(i)
            | Self::StringLiteral(i)
            | Self::StringLiteralMulti(i)
            | Self::CharLiteral(i) => i.len(),
            Self::Number(n) => n.len(),
            Self::HexLiteral(n) | Self::BinaryLiteral(n) => n.len() + 2,
            Self::LineComment(c) => c.len() + 2,
            Self::BlockComment(c, _) => c.len() + 4,
            Self::AtInterface | Self::Protected => 11,
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
            | Self::QuestionMark
            | Self::SingleQuote => 1,
            Self::EqualDouble
            | Self::Le
            | Self::LtLt
            | Self::Ge
            | Self::Ne
            | Self::Arrow
            | Self::Do
            | Self::To
            | Self::PlusEqual
            | Self::PlusPlus
            | Self::VerticalBarEqual
            | Self::VerticalBarVerticalBar
            | Self::DashDash
            | Self::DashEqual
            | Self::AmpersandAmpersand
            | Self::PercentEqual
            | Self::SlashEqual
            | Self::StarEqual
            | Self::If => 2,
            Self::While
            | Self::Class
            | Self::False
            | Self::Float
            | Self::Short
            | Self::Final
            | Self::Break
            | Self::Catch
            | Self::Throw
            | Self::Yield
            | Self::Super
            | Self::Opens => 5,
            Self::Package
            | Self::Private
            | Self::Extends
            | Self::Boolean
            | Self::Default
            | Self::Finally
            | Self::Permits
            | Self::Exports => 7,
            Self::Import
            | Self::Public
            | Self::Throws
            | Self::Double
            | Self::Return
            | Self::Static
            | Self::Switch
            | Self::Record
            | Self::Native
            | Self::Sealed
            | Self::Module
            | Self::Assert => 6,
            Self::Interface | Self::Transient => 9,
            Self::Enum
            | Self::Void
            | Self::True
            | Self::Char
            | Self::Byte
            | Self::Long
            | Self::Else
            | Self::Case
            | Self::This
            | Self::Uses
            | Self::With
            | Self::Open => 4,
            Self::Int | Self::New | Self::For | Self::Try | Self::Non | Self::Var => 3,
            Self::Implements | Self::InstanceOf | Self::Transitive => 10,
            Self::Continue
            | Self::Abstract
            | Self::Volatile
            | Self::StrictFp
            | Self::Provides
            | Self::Requires => 8,
            Self::Synchronized => 12,
        }
    }

    #[must_use]
    /// if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    /// Make `NuVec`
    pub fn as_nuvec(&self) -> NuVec {
        match self {
            Self::Identifier(s) | Self::Number(s) => s.clone(),
            Self::StringLiteral(s) => {
                let mut b = NuVecBuilder::new();
                b.push(b'\"');
                b.extend(s);
                b.push(b'\"');
                b.finish()
            }
            Self::StringLiteralMulti(s) => {
                let mut b = NuVecBuilder::new();
                b.pusha(b"\"\"\"\"");
                b.extend(s);
                b.pusha(b"\"\"\"\"");
                b.finish()
            }
            Self::CharLiteral(s) => {
                let mut b = NuVecBuilder::new();
                b.push(b'\'');
                b.extend(s);
                b.push(b'\'');
                b.finish()
            }
            Self::LineComment(c) => {
                let mut b = NuVecBuilder::new();
                b.pusha(b"// ");
                b.extend(c);
                b.finish()
            }
            Self::BlockComment(c, _) => {
                let mut b = NuVecBuilder::new();
                b.pusha(b"/*");
                b.extend(c);
                b.pusha(b"*/");
                b.finish()
            }
            Self::HexLiteral(num) => {
                let mut b = NuVecBuilder::new();
                b.pusha(b"0x");
                b.extend(num);
                b.finish()
            }
            Self::BinaryLiteral(num) => {
                let mut b = NuVecBuilder::new();
                b.pusha(b"0b");
                b.extend(num);
                b.finish()
            }
            Self::LeftParen => NuVec::new_static(b"("),
            Self::RightParen => NuVec::new_static(b")"),
            Self::Plus => NuVec::new_static(b"+"),
            Self::PlusEqual => NuVec::new_static(b"+="),
            Self::PlusPlus => NuVec::new_static(b"++"),
            Self::Dash => NuVec::new_static(b"-"),
            Self::DashDash => NuVec::new_static(b"--"),
            Self::DashEqual => NuVec::new_static(b"-="),
            Self::Star => NuVec::new_static(b"*"),
            Self::StarEqual => NuVec::new_static(b"*="),
            Self::Dot => NuVec::new_static(b"."),
            Self::Semicolon => NuVec::new_static(b";"),
            Self::Colon => NuVec::new_static(b":"),
            Self::Percent => NuVec::new_static(b"%"),
            Self::PercentEqual => NuVec::new_static(b"%="),
            Self::Ampersand => NuVec::new_static(b"&"),
            Self::AmpersandAmpersand => NuVec::new_static(b"&&"),
            Self::VerticalBar => NuVec::new_static(b"|"),
            Self::VerticalBarEqual => NuVec::new_static(b"|="),
            Self::VerticalBarVerticalBar => NuVec::new_static(b"||"),
            Self::LeftParenCurly => NuVec::new_static(b"{{"),
            Self::RightParenCurly => NuVec::new_static(b"}}"),
            Self::LeftParenSquare => NuVec::new_static(b"["),
            Self::RightParenSquare => NuVec::new_static(b"]"),
            Self::Comma => NuVec::new_static(b"),"),
            Self::If => NuVec::new_static(b"if"),
            Self::While => NuVec::new_static(b"while"),
            Self::Package => NuVec::new_static(b"package"),
            Self::Import => NuVec::new_static(b"import"),
            Self::Public => NuVec::new_static(b"public"),
            Self::Private => NuVec::new_static(b"private"),
            Self::Protected => NuVec::new_static(b"protedted"),
            Self::Class => NuVec::new_static(b"class"),
            Self::Interface => NuVec::new_static(b"interface"),
            Self::Enum => NuVec::new_static(b"enum"),
            Self::Void => NuVec::new_static(b"void"),
            Self::Throws => NuVec::new_static(b"throws"),
            Self::Int => NuVec::new_static(b"int"),
            Self::Double => NuVec::new_static(b"double"),
            Self::Float => NuVec::new_static(b"float"),
            Self::Slash => NuVec::new_static(b"/"),
            Self::SlashEqual => NuVec::new_static(b"/="),
            Self::BackSlash => NuVec::new_static(b"\\"),
            Self::At => NuVec::new_static(b"@"),
            Self::Le => NuVec::new_static(b"<="),
            Self::Lt => NuVec::new_static(b"<"),
            Self::LtLt => NuVec::new_static(b"<<"),
            Self::Ge => NuVec::new_static(b">="),
            Self::Gt => NuVec::new_static(b">"),
            Self::Extends => NuVec::new_static(b"extends"),
            Self::Implements => NuVec::new_static(b"implements"),
            Self::True => NuVec::new_static(b"true"),
            Self::False => NuVec::new_static(b"false"),
            Self::EqualDouble => NuVec::new_static(b"=="),
            Self::Equal => NuVec::new_static(b"="),
            Self::Ne => NuVec::new_static(b"!="),
            Self::ExclamationMark => NuVec::new_static(b"!"),
            Self::SingleQuote => NuVec::new_static(b"'"),
            Self::New => NuVec::new_static(b"new"),
            Self::Return => NuVec::new_static(b"return"),
            Self::QuestionMark => NuVec::new_static(b"?"),
            Self::Char => NuVec::new_static(b"char"),
            Self::Boolean => NuVec::new_static(b"boolean"),
            Self::Byte => NuVec::new_static(b"byte"),
            Self::Short => NuVec::new_static(b"short"),
            Self::Long => NuVec::new_static(b"long"),
            Self::Static => NuVec::new_static(b"static"),
            Self::Final => NuVec::new_static(b"final"),
            Self::Default => NuVec::new_static(b"default"),
            Self::Else => NuVec::new_static(b"else"),
            Self::For => NuVec::new_static(b"for"),
            Self::Break => NuVec::new_static(b"break"),
            Self::Continue => NuVec::new_static(b"continue"),
            Self::Switch => NuVec::new_static(b"switch"),
            Self::Case => NuVec::new_static(b"case"),
            Self::Do => NuVec::new_static(b"do"),
            Self::Try => NuVec::new_static(b"try"),
            Self::Catch => NuVec::new_static(b"catch"),
            Self::Finally => NuVec::new_static(b"finally"),
            Self::Throw => NuVec::new_static(b"throw"),
            Self::Yield => NuVec::new_static(b"yield"),
            Self::Var => NuVec::new_static(b"var"),
            Self::This => NuVec::new_static(b"this"),
            Self::Underscore => NuVec::new_static(b"_"),
            Self::Abstract => NuVec::new_static(b"abstract"),
            Self::Record => NuVec::new_static(b"record"),
            Self::Synchronized => NuVec::new_static(b"synchronized"),
            Self::InstanceOf => NuVec::new_static(b"instanceof"),
            Self::Volatile => NuVec::new_static(b"volatile"),
            Self::Transient => NuVec::new_static(b"transient"),
            Self::Native => NuVec::new_static(b"native"),
            Self::Caret => NuVec::new_static(b"^"),
            Self::Tilde => NuVec::new_static(b"~"),
            Self::Sealed => NuVec::new_static(b"sealed"),
            Self::Non => NuVec::new_static(b"non"),
            Self::Permits => NuVec::new_static(b"permits"),
            Self::Arrow => NuVec::new_static(b"->"),
            Self::Super => NuVec::new_static(b"super"),
            Self::StrictFp => NuVec::new_static(b"staticfp"),
            Self::AtInterface => NuVec::new_static(b"@interface"),
            Self::Module => NuVec::new_static(b"module"),
            Self::Exports => NuVec::new_static(b"exports"),
            Self::To => NuVec::new_static(b"to"),
            Self::Open => NuVec::new_static(b"open"),
            Self::Uses => NuVec::new_static(b"uses"),
            Self::Assert => NuVec::new_static(b"assert"),
            Self::Provides => NuVec::new_static(b"provides"),
            Self::With => NuVec::new_static(b"with"),
            Self::Requires => NuVec::new_static(b"requires"),
            Self::Transitive => NuVec::new_static(b"transitive"),
            Self::Opens => NuVec::new_static(b"opens"),
        }
    }
}
impl fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Identifier(s) => {
                write!(f, "{s}")
            }
            Self::Number(s) => {
                write!(f, "{}", s.to_str())
            }
            Self::StringLiteral(s) => {
                write!(f, "\"{}\"", s.to_str())
            }
            Self::StringLiteralMulti(s) => {
                write!(f, "\"\"\"{}\"\"\"", s.to_str())
            }
            Self::CharLiteral(s) => {
                write!(f, "'{}'", s.to_str())
            }
            Self::LineComment(c) => {
                write!(f, "// {}", c.to_str())
            }
            Self::BlockComment(c, _) => {
                write!(f, "/*{}*/", c.to_str())
            }
            Self::HexLiteral(num) => write!(f, "0x{}", num.to_str()),
            Self::BinaryLiteral(num) => write!(f, "0b{}", num.to_str()),
            Self::LeftParen => write!(f, "("),
            Self::RightParen => write!(f, ")"),
            Self::Plus => write!(f, "+"),
            Self::PlusEqual => write!(f, "+="),
            Self::PlusPlus => write!(f, "++"),
            Self::Dash => write!(f, "-"),
            Self::DashDash => write!(f, "--"),
            Self::DashEqual => write!(f, "-="),
            Self::Star => write!(f, "*"),
            Self::StarEqual => write!(f, "*="),
            Self::Dot => write!(f, "."),
            Self::Semicolon => write!(f, ";"),
            Self::Colon => write!(f, ":"),
            Self::Percent => write!(f, "%"),
            Self::PercentEqual => write!(f, "%="),
            Self::Ampersand => write!(f, "&"),
            Self::AmpersandAmpersand => write!(f, "&&"),
            Self::VerticalBar => write!(f, "|"),
            Self::VerticalBarEqual => write!(f, "|="),
            Self::VerticalBarVerticalBar => write!(f, "||"),
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
            Self::SlashEqual => write!(f, "/="),
            Self::BackSlash => write!(f, "\\"),
            Self::At => write!(f, "@"),
            Self::Le => write!(f, "<="),
            Self::Lt => write!(f, "<"),
            Self::LtLt => write!(f, "<<"),
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

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Identifier(s) => {
                write!(f, "Identifier(\"{s}\")")
            }
            Self::Number(s) => {
                write!(f, "Number({})", s.to_str())
            }
            Self::StringLiteral(s) => {
                write!(f, "String(\"{}\")", s.to_str())
            }
            Self::StringLiteralMulti(s) => {
                write!(f, "\"\"\"{}\"\"\"", s.to_str())
            }
            Self::CharLiteral(s) => {
                write!(f, "Char('{}')", s.to_str())
            }
            Self::LineComment(c) => {
                write!(f, "// {}", c.to_str())
            }
            Self::BlockComment(c, _) => {
                write!(f, "/*{}*/", c.to_str())
            }
            Self::HexLiteral(num) => write!(f, "Hex(0x{})", num.to_str()),
            Self::BinaryLiteral(num) => write!(f, "Binary(0b{})", num.to_str()),
            Self::LeftParen => write!(f, "("),
            Self::RightParen => write!(f, ")"),
            Self::Plus => write!(f, "+"),
            Self::PlusEqual => write!(f, "+="),
            Self::PlusPlus => write!(f, "++"),
            Self::Dash => write!(f, "-"),
            Self::DashDash => write!(f, "--"),
            Self::DashEqual => write!(f, "-="),
            Self::Star => write!(f, "*"),
            Self::StarEqual => write!(f, "*="),
            Self::Dot => write!(f, "."),
            Self::Semicolon => write!(f, ";"),
            Self::Colon => write!(f, ":"),
            Self::Percent => write!(f, "%"),
            Self::PercentEqual => write!(f, "%="),
            Self::Ampersand => write!(f, "&"),
            Self::AmpersandAmpersand => write!(f, "&&"),
            Self::VerticalBar => write!(f, "|"),
            Self::VerticalBarEqual => write!(f, "|="),
            Self::VerticalBarVerticalBar => write!(f, "||"),
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
            Self::SlashEqual => write!(f, "/="),
            Self::BackSlash => write!(f, "\\"),
            Self::At => write!(f, "@"),
            Self::Le => write!(f, "<="),
            Self::Lt => write!(f, "<"),
            Self::LtLt => write!(f, "<<"),
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
#[derive(PartialEq, Eq, Clone)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum Token {
    /// Line comment
    LineComment(NuVec),
    /// Block comment
    BlockComment(NuVec, usize),
    /// Data
    Identifier(NuVec),
    /// "Data"
    StringLiteral(NuVec),
    /// """Data"""
    StringLiteralMulti(NuVec),
    /// \r
    CharLiteral(NuVec),
    /// 123
    Number(NuVec),
    /// `0xFFFFFF`
    HexLiteral(NuVec),
    /// `0b101`
    BinaryLiteral(NuVec),
    /// (
    LeftParen,
    /// )
    RightParen,
    /// +
    Plus,
    /// +=
    PlusEqual,
    /// ++
    PlusPlus,
    /// -
    Dash,
    /// --
    DashDash,
    /// -=
    DashEqual,
    /// *
    Star,
    /// *=
    StarEqual,
    /// .
    Dot,
    /// ;
    Semicolon,
    /// :
    Colon,
    /// %
    Percent,
    /// %=
    PercentEqual,
    /// &
    Ampersand,
    /// &&
    AmpersandAmpersand,
    /// |
    VerticalBar,
    /// |=
    VerticalBarEqual,
    /// ||
    VerticalBarVerticalBar,
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
    /// /=
    SlashEqual,
    /// \
    BackSlash,
    /// @
    At,
    /// <=
    Le,
    /// <
    Lt,
    /// <<
    LtLt,
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
    /// Tried to read after file
    EOF(usize, usize),
}

fn keyword_to_token(key: &[u8]) -> Option<Token> {
    match key {
        b"if" => Some(Token::If),
        b"true" => Some(Token::True),
        b"false" => Some(Token::False),
        b"while" => Some(Token::While),
        b"for" => Some(Token::For),
        b"package" => Some(Token::Package),
        b"import" => Some(Token::Import),
        b"public" => Some(Token::Public),
        b"private" => Some(Token::Private),
        b"protected" => Some(Token::Protected),
        b"class" => Some(Token::Class),
        b"interface" => Some(Token::Interface),
        b"enum" => Some(Token::Enum),
        b"void" => Some(Token::Void),
        b"throws" => Some(Token::Throws),
        b"int" => Some(Token::Int),
        b"double" => Some(Token::Double),
        b"float" => Some(Token::Float),
        b"extends" => Some(Token::Extends),
        b"implements" => Some(Token::Implements),
        b"new" => Some(Token::New),
        b"return" => Some(Token::Return),
        b"char" => Some(Token::Char),
        b"boolean" => Some(Token::Boolean),
        b"byte" => Some(Token::Byte),
        b"short" => Some(Token::Short),
        b"long" => Some(Token::Long),
        b"static" => Some(Token::Static),
        b"final" => Some(Token::Final),
        b"default" => Some(Token::Default),
        b"else" => Some(Token::Else),
        b"break" => Some(Token::Break),
        b"continue" => Some(Token::Continue),
        b"switch" => Some(Token::Switch),
        b"case" => Some(Token::Case),
        b"do" => Some(Token::Do),
        b"try" => Some(Token::Try),
        b"catch" => Some(Token::Catch),
        b"finally" => Some(Token::Finally),
        b"throw" => Some(Token::Throw),
        b"yield" => Some(Token::Yield),
        b"var" => Some(Token::Var),
        b"this" => Some(Token::This),
        b"abstract" => Some(Token::Abstract),
        b"record" => Some(Token::Record),
        b"synchronized" => Some(Token::Synchronized),
        b"instanceof" => Some(Token::InstanceOf),
        b"volatile" => Some(Token::Volatile),
        b"transient" => Some(Token::Transient),
        b"native" => Some(Token::Native),
        b"sealed" => Some(Token::Sealed),
        b"non" => Some(Token::Non),
        b"permits" => Some(Token::Permits),
        b"super" => Some(Token::Super),
        b"strictfp" => Some(Token::StrictFp),
        b"module" => Some(Token::Module),
        b"exports" => Some(Token::Exports),
        b"to" => Some(Token::To),
        b"uses" => Some(Token::Uses),
        b"assert" => Some(Token::Assert),
        b"provides" => Some(Token::Provides),
        b"with" => Some(Token::With),
        b"requires" => Some(Token::Requires),
        b"transitive" => Some(Token::Transitive),
        b"opens" => Some(Token::Opens),
        b"open" => Some(Token::Open),
        _ => None,
    }
}
/// Output token vec for document
pub fn lex(input: &[u8]) -> Result<Vec<PositionToken>, LexerError> {
    lex_v::<false>(input)
}
/// Output token vec for document
pub fn lex_v<const INCLUDE_COMMENTS: bool>(input: &[u8]) -> Result<Vec<PositionToken>, LexerError> {
    let mut tokens = Vec::new();

    lex_mut::<INCLUDE_COMMENTS>(input, &mut tokens)?;

    Ok(tokens)
}

/// Fill tokens vec with tokens for document
///
/// Use this instead of `lex` if there is already a vec of tokens that can be reused
pub fn lex_mut<const INCLUDE_COMMENTS: bool>(
    input: &[u8],
    tokens: &mut Vec<PositionToken>,
) -> Result<(), LexerError> {
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
            b'\r' => {
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
                let peek = input.get(index + 1);
                if matches!(peek, Some(b'=')) {
                    tokens.push(PositionToken {
                        token: Token::PlusEqual,
                        line,
                        col,
                    });
                    index += 1;
                    col += 2;
                } else if matches!(peek, Some(b'+')) {
                    tokens.push(PositionToken {
                        token: Token::PlusPlus,
                        line,
                        col,
                    });
                    index += 1;
                    col += 2;
                } else {
                    tokens.push(PositionToken {
                        token: Token::Plus,
                        line,
                        col,
                    });
                    col += 1;
                }
            }
            b'-' => {
                let peek = input.get(index + 1);
                if matches!(peek, Some(b'-')) {
                    tokens.push(PositionToken {
                        token: Token::DashDash,
                        line,
                        col,
                    });
                    index += 1;
                    col += 2;
                } else if matches!(peek, Some(b'=')) {
                    tokens.push(PositionToken {
                        token: Token::DashEqual,
                        line,
                        col,
                    });
                    index += 1;
                    col += 2;
                } else if matches!(peek, Some(b'>')) {
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
                let peek = input.get(index + 1);
                if matches!(peek, Some(b'=')) {
                    tokens.push(PositionToken {
                        token: Token::StarEqual,
                        line,
                        col,
                    });
                    col += 1;
                    index += 1;
                } else {
                    tokens.push(PositionToken {
                        token: Token::Star,
                        line,
                        col,
                    });
                    col += 1;
                }
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
                if let Some(interface) = input.get(index + 1..index + 10)
                    && interface == b"interface"
                {
                    col += 1;
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
                if matches!(input.get(index + 1), Some(b'=')) {
                    tokens.push(PositionToken {
                        token: Token::PercentEqual,
                        line,
                        col,
                    });
                    index += 1;
                } else {
                    tokens.push(PositionToken {
                        token: Token::Percent,
                        line,
                        col,
                    });
                }
                col += 1;
            }
            b'&' => {
                if matches!(input.get(index + 1), Some(b'&')) {
                    tokens.push(PositionToken {
                        token: Token::AmpersandAmpersand,
                        line,
                        col,
                    });
                    col += 2;
                    index += 1;
                } else {
                    tokens.push(PositionToken {
                        token: Token::Ampersand,
                        line,
                        col,
                    });
                    col += 1;
                }
            }
            b'|' => {
                if let Some(peek) = input.get(index + 1) {
                    if peek == &b'=' {
                        tokens.push(PositionToken {
                            token: Token::VerticalBarEqual,
                            line,
                            col,
                        });
                        col += 2;
                        index += 1;
                    } else if peek == &b'|' {
                        tokens.push(PositionToken {
                            token: Token::VerticalBarVerticalBar,
                            line,
                            col,
                        });
                        col += 2;
                        index += 1;
                    } else {
                        tokens.push(PositionToken {
                            token: Token::VerticalBar,
                            line,
                            col,
                        });
                        col += 1;
                    }
                } else {
                    tokens.push(PositionToken {
                        token: Token::VerticalBar,
                        line,
                        col,
                    });
                    col += 1;
                }
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
                if peek == &b'=' {
                    tokens.push(PositionToken {
                        token: Token::SlashEqual,
                        line,
                        col,
                    });
                    col += 1;
                    index += 1;
                } else if peek == &b'/' {
                    let s = index + 2;
                    let slice = &input[s..];
                    let Some(m) = memchr(b'\n', slice) else {
                        break;
                    };
                    let length = m;
                    if INCLUDE_COMMENTS {
                        let content = &input
                            .get(s..input.len().min(s + length))
                            .ok_or(LexerError::EOF(line, col))?;
                        tokens.push(PositionToken {
                            token: Token::LineComment(NuVec::new(content)),
                            line,
                            col,
                        });
                    }
                    // slice is offset my 2
                    index += length + 3;
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
                        let ln_count = ln.count() + 1;
                        if INCLUDE_COMMENTS {
                            tokens.push(PositionToken {
                                token: Token::BlockComment(NuVec::new(for_ln_count), ln_count),
                                line,
                                col,
                            });
                        }
                        col += length - (last + 1);
                        line += ln_count;
                    } else {
                        // Full comment contains no newline
                        if INCLUDE_COMMENTS {
                            tokens.push(PositionToken {
                                token: Token::BlockComment(NuVec::new(for_ln_count), 0),
                                line,
                                col,
                            });
                        }
                        col += length + 2;
                    }
                    // slice is offset my 2
                    index += length + 2;
                    continue;
                } else {
                    tokens.push(PositionToken {
                        token: Token::Slash,
                        line,
                        col,
                    });
                    col += 1;
                }
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
                let mut str = NuVecBuilder::new();
                let mut multi_line = false;
                if matches!(input.get(index), Some(b'"'))
                    && matches!(input.get(index + 1), Some(b'"'))
                {
                    multi_line = true;
                    index += 2;
                }
                'string_literal: while let Some(ch) = input.get(index) {
                    if *ch == b'\\' {
                        let Some(peek) = input.get(index + 1) else {
                            break;
                        };
                        if *peek == b'\\' {
                            str.push(b'\\');
                            str.push(b'\\');
                            col += 2;
                            index += 2;
                            continue;
                        } else if *peek == b'"' {
                            str.push(b'\\');
                            str.push(b'\"');
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
                    str.push(*ch);
                    index += 1;
                    col += 1;
                }
                if multi_line {
                    tokens.push(PositionToken {
                        token: Token::StringLiteralMulti(str.finish()),
                        line,
                        col,
                    });
                } else {
                    tokens.push(PositionToken {
                        token: Token::StringLiteral(str.finish()),
                        line,
                        col,
                    });
                }
                col += 1;
            }
            b'\'' => {
                index += 1;
                let mut char = NuVecBuilder::new();
                'char_literal: while let Some(ch) = input.get(index) {
                    if *ch == b'\\' {
                        let Some(peek) = input.get(index + 1) else {
                            break;
                        };
                        if *peek == b'\\' {
                            char.push(b'\\');
                            char.push(b'\\');
                            col += 2;
                            index += 2;
                            continue;
                        } else if *peek == b'\'' {
                            char.push(b'\\');
                            char.push(b'\'');
                            col += 2;
                            index += 2;
                            continue;
                        }
                    }
                    if *ch == b'\'' {
                        break 'char_literal;
                    }
                    char.push(*ch);
                    index += 1;
                    col += 1;
                }
                tokens.push(PositionToken {
                    token: Token::CharLiteral(char.finish()),
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
                let peek = input.get(index + 1);
                if matches!(peek, Some(b'=')) {
                    tokens.push(PositionToken {
                        token: Token::Le,
                        line,
                        col,
                    });
                    col += 2;
                    index += 1;
                } else if matches!(peek, Some(b'<')) {
                    tokens.push(PositionToken {
                        token: Token::LtLt,
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
                let peek = input.get(index + 1);
                if matches!(peek, Some(b'=')) {
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
                            let mut string = NuVecBuilder::new();
                            while let Some(ch) = input.get(index) {
                                if ch.is_ascii_hexdigit()
                                    || ch == &b'_'
                                    || ch == &b'.'
                                    || ch == &b'p'
                                    || ch == &b'-'
                                {
                                    string.push(*ch);
                                    index += 1;
                                } else {
                                    break;
                                }
                            }
                            let string = string.finish();
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
                            let mut string = NuVecBuilder::new();
                            while let Some(ch) = input.get(index) {
                                if ch == &b'_' || ch == &b'0' || ch == &b'1' {
                                    string.push(*ch);
                                    index += 1;
                                } else {
                                    break;
                                }
                            }
                            let finish = string.finish();
                            col += finish.len();
                            tokens.push(PositionToken {
                                token: Token::BinaryLiteral(finish),
                                line,
                                col,
                            });
                            continue;
                        }
                        _ => (),
                    }
                }
                let mut string = NuVecBuilder::new();
                while let Some(ch) = input.get(index) {
                    if ch.is_ascii_digit() || ch == &b'_' {
                        string.push(*ch);
                    } else {
                        break;
                    }
                    index += 1;
                }

                let string = string.finish();
                col += string.len();
                tokens.push(PositionToken {
                    token: Token::Number(string),
                    line,
                    col,
                });
                continue;
            }
            _ => {
                let start = index;
                while let Some(ch) = input.get(index) {
                    if matches!(
                        ch,
                        b' ' | b'\t'
                            | b'\n'
                            | b'\r'
                            | b'\"'
                            | b'\''
                            | b'.'
                            | b'!'
                            | b'|'
                            | b'@'
                            | b':'
                            | b','
                            | b';'
                            | b'('
                            | b')'
                            | b'['
                            | b']'
                            | b'{'
                            | b'}'
                            | b'+'
                            | b'-'
                            | b'*'
                            | b'/'
                            | b'='
                            | b'<'
                            | b'>'
                    ) | (b'\x09'..b'\x0d').contains(ch)
                    {
                        break;
                    }
                    index += 1;
                }
                let end = index;
                let content = &input[start..end];
                let len = content.len();
                if let Some(t) = keyword_to_token(content) {
                    tokens.push(PositionToken {
                        token: t,
                        line,
                        col,
                    });
                } else {
                    let ident = NuVec::new(content);
                    tokens.push(PositionToken {
                        token: Token::Identifier(ident),
                        line,
                        col,
                    });
                }
                col += len;
                continue;
            }
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
    use expect_test::expect;

    use crate::lexer::{self};

    #[test]
    fn local_variable_table() {
        let content = include_bytes!("../../parser/test/LocalVariableTable.java");
        let tokens = lexer::lex(content).expect("Test");
        let expected = expect![[r#"
            [
                PositionToken {
                    token: package,
                    line: 0,
                    col: 0,
                },
                PositionToken {
                    token: Identifier("ch"),
                    line: 0,
                    col: 8,
                },
                PositionToken {
                    token: .,
                    line: 0,
                    col: 10,
                },
                PositionToken {
                    token: Identifier("emilycares"),
                    line: 0,
                    col: 11,
                },
                PositionToken {
                    token: ;,
                    line: 0,
                    col: 21,
                },
                PositionToken {
                    token: import,
                    line: 1,
                    col: 0,
                },
                PositionToken {
                    token: Identifier("java"),
                    line: 1,
                    col: 7,
                },
                PositionToken {
                    token: .,
                    line: 1,
                    col: 11,
                },
                PositionToken {
                    token: Identifier("util"),
                    line: 1,
                    col: 12,
                },
                PositionToken {
                    token: .,
                    line: 1,
                    col: 16,
                },
                PositionToken {
                    token: *,
                    line: 1,
                    col: 17,
                },
                PositionToken {
                    token: ;,
                    line: 1,
                    col: 18,
                },
                PositionToken {
                    token: public,
                    line: 2,
                    col: 0,
                },
                PositionToken {
                    token: class,
                    line: 2,
                    col: 7,
                },
                PositionToken {
                    token: Identifier("LocalVariableTable"),
                    line: 2,
                    col: 13,
                },
                PositionToken {
                    token: {,
                    line: 2,
                    col: 32,
                },
                PositionToken {
                    token: private,
                    line: 4,
                    col: 2,
                },
                PositionToken {
                    token: Identifier("HashSet"),
                    line: 4,
                    col: 10,
                },
                PositionToken {
                    token: <,
                    line: 4,
                    col: 17,
                },
                PositionToken {
                    token: Identifier("String"),
                    line: 4,
                    col: 18,
                },
                PositionToken {
                    token: >,
                    line: 4,
                    col: 24,
                },
                PositionToken {
                    token: Identifier("a"),
                    line: 4,
                    col: 26,
                },
                PositionToken {
                    token: =,
                    line: 4,
                    col: 29,
                },
                PositionToken {
                    token: new,
                    line: 4,
                    col: 30,
                },
                PositionToken {
                    token: Identifier("HashSet"),
                    line: 4,
                    col: 34,
                },
                PositionToken {
                    token: <,
                    line: 4,
                    col: 41,
                },
                PositionToken {
                    token: >,
                    line: 4,
                    col: 42,
                },
                PositionToken {
                    token: (,
                    line: 4,
                    col: 43,
                },
                PositionToken {
                    token: ),
                    line: 4,
                    col: 44,
                },
                PositionToken {
                    token: ;,
                    line: 4,
                    col: 45,
                },
                PositionToken {
                    token: public,
                    line: 6,
                    col: 2,
                },
                PositionToken {
                    token: void,
                    line: 6,
                    col: 9,
                },
                PositionToken {
                    token: Identifier("hereIsCode"),
                    line: 6,
                    col: 14,
                },
                PositionToken {
                    token: (,
                    line: 6,
                    col: 24,
                },
                PositionToken {
                    token: ),
                    line: 6,
                    col: 25,
                },
                PositionToken {
                    token: {,
                    line: 6,
                    col: 27,
                },
                PositionToken {
                    token: Identifier("HashMap"),
                    line: 7,
                    col: 4,
                },
                PositionToken {
                    token: <,
                    line: 7,
                    col: 11,
                },
                PositionToken {
                    token: Identifier("Integer"),
                    line: 7,
                    col: 12,
                },
                PositionToken {
                    token: ,,
                    line: 7,
                    col: 19,
                },
                PositionToken {
                    token: Identifier("String"),
                    line: 7,
                    col: 21,
                },
                PositionToken {
                    token: >,
                    line: 7,
                    col: 27,
                },
                PositionToken {
                    token: Identifier("a"),
                    line: 7,
                    col: 29,
                },
                PositionToken {
                    token: =,
                    line: 7,
                    col: 32,
                },
                PositionToken {
                    token: new,
                    line: 7,
                    col: 33,
                },
                PositionToken {
                    token: Identifier("HashMap"),
                    line: 7,
                    col: 37,
                },
                PositionToken {
                    token: <,
                    line: 7,
                    col: 44,
                },
                PositionToken {
                    token: >,
                    line: 7,
                    col: 45,
                },
                PositionToken {
                    token: (,
                    line: 7,
                    col: 46,
                },
                PositionToken {
                    token: ),
                    line: 7,
                    col: 47,
                },
                PositionToken {
                    token: ;,
                    line: 7,
                    col: 48,
                },
                PositionToken {
                    token: Identifier("a"),
                    line: 8,
                    col: 4,
                },
                PositionToken {
                    token: .,
                    line: 8,
                    col: 5,
                },
                PositionToken {
                    token: Identifier("put"),
                    line: 8,
                    col: 6,
                },
                PositionToken {
                    token: (,
                    line: 8,
                    col: 9,
                },
                PositionToken {
                    token: Number(1),
                    line: 8,
                    col: 11,
                },
                PositionToken {
                    token: ,,
                    line: 8,
                    col: 11,
                },
                PositionToken {
                    token: String(""),
                    line: 8,
                    col: 14,
                },
                PositionToken {
                    token: ),
                    line: 8,
                    col: 15,
                },
                PositionToken {
                    token: ;,
                    line: 8,
                    col: 16,
                },
                PositionToken {
                    token: },
                    line: 9,
                    col: 2,
                },
                PositionToken {
                    token: public,
                    line: 10,
                    col: 2,
                },
                PositionToken {
                    token: int,
                    line: 10,
                    col: 9,
                },
                PositionToken {
                    token: Identifier("hereIsCode"),
                    line: 10,
                    col: 13,
                },
                PositionToken {
                    token: (,
                    line: 10,
                    col: 23,
                },
                PositionToken {
                    token: int,
                    line: 10,
                    col: 24,
                },
                PositionToken {
                    token: Identifier("a"),
                    line: 10,
                    col: 28,
                },
                PositionToken {
                    token: ,,
                    line: 10,
                    col: 29,
                },
                PositionToken {
                    token: int,
                    line: 10,
                    col: 31,
                },
                PositionToken {
                    token: Identifier("b"),
                    line: 10,
                    col: 35,
                },
                PositionToken {
                    token: ),
                    line: 10,
                    col: 36,
                },
                PositionToken {
                    token: {,
                    line: 10,
                    col: 38,
                },
                PositionToken {
                    token: int,
                    line: 11,
                    col: 4,
                },
                PositionToken {
                    token: Identifier("o"),
                    line: 11,
                    col: 8,
                },
                PositionToken {
                    token: =,
                    line: 11,
                    col: 11,
                },
                PositionToken {
                    token: Identifier("a"),
                    line: 11,
                    col: 12,
                },
                PositionToken {
                    token: +,
                    line: 11,
                    col: 14,
                },
                PositionToken {
                    token: Identifier("b"),
                    line: 11,
                    col: 16,
                },
                PositionToken {
                    token: ;,
                    line: 11,
                    col: 17,
                },
                PositionToken {
                    token: return,
                    line: 12,
                    col: 4,
                },
                PositionToken {
                    token: Identifier("o"),
                    line: 12,
                    col: 11,
                },
                PositionToken {
                    token: -,
                    line: 12,
                    col: 13,
                },
                PositionToken {
                    token: Number(1),
                    line: 12,
                    col: 16,
                },
                PositionToken {
                    token: ;,
                    line: 12,
                    col: 16,
                },
                PositionToken {
                    token: },
                    line: 13,
                    col: 2,
                },
                PositionToken {
                    token: },
                    line: 14,
                    col: 0,
                },
            ]
        "#]];
        expected.assert_debug_eq(&tokens);
    }

    #[test]
    fn supere() {
        let content = include_bytes!("../../parser/test/Super.java");
        let tokens = lexer::lex(content).expect("Test");
        let expected = expect![[r#"
            [
                PositionToken {
                    token: package,
                    line: 0,
                    col: 0,
                },
                PositionToken {
                    token: Identifier("ch"),
                    line: 0,
                    col: 8,
                },
                PositionToken {
                    token: .,
                    line: 0,
                    col: 10,
                },
                PositionToken {
                    token: Identifier("emilycares"),
                    line: 0,
                    col: 11,
                },
                PositionToken {
                    token: ;,
                    line: 0,
                    col: 21,
                },
                PositionToken {
                    token: import,
                    line: 2,
                    col: 0,
                },
                PositionToken {
                    token: Identifier("java"),
                    line: 2,
                    col: 7,
                },
                PositionToken {
                    token: .,
                    line: 2,
                    col: 11,
                },
                PositionToken {
                    token: Identifier("io"),
                    line: 2,
                    col: 12,
                },
                PositionToken {
                    token: .,
                    line: 2,
                    col: 14,
                },
                PositionToken {
                    token: Identifier("IOException"),
                    line: 2,
                    col: 15,
                },
                PositionToken {
                    token: ;,
                    line: 2,
                    col: 26,
                },
                PositionToken {
                    token: public,
                    line: 4,
                    col: 0,
                },
                PositionToken {
                    token: class,
                    line: 4,
                    col: 7,
                },
                PositionToken {
                    token: Identifier("Super"),
                    line: 4,
                    col: 13,
                },
                PositionToken {
                    token: extends,
                    line: 4,
                    col: 19,
                },
                PositionToken {
                    token: Identifier("IOException"),
                    line: 4,
                    col: 27,
                },
                PositionToken {
                    token: {,
                    line: 4,
                    col: 39,
                },
                PositionToken {
                    token: },
                    line: 5,
                    col: 0,
                },
            ]
        "#]];
        expected.assert_debug_eq(&tokens);
    }

    #[test]
    fn super_interface() {
        let content = include_bytes!("../../parser/test/SuperInterface.java");
        let tokens = lexer::lex(content).expect("Test");
        let expected = expect![[r#"
            [
                PositionToken {
                    token: package,
                    line: 0,
                    col: 0,
                },
                PositionToken {
                    token: Identifier("ch"),
                    line: 0,
                    col: 8,
                },
                PositionToken {
                    token: .,
                    line: 0,
                    col: 10,
                },
                PositionToken {
                    token: Identifier("emilycares"),
                    line: 0,
                    col: 11,
                },
                PositionToken {
                    token: ;,
                    line: 0,
                    col: 21,
                },
                PositionToken {
                    token: import,
                    line: 2,
                    col: 0,
                },
                PositionToken {
                    token: Identifier("java"),
                    line: 2,
                    col: 7,
                },
                PositionToken {
                    token: .,
                    line: 2,
                    col: 11,
                },
                PositionToken {
                    token: Identifier("util"),
                    line: 2,
                    col: 12,
                },
                PositionToken {
                    token: .,
                    line: 2,
                    col: 16,
                },
                PositionToken {
                    token: Identifier("Collection"),
                    line: 2,
                    col: 17,
                },
                PositionToken {
                    token: ;,
                    line: 2,
                    col: 27,
                },
                PositionToken {
                    token: import,
                    line: 3,
                    col: 0,
                },
                PositionToken {
                    token: Identifier("java"),
                    line: 3,
                    col: 7,
                },
                PositionToken {
                    token: .,
                    line: 3,
                    col: 11,
                },
                PositionToken {
                    token: Identifier("util"),
                    line: 3,
                    col: 12,
                },
                PositionToken {
                    token: .,
                    line: 3,
                    col: 16,
                },
                PositionToken {
                    token: Identifier("List"),
                    line: 3,
                    col: 17,
                },
                PositionToken {
                    token: ;,
                    line: 3,
                    col: 21,
                },
                PositionToken {
                    token: import,
                    line: 5,
                    col: 0,
                },
                PositionToken {
                    token: Identifier("java"),
                    line: 5,
                    col: 7,
                },
                PositionToken {
                    token: .,
                    line: 5,
                    col: 11,
                },
                PositionToken {
                    token: Identifier("util"),
                    line: 5,
                    col: 12,
                },
                PositionToken {
                    token: .,
                    line: 5,
                    col: 16,
                },
                PositionToken {
                    token: Identifier("stream"),
                    line: 5,
                    col: 17,
                },
                PositionToken {
                    token: .,
                    line: 5,
                    col: 23,
                },
                PositionToken {
                    token: Identifier("Stream"),
                    line: 5,
                    col: 24,
                },
                PositionToken {
                    token: ;,
                    line: 5,
                    col: 30,
                },
                PositionToken {
                    token: import,
                    line: 6,
                    col: 0,
                },
                PositionToken {
                    token: Identifier("java"),
                    line: 6,
                    col: 7,
                },
                PositionToken {
                    token: .,
                    line: 6,
                    col: 11,
                },
                PositionToken {
                    token: Identifier("util"),
                    line: 6,
                    col: 12,
                },
                PositionToken {
                    token: .,
                    line: 6,
                    col: 16,
                },
                PositionToken {
                    token: Identifier("stream"),
                    line: 6,
                    col: 17,
                },
                PositionToken {
                    token: .,
                    line: 6,
                    col: 23,
                },
                PositionToken {
                    token: Identifier("StreamSupport"),
                    line: 6,
                    col: 24,
                },
                PositionToken {
                    token: ;,
                    line: 6,
                    col: 37,
                },
                PositionToken {
                    token: public,
                    line: 8,
                    col: 0,
                },
                PositionToken {
                    token: interface,
                    line: 8,
                    col: 7,
                },
                PositionToken {
                    token: Identifier("SuperInterface"),
                    line: 8,
                    col: 17,
                },
                PositionToken {
                    token: <,
                    line: 8,
                    col: 31,
                },
                PositionToken {
                    token: Identifier("E"),
                    line: 8,
                    col: 32,
                },
                PositionToken {
                    token: >,
                    line: 8,
                    col: 33,
                },
                PositionToken {
                    token: extends,
                    line: 8,
                    col: 35,
                },
                PositionToken {
                    token: Identifier("Collection"),
                    line: 8,
                    col: 43,
                },
                PositionToken {
                    token: ,,
                    line: 8,
                    col: 53,
                },
                PositionToken {
                    token: Identifier("List"),
                    line: 8,
                    col: 55,
                },
                PositionToken {
                    token: {,
                    line: 8,
                    col: 60,
                },
                PositionToken {
                    token: default,
                    line: 9,
                    col: 4,
                },
                PositionToken {
                    token: Identifier("Stream"),
                    line: 9,
                    col: 12,
                },
                PositionToken {
                    token: <,
                    line: 9,
                    col: 18,
                },
                PositionToken {
                    token: Identifier("E"),
                    line: 9,
                    col: 19,
                },
                PositionToken {
                    token: >,
                    line: 9,
                    col: 20,
                },
                PositionToken {
                    token: Identifier("stream"),
                    line: 9,
                    col: 22,
                },
                PositionToken {
                    token: (,
                    line: 9,
                    col: 28,
                },
                PositionToken {
                    token: ),
                    line: 9,
                    col: 29,
                },
                PositionToken {
                    token: {,
                    line: 9,
                    col: 31,
                },
                PositionToken {
                    token: return,
                    line: 10,
                    col: 8,
                },
                PositionToken {
                    token: Identifier("StreamSupport"),
                    line: 10,
                    col: 15,
                },
                PositionToken {
                    token: .,
                    line: 10,
                    col: 28,
                },
                PositionToken {
                    token: Identifier("stream"),
                    line: 10,
                    col: 29,
                },
                PositionToken {
                    token: (,
                    line: 10,
                    col: 35,
                },
                PositionToken {
                    token: Identifier("spliterator"),
                    line: 10,
                    col: 36,
                },
                PositionToken {
                    token: (,
                    line: 10,
                    col: 47,
                },
                PositionToken {
                    token: ),
                    line: 10,
                    col: 48,
                },
                PositionToken {
                    token: ,,
                    line: 10,
                    col: 49,
                },
                PositionToken {
                    token: false,
                    line: 10,
                    col: 51,
                },
                PositionToken {
                    token: ),
                    line: 10,
                    col: 56,
                },
                PositionToken {
                    token: ;,
                    line: 10,
                    col: 57,
                },
                PositionToken {
                    token: },
                    line: 11,
                    col: 4,
                },
                PositionToken {
                    token: },
                    line: 12,
                    col: 0,
                },
            ]
        "#]];
        expected.assert_debug_eq(&tokens);
    }

    #[test]
    fn everything() {
        let content = include_bytes!("../../parser/test/Everything.java");
        let tokens = lexer::lex(content).expect("Test");
        let expected = expect![[r#"
            [
                PositionToken {
                    token: package,
                    line: 0,
                    col: 0,
                },
                PositionToken {
                    token: Identifier("ch"),
                    line: 0,
                    col: 8,
                },
                PositionToken {
                    token: .,
                    line: 0,
                    col: 10,
                },
                PositionToken {
                    token: Identifier("emilycares"),
                    line: 0,
                    col: 11,
                },
                PositionToken {
                    token: ;,
                    line: 0,
                    col: 21,
                },
                PositionToken {
                    token: public,
                    line: 2,
                    col: 0,
                },
                PositionToken {
                    token: class,
                    line: 2,
                    col: 7,
                },
                PositionToken {
                    token: Identifier("Everything"),
                    line: 2,
                    col: 13,
                },
                PositionToken {
                    token: {,
                    line: 2,
                    col: 24,
                },
                PositionToken {
                    token: int,
                    line: 3,
                    col: 4,
                },
                PositionToken {
                    token: Identifier("noprop"),
                    line: 3,
                    col: 8,
                },
                PositionToken {
                    token: ;,
                    line: 3,
                    col: 14,
                },
                PositionToken {
                    token: public,
                    line: 5,
                    col: 4,
                },
                PositionToken {
                    token: Identifier("Everything"),
                    line: 5,
                    col: 11,
                },
                PositionToken {
                    token: (,
                    line: 5,
                    col: 21,
                },
                PositionToken {
                    token: ),
                    line: 5,
                    col: 22,
                },
                PositionToken {
                    token: {,
                    line: 5,
                    col: 24,
                },
                PositionToken {
                    token: },
                    line: 6,
                    col: 4,
                },
                PositionToken {
                    token: public,
                    line: 7,
                    col: 4,
                },
                PositionToken {
                    token: int,
                    line: 7,
                    col: 11,
                },
                PositionToken {
                    token: Identifier("publicproperty"),
                    line: 7,
                    col: 15,
                },
                PositionToken {
                    token: ;,
                    line: 7,
                    col: 29,
                },
                PositionToken {
                    token: private,
                    line: 8,
                    col: 4,
                },
                PositionToken {
                    token: int,
                    line: 8,
                    col: 12,
                },
                PositionToken {
                    token: Identifier("privateproperty"),
                    line: 8,
                    col: 16,
                },
                PositionToken {
                    token: ;,
                    line: 8,
                    col: 31,
                },
                PositionToken {
                    token: void,
                    line: 10,
                    col: 4,
                },
                PositionToken {
                    token: Identifier("method"),
                    line: 10,
                    col: 9,
                },
                PositionToken {
                    token: (,
                    line: 10,
                    col: 15,
                },
                PositionToken {
                    token: ),
                    line: 10,
                    col: 16,
                },
                PositionToken {
                    token: {,
                    line: 10,
                    col: 18,
                },
                PositionToken {
                    token: },
                    line: 11,
                    col: 4,
                },
                PositionToken {
                    token: public,
                    line: 13,
                    col: 4,
                },
                PositionToken {
                    token: void,
                    line: 13,
                    col: 11,
                },
                PositionToken {
                    token: Identifier("public_method"),
                    line: 13,
                    col: 16,
                },
                PositionToken {
                    token: (,
                    line: 13,
                    col: 29,
                },
                PositionToken {
                    token: ),
                    line: 13,
                    col: 30,
                },
                PositionToken {
                    token: {,
                    line: 13,
                    col: 32,
                },
                PositionToken {
                    token: },
                    line: 14,
                    col: 4,
                },
                PositionToken {
                    token: private,
                    line: 16,
                    col: 4,
                },
                PositionToken {
                    token: void,
                    line: 16,
                    col: 12,
                },
                PositionToken {
                    token: Identifier("private_method"),
                    line: 16,
                    col: 17,
                },
                PositionToken {
                    token: (,
                    line: 16,
                    col: 31,
                },
                PositionToken {
                    token: ),
                    line: 16,
                    col: 32,
                },
                PositionToken {
                    token: {,
                    line: 16,
                    col: 34,
                },
                PositionToken {
                    token: },
                    line: 17,
                    col: 4,
                },
                PositionToken {
                    token: int,
                    line: 19,
                    col: 4,
                },
                PositionToken {
                    token: Identifier("out"),
                    line: 19,
                    col: 8,
                },
                PositionToken {
                    token: (,
                    line: 19,
                    col: 11,
                },
                PositionToken {
                    token: ),
                    line: 19,
                    col: 12,
                },
                PositionToken {
                    token: {,
                    line: 19,
                    col: 14,
                },
                PositionToken {
                    token: return,
                    line: 20,
                    col: 8,
                },
                PositionToken {
                    token: Number(0),
                    line: 20,
                    col: 16,
                },
                PositionToken {
                    token: ;,
                    line: 20,
                    col: 16,
                },
                PositionToken {
                    token: },
                    line: 21,
                    col: 4,
                },
                PositionToken {
                    token: int,
                    line: 29,
                    col: 4,
                },
                PositionToken {
                    token: Identifier("add"),
                    line: 29,
                    col: 8,
                },
                PositionToken {
                    token: (,
                    line: 29,
                    col: 11,
                },
                PositionToken {
                    token: int,
                    line: 29,
                    col: 12,
                },
                PositionToken {
                    token: Identifier("a"),
                    line: 29,
                    col: 16,
                },
                PositionToken {
                    token: ,,
                    line: 29,
                    col: 17,
                },
                PositionToken {
                    token: int,
                    line: 29,
                    col: 19,
                },
                PositionToken {
                    token: Identifier("b"),
                    line: 29,
                    col: 23,
                },
                PositionToken {
                    token: ),
                    line: 29,
                    col: 24,
                },
                PositionToken {
                    token: {,
                    line: 29,
                    col: 26,
                },
                PositionToken {
                    token: return,
                    line: 30,
                    col: 8,
                },
                PositionToken {
                    token: Identifier("a"),
                    line: 30,
                    col: 15,
                },
                PositionToken {
                    token: +,
                    line: 30,
                    col: 17,
                },
                PositionToken {
                    token: Identifier("b"),
                    line: 30,
                    col: 19,
                },
                PositionToken {
                    token: ;,
                    line: 30,
                    col: 20,
                },
                PositionToken {
                    token: },
                    line: 31,
                    col: 4,
                },
                PositionToken {
                    token: static,
                    line: 33,
                    col: 4,
                },
                PositionToken {
                    token: int,
                    line: 33,
                    col: 11,
                },
                PositionToken {
                    token: Identifier("sadd"),
                    line: 33,
                    col: 15,
                },
                PositionToken {
                    token: (,
                    line: 33,
                    col: 19,
                },
                PositionToken {
                    token: int,
                    line: 33,
                    col: 20,
                },
                PositionToken {
                    token: Identifier("a"),
                    line: 33,
                    col: 24,
                },
                PositionToken {
                    token: ,,
                    line: 33,
                    col: 25,
                },
                PositionToken {
                    token: int,
                    line: 33,
                    col: 27,
                },
                PositionToken {
                    token: Identifier("b"),
                    line: 33,
                    col: 31,
                },
                PositionToken {
                    token: ),
                    line: 33,
                    col: 32,
                },
                PositionToken {
                    token: {,
                    line: 33,
                    col: 34,
                },
                PositionToken {
                    token: return,
                    line: 34,
                    col: 6,
                },
                PositionToken {
                    token: Identifier("a"),
                    line: 34,
                    col: 13,
                },
                PositionToken {
                    token: +,
                    line: 34,
                    col: 15,
                },
                PositionToken {
                    token: Identifier("b"),
                    line: 34,
                    col: 17,
                },
                PositionToken {
                    token: ;,
                    line: 34,
                    col: 18,
                },
                PositionToken {
                    token: },
                    line: 35,
                    col: 4,
                },
                PositionToken {
                    token: },
                    line: 36,
                    col: 0,
                },
            ]
        "#]];
        expected.assert_debug_eq(&tokens);
    }

    #[test]
    fn thrower() {
        let content = include_bytes!("../../parser/test/Thrower.java");
        let tokens = lexer::lex(content).expect("Test");
        let expected = expect![[r#"
            [
                PositionToken {
                    token: package,
                    line: 0,
                    col: 0,
                },
                PositionToken {
                    token: Identifier("ch"),
                    line: 0,
                    col: 8,
                },
                PositionToken {
                    token: .,
                    line: 0,
                    col: 10,
                },
                PositionToken {
                    token: Identifier("emilycares"),
                    line: 0,
                    col: 11,
                },
                PositionToken {
                    token: ;,
                    line: 0,
                    col: 21,
                },
                PositionToken {
                    token: import,
                    line: 2,
                    col: 0,
                },
                PositionToken {
                    token: Identifier("java"),
                    line: 2,
                    col: 7,
                },
                PositionToken {
                    token: .,
                    line: 2,
                    col: 11,
                },
                PositionToken {
                    token: Identifier("io"),
                    line: 2,
                    col: 12,
                },
                PositionToken {
                    token: .,
                    line: 2,
                    col: 14,
                },
                PositionToken {
                    token: Identifier("IOException"),
                    line: 2,
                    col: 15,
                },
                PositionToken {
                    token: ;,
                    line: 2,
                    col: 26,
                },
                PositionToken {
                    token: public,
                    line: 4,
                    col: 0,
                },
                PositionToken {
                    token: class,
                    line: 4,
                    col: 7,
                },
                PositionToken {
                    token: Identifier("Thrower"),
                    line: 4,
                    col: 13,
                },
                PositionToken {
                    token: {,
                    line: 4,
                    col: 21,
                },
                PositionToken {
                    token: public,
                    line: 5,
                    col: 2,
                },
                PositionToken {
                    token: void,
                    line: 5,
                    col: 9,
                },
                PositionToken {
                    token: Identifier("ioThrower"),
                    line: 5,
                    col: 14,
                },
                PositionToken {
                    token: (,
                    line: 5,
                    col: 23,
                },
                PositionToken {
                    token: ),
                    line: 5,
                    col: 24,
                },
                PositionToken {
                    token: throws,
                    line: 5,
                    col: 26,
                },
                PositionToken {
                    token: Identifier("IOException"),
                    line: 5,
                    col: 33,
                },
                PositionToken {
                    token: {,
                    line: 5,
                    col: 45,
                },
                PositionToken {
                    token: },
                    line: 5,
                    col: 46,
                },
                PositionToken {
                    token: public,
                    line: 6,
                    col: 2,
                },
                PositionToken {
                    token: void,
                    line: 6,
                    col: 9,
                },
                PositionToken {
                    token: Identifier("ioThrower"),
                    line: 6,
                    col: 14,
                },
                PositionToken {
                    token: (,
                    line: 6,
                    col: 23,
                },
                PositionToken {
                    token: int,
                    line: 6,
                    col: 24,
                },
                PositionToken {
                    token: Identifier("a"),
                    line: 6,
                    col: 28,
                },
                PositionToken {
                    token: ),
                    line: 6,
                    col: 29,
                },
                PositionToken {
                    token: throws,
                    line: 6,
                    col: 31,
                },
                PositionToken {
                    token: Identifier("IOException"),
                    line: 6,
                    col: 38,
                },
                PositionToken {
                    token: ,,
                    line: 6,
                    col: 49,
                },
                PositionToken {
                    token: Identifier("IOException"),
                    line: 6,
                    col: 51,
                },
                PositionToken {
                    token: {,
                    line: 6,
                    col: 63,
                },
                PositionToken {
                    token: },
                    line: 6,
                    col: 64,
                },
                PositionToken {
                    token: },
                    line: 7,
                    col: 0,
                },
            ]
        "#]];
        expected.assert_debug_eq(&tokens);
    }
    #[test]
    fn escaped_double_quetes() {
        let content = br#"return "\"" + s + "\"";"#;
        let tokens = lexer::lex(content).expect("Test");
        let expected = expect![[r#"
            [
                PositionToken {
                    token: return,
                    line: 0,
                    col: 0,
                },
                PositionToken {
                    token: String("\""),
                    line: 0,
                    col: 10,
                },
                PositionToken {
                    token: +,
                    line: 0,
                    col: 12,
                },
                PositionToken {
                    token: Identifier("s"),
                    line: 0,
                    col: 14,
                },
                PositionToken {
                    token: +,
                    line: 0,
                    col: 16,
                },
                PositionToken {
                    token: String("\""),
                    line: 0,
                    col: 21,
                },
                PositionToken {
                    token: ;,
                    line: 0,
                    col: 22,
                },
            ]
        "#]];
        expected.assert_debug_eq(&tokens);
    }
    #[test]
    fn escaped_backslash() {
        let content = br#" "\\" "#;
        let tokens = lexer::lex(content).expect("Test");
        let expected = expect![[r#"
            [
                PositionToken {
                    token: String("\\"),
                    line: 0,
                    col: 4,
                },
            ]
        "#]];
        expected.assert_debug_eq(&tokens);
    }
    #[test]
    fn escaped_others() {
        let content = br#" 
            '\b' + 
            '\t' + 
            '\n' + 
            '\f' + 
            '\r' + 
            '\"' +
            '\\' + 
         "#;
        let tokens = lexer::lex(content).expect("Test");
        let expected = expect![[r#"
            [
                PositionToken {
                    token: Char('\b'),
                    line: 1,
                    col: 14,
                },
                PositionToken {
                    token: +,
                    line: 1,
                    col: 16,
                },
                PositionToken {
                    token: Char('\t'),
                    line: 2,
                    col: 14,
                },
                PositionToken {
                    token: +,
                    line: 2,
                    col: 16,
                },
                PositionToken {
                    token: Char('\n'),
                    line: 3,
                    col: 14,
                },
                PositionToken {
                    token: +,
                    line: 3,
                    col: 16,
                },
                PositionToken {
                    token: Char('\f'),
                    line: 4,
                    col: 14,
                },
                PositionToken {
                    token: +,
                    line: 4,
                    col: 16,
                },
                PositionToken {
                    token: Char('\r'),
                    line: 5,
                    col: 14,
                },
                PositionToken {
                    token: +,
                    line: 5,
                    col: 16,
                },
                PositionToken {
                    token: Char('\"'),
                    line: 6,
                    col: 14,
                },
                PositionToken {
                    token: +,
                    line: 6,
                    col: 16,
                },
                PositionToken {
                    token: Char('\\'),
                    line: 7,
                    col: 14,
                },
                PositionToken {
                    token: +,
                    line: 7,
                    col: 16,
                },
            ]
        "#]];
        expected.assert_debug_eq(&tokens);
    }
}
