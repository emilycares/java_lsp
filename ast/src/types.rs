use smol_str::SmolStr;

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

    pub fn is_in_range(&self, point: &AstPoint) -> bool {
        let start = &self.start;
        let end = &self.end;

        if point >= start && point <= end {
            return true;
        }
        false
    }
}

#[derive(Debug, PartialEq, Default, Clone, PartialOrd)]
pub struct AstPoint {
    pub line: usize,
    pub col: usize,
}

impl AstPoint {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

#[derive(Debug, PartialEq)]
pub struct AstFile {
    pub package: AstIdentifier,
    pub imports: AstImports,
    pub thing: AstThing,
}
#[derive(Debug, PartialEq)]
pub struct AstImports {
    pub range: AstRange,
    pub imports: Vec<AstImport>,
}
#[derive(Debug, PartialEq)]
pub struct AstImport {
    pub range: AstRange,
    pub unit: AstImportUnit,
}

#[derive(Debug, PartialEq)]
pub enum AstImportUnit {
    Class(AstIdentifier),
    StaticClass(AstIdentifier),
    StaticClassMethod(AstIdentifier, AstIdentifier),
    Prefix(AstIdentifier),
    StaticPrefix(AstIdentifier),
}

#[derive(Debug, PartialEq)]
pub enum AstAvailability {
    Public,
    Private,
    Protected,
    Undefined,
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
    pub(crate) fin: bool,
}

#[derive(Debug, PartialEq)]
pub struct AstClassMethod {
    pub range: AstRange,
    pub header: AstMethodHeader,
    pub block: AstBlock,
}
#[derive(Debug, PartialEq)]
pub struct AstMethodHeader {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub parameters: AstMethodParamerters,
    pub stat: bool,
    pub throws: Option<AstThrowsDeclaration>,
    pub type_parameters: Option<AstTypeParameters>,
}
#[derive(Debug, PartialEq)]
pub struct AstThrowsDeclaration {
    pub range: AstRange,
    pub parameters: Vec<AstJType>,
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
    pub(crate) fin: bool,
}
#[derive(Debug, PartialEq)]
pub enum AstBlockEntry {
    Return(AstBlockReturn),
    Variable(AstBlockVariable),
    Expression(AstBlockExpression),
    Assign(AstBlockAssign),
}
#[derive(Debug, PartialEq)]
pub struct AstBlockAssign {
    pub range: AstRange,
    pub key: AstExpression,
    pub value: AstValue,
}
#[derive(Debug, PartialEq)]
pub struct AstBlockExpression {
    pub range: AstRange,
    pub value: AstExpression,
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
    pub value: SmolStr,
}

impl From<AstIdentifier> for String {
    fn from(value: AstIdentifier) -> Self {
        value.value.to_string()
    }
}

impl From<&AstIdentifier> for String {
    fn from(value: &AstIdentifier) -> Self {
        value.value.to_string()
    }
}
impl From<AstIdentifier> for SmolStr {
    fn from(value: AstIdentifier) -> Self {
        value.value
    }
}

impl From<&AstIdentifier> for SmolStr {
    fn from(value: &AstIdentifier) -> Self {
        value.value.clone()
    }
}

#[derive(Debug, PartialEq)]
pub struct AstNumber {
    pub range: AstRange,
    pub value: i64,
}
#[derive(Debug, PartialEq)]
pub struct AstDouble {
    pub range: AstRange,
    pub value: f64,
}

#[derive(Debug, PartialEq)]
pub enum AstSuperClass {
    None,
    Name(AstIdentifier),
    ClassPath(AstIdentifier),
}
#[derive(Debug, PartialEq)]
pub struct AstAnnotation {
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub fields: Vec<AstAnnotationField>,
}
#[derive(Debug, PartialEq)]
pub struct AstAnnotationField {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub value: AstValue,
}

#[derive(Debug, PartialEq)]
pub struct AstInterface {
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub type_parameters: Option<AstTypeParameters>,
    pub extends: Option<AstExtends>,
    pub constants: Vec<AstInterfaceConstant>,
    pub methods: Vec<AstInterfaceMethod>,
    pub default_methods: Vec<AstInterfaceMethodDefault>,
}

#[derive(Debug, PartialEq)]
pub enum AstThing {
    Class(AstClass),
    Interface(AstInterface),
    Enumeration(AstEnumeration),
    Annotation(AstAnnotation),
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
    Variable(AstIdentifier),
    Number(AstNumber),
    Double(AstDouble),
    Float(AstDouble),
    StringLiteral(AstIdentifier),
    CharLiteral(AstIdentifier),
    BooleanLiteral(AstBoolean),
    Expression(AstExpression),
}

#[derive(Debug, PartialEq)]
pub struct AstExpression {
    pub range: AstRange,
    pub ident: Option<AstIdentifier>,
    pub values: Option<Vec<AstValue>>,
    pub next: Option<Box<AstExpression>>,
}
impl AstExpression {
    pub fn has_content(&self) -> bool {
        self.ident.is_some() || self.next.is_some() || self.values.is_some()
    }
}

#[derive(Debug, PartialEq)]
pub struct AstValueNewClass {
    pub range: AstRange,
    pub jtype: AstJType,
    pub parameters: Vec<AstValue>,
}

#[derive(Debug, PartialEq)]
pub struct AstBoolean {
    pub range: AstRange,
    pub value: bool,
}

#[derive(Debug, PartialEq)]
pub struct AstValueEquasion {
    pub range: AstRange,
    pub lhs: Box<AstValue>,
    pub operator: AstValueEquasionOperator,
    pub rhs: Box<AstValue>,
}
#[derive(Debug, PartialEq)]
pub enum AstValueEquasionOperator {
    Plus(AstRange),
    Minus(AstRange),
}

#[derive(Debug, PartialEq)]
pub struct AstTypeParameters {
    pub range: AstRange,
    pub parameters: Vec<AstIdentifier>,
}
#[derive(Debug, PartialEq)]
pub struct AstExtends {
    pub range: AstRange,
    pub parameters: Vec<AstJType>,
}
#[derive(Debug, PartialEq)]
pub struct AstInterfaceConstant {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub value: AstValue,
}
#[derive(Debug, PartialEq)]
pub struct AstInterfaceMethod {
    pub range: AstRange,
    pub header: AstMethodHeader,
}
#[derive(Debug, PartialEq)]
pub struct AstInterfaceMethodDefault {
    pub range: AstRange,
    pub header: AstMethodHeader,
    pub block: AstBlock,
}

#[derive(Debug, PartialEq)]
pub struct AstEnumeration {
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub variants: Vec<AstEnumerationVariant>,
    pub methods: Vec<AstClassMethod>,
    pub variables: Vec<AstClassVariable>,
    pub constructors: Vec<AstClassConstructor>,
}
#[derive(Debug, PartialEq)]
pub struct AstEnumerationVariant {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub parameters: Vec<AstValue>,
}
