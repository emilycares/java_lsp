use smol_str::SmolStr;

use crate::lexer::PositionToken;

#[derive(Debug, PartialEq, Default, Clone, Copy)]
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
    pub fn is_after_range(&self, point: &AstPoint) -> bool {
        let after = AstPoint {
            line: self.end.line,
            col: self.end.col + 1,
        };
        point == &after
    }
}

#[derive(Debug, PartialEq, Default, Clone, Copy, PartialOrd)]
pub struct AstPoint {
    pub line: usize,
    pub col: usize,
}

impl AstPoint {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

#[derive(Debug)]
pub struct AstFile {
    pub package: AstIdentifier,
    pub imports: AstImports,
    pub thing: AstThing,
}
#[derive(Debug)]
pub struct AstImports {
    pub range: AstRange,
    pub imports: Vec<AstImport>,
}
#[derive(Debug)]
pub struct AstImport {
    pub range: AstRange,
    pub unit: AstImportUnit,
}

#[derive(Debug)]
pub enum AstImportUnit {
    Class(AstIdentifier),
    StaticClass(AstIdentifier),
    StaticClassMethod(AstIdentifier, AstIdentifier),
    Prefix(AstIdentifier),
    StaticPrefix(AstIdentifier),
}

#[derive(Debug)]
pub enum AstAvailability {
    Public,
    Private,
    Protected,
    Undefined,
}

#[derive(Debug)]
pub struct AstClass {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub superclass: AstSuperClass,
    pub variables: Vec<AstClassVariable>,
    pub methods: Vec<AstClassMethod>,
    pub constructors: Vec<AstClassConstructor>,
}

#[derive(Debug)]
pub struct AstClassVariable {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub expression: Option<AstExpression>,
    pub fin: bool,
}

#[derive(Debug)]
pub struct AstClassMethod {
    pub range: AstRange,
    pub header: AstMethodHeader,
    pub block: AstBlock,
}
#[derive(Debug)]
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
#[derive(Debug)]
pub struct AstThrowsDeclaration {
    pub range: AstRange,
    pub parameters: Vec<AstJType>,
}
#[derive(Debug)]
pub struct AstClassConstructor {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub parameters: AstMethodParamerters,
    pub block: AstBlock,
}
#[derive(Debug)]
pub struct AstMethodParamerters {
    pub range: AstRange,
    pub parameters: Vec<AstMethodParamerter>,
}
#[derive(Debug)]
pub struct AstMethodParamerter {
    pub range: AstRange,
    pub jtype: AstJType,
    pub name: AstIdentifier,
    pub fin: bool,
}
#[derive(Debug)]
pub enum AstBlockEntry {
    Return(AstBlockReturn),
    Variable(AstBlockVariable),
    Expression(AstBlockExpression),
    Assign(AstBlockAssign),
    If(AstIf),
    While(AstWhile),
    For(AstFor),
}
#[derive(Debug)]
pub struct AstWhile {
    pub range: AstRange,
    pub control: AstValue,
    pub block: AstBlock,
    pub lable: Option<AstIdentifier>,
}
#[derive(Debug)]
pub struct AstFor {
    pub range: AstRange,
    pub var: AstBlockVariable,
    pub check: AstExpression,
    pub change: AstExpression,
    pub block: AstBlock,
    pub lable: Option<AstIdentifier>,
}
#[derive(Debug)]
pub enum AstIf {
    If {
        range: AstRange,
        control: AstExpression,
        control_range: AstRange,
        content: AstIfContent,
        el: Option<Box<AstIf>>,
    },
    Else {
        range: AstRange,
        content: AstIfContent,
    },
}
#[derive(Debug)]
pub enum AstIfContent {
    Block(AstBlock),
    Expression(AstExpression),
    None,
}
#[derive(Debug)]
pub struct AstBlockAssign {
    pub range: AstRange,
    pub key: AstExpression,
    pub expression: AstExpression,
}
#[derive(Debug)]
pub struct AstBlockExpression {
    pub range: AstRange,
    pub value: AstExpression,
}
#[derive(Debug)]
pub struct AstBlock {
    pub range: AstRange,
    pub entries: Vec<Box<AstBlockEntry>>,
}
#[derive(Debug)]
pub struct AstBlockVariable {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub expression: Option<AstExpression>,
}

#[derive(Debug)]
pub struct AstBlockReturn {
    pub range: AstRange,
    pub expression: Option<AstExpression>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug)]
pub struct AstNumber {
    pub range: AstRange,
    pub value: i64,
}
#[derive(Debug)]
pub struct AstDouble {
    pub range: AstRange,
    pub value: f64,
}

#[derive(Debug)]
pub enum AstSuperClass {
    None,
    Name(AstIdentifier),
    ClassPath(AstIdentifier),
}
#[derive(Debug)]
pub struct AstAnnotation {
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub fields: Vec<AstAnnotationField>,
}
#[derive(Debug)]
pub struct AstAnnotationField {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub value: AstValue,
}

#[derive(Debug)]
pub struct AstInterface {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub type_parameters: Option<AstTypeParameters>,
    pub extends: Option<AstExtends>,
    pub constants: Vec<AstInterfaceConstant>,
    pub methods: Vec<AstInterfaceMethod>,
    pub default_methods: Vec<AstInterfaceMethodDefault>,
}

#[derive(Debug)]
pub enum AstThing {
    Class(AstClass),
    Interface(AstInterface),
    Enumeration(AstEnumeration),
    Annotation(AstAnnotation),
}

#[derive(Debug)]
pub struct AstJType {
    pub range: AstRange,
    pub value: AstJTypeKind,
}
#[derive(Debug)]
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

#[derive(Debug)]
pub enum AstValue {
    NewClass(AstValueNewClass),
    Variable(AstIdentifier),
    Nuget(AstValueNuget),
    Array(AstValues),
}
#[derive(Debug)]
pub enum AstValueNuget {
    Number(AstNumber),
    Double(AstDouble),
    Float(AstDouble),
    StringLiteral(AstIdentifier),
    CharLiteral(AstIdentifier),
    BooleanLiteral(AstBoolean),
}

#[derive(Debug)]
pub struct AstExpression {
    pub range: AstRange,
    pub ident: Option<AstExpressionIdentifier>,
    pub values: Option<AstValues>,
    pub next: Option<Box<AstExpression>>,
    pub operator: AstExpressionOperator,
}
#[derive(Debug)]
pub enum AstExpressionIdentifier {
    Identifier(AstIdentifier),
    Nuget(AstValueNuget),
    Value(Box<AstValue>),
    ArrayAccess(AstValue),
}

#[derive(Debug)]
pub struct AstValues {
    pub range: AstRange,
    pub values: Vec<AstExpression>,
}
impl AstExpression {
    pub fn has_content(&self) -> bool {
        self.ident.is_some() || self.next.is_some() || self.values.is_some()
    }
}

#[derive(Debug)]
pub struct AstValueNewClass {
    pub range: AstRange,
    pub jtype: AstJType,
    pub parameters: Vec<AstExpression>,
}

#[derive(Debug)]
pub struct AstBoolean {
    pub range: AstRange,
    pub value: bool,
}

#[derive(Debug, PartialEq)]
pub enum AstExpressionOperator {
    Plus(AstRange),
    PlusPlus(AstRange),
    Minus(AstRange),
    MinusMinus(AstRange),
    Equal(AstRange),
    NotEqual(AstRange),
    Multiply(AstRange),
    Devide(AstRange),
    Modulo(AstRange),
    Le(AstRange),
    Lt(AstRange),
    Ge(AstRange),
    Gt(AstRange),
    None,
    Dot(AstRange),
}

#[derive(Debug)]
pub struct AstTypeParameters {
    pub range: AstRange,
    pub parameters: Vec<AstIdentifier>,
}
#[derive(Debug)]
pub struct AstExtends {
    pub range: AstRange,
    pub parameters: Vec<AstJType>,
}
#[derive(Debug)]
pub struct AstInterfaceConstant {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub value: AstValue,
}
#[derive(Debug)]
pub struct AstInterfaceMethod {
    pub range: AstRange,
    pub header: AstMethodHeader,
}
#[derive(Debug)]
pub struct AstInterfaceMethodDefault {
    pub range: AstRange,
    pub header: AstMethodHeader,
    pub block: AstBlock,
}

#[derive(Debug)]
pub struct AstEnumeration {
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub variants: Vec<AstEnumerationVariant>,
    pub methods: Vec<AstClassMethod>,
    pub variables: Vec<AstClassVariable>,
    pub constructors: Vec<AstClassConstructor>,
}
#[derive(Debug)]
pub struct AstEnumerationVariant {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub parameters: Vec<AstExpression>,
}
