use crate::lexer::PositionToken;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct AstRange {
    pub start: AstPoint,
    pub end: AstPoint,
}

impl AstRange {
    pub fn from_position_token(start: &PositionToken, end: &PositionToken) -> Self {
        Self {
            start: start.start_point(),
            end: end.end_point(),
        }
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct AstPoint {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, PartialEq)]
pub struct AstFile {
    pub package: AstIdentifier,
    pub imports: Vec<AstIdentifier>,
    pub thing: AstThing,
}
#[derive(Debug, PartialEq)]
pub enum AstAvailability {
    Public,
    Private,
    Protected,
}

#[derive(Debug, PartialEq)]
pub struct AstClass {
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub superclass: AstSuperClass,
    pub variables: Vec<AstClassVariable>,
}

#[derive(Debug, PartialEq)]
pub struct AstClassVariable {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub value: Option<AstValue>,
}
#[derive(Debug, PartialEq)]
pub struct AstIdentifier {
    pub range: AstRange,
    pub value: String,
}

#[derive(Debug, PartialEq)]
pub enum AstSuperClass {
    None,
    Name(AstIdentifier),
    ClassPath(AstIdentifier),
}

#[derive(Debug, PartialEq)]
pub struct AstInterface {
    pub avaliability: AstAvailability,
}

#[derive(Debug, PartialEq)]
pub enum AstThing {
    Class(AstClass),
    Interface(AstInterface),
    None,
}

#[derive(Debug, PartialEq)]
pub struct AstJType {
    pub range: AstRange,
    pub value: AstJTypeKind,
}
#[derive(Debug, PartialEq)]
pub enum AstJTypeKind {
    Void,
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Short,
    Boolean,
    Wildcard,
    Class(AstIdentifier),
    Array(Box<AstJType>),
    Generic(AstIdentifier, Vec<AstJType>),
    Parameter(AstIdentifier),
}

#[derive(Debug, PartialEq)]
pub enum AstValue {
    NewClass(AstValueNewClass),
}

#[derive(Debug, PartialEq)]
pub struct AstValueNewClass {
    pub range: AstRange,
    pub jtype: AstJType,
    pub parameters: Vec<AstValue>,
}
