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
    pub methods: Vec<AstClassMethod>,
    pub constructors: Vec<AstClassConstructor>,
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
pub struct AstClassMethod {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub parameters: AstMethodParamerters,
    pub block: AstBlock,
    pub stat: bool,
}
#[derive(Debug, PartialEq)]
pub struct AstClassConstructor {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub parameters: AstMethodParamerters,
    pub block: AstBlock,
}
#[derive(Debug, PartialEq)]
pub struct AstMethodParamerters {
    pub range: AstRange,
    pub parameters: Vec<AstMethodParamerter>,
}
#[derive(Debug, PartialEq)]
pub struct AstMethodParamerter {
    pub range: AstRange,
    pub jtype: AstJType,
    pub name: AstIdentifier,
}
#[derive(Debug, PartialEq)]
pub enum AstBlockEntry {
    Return(AstBlockReturn),
    Variable(AstBlockVariable),
    Expression(AstBlockExpression),
}
#[derive(Debug, PartialEq)]
pub struct AstBlockExpression {
    pub range: AstRange,
}
#[derive(Debug, PartialEq)]
pub struct AstBlock {
    pub range: AstRange,
    pub entries: Vec<AstBlockEntry>,
}
#[derive(Debug, PartialEq)]
pub struct AstBlockVariable {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub value: Option<AstValue>,
}

#[derive(Debug, PartialEq)]
pub struct AstBlockReturn {
    pub range: AstRange,
    pub value: AstValue,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AstIdentifier {
    pub range: AstRange,
    pub value: String,
}

#[derive(Debug, PartialEq)]
pub struct AstNumber {
    pub range: AstRange,
    pub value: i64,
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
    Equasion(AstValueEquasion),
    Nuget(AstValueNuget),
}

#[derive(Debug, PartialEq)]
pub struct AstValueNewClass {
    pub range: AstRange,
    pub jtype: AstJType,
    pub parameters: Vec<AstValue>,
}

#[derive(Debug, PartialEq)]
pub enum AstValueNuget {
    Variable(AstIdentifier),
    Number(AstNumber),
}

#[derive(Debug, PartialEq)]
pub struct AstValueEquasion {
    pub range: AstRange,
    pub lhs: AstValueNuget,
    pub operator: AstValueEquasionOperator,
    pub rhs: AstValueNuget,
}
#[derive(Debug, PartialEq)]
pub enum AstValueEquasionOperator {
    Plus(AstRange),
    Minus(AstRange),
}
