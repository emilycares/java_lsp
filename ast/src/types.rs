#![allow(missing_docs)]
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
    PublicFinal,
    PublicStatic,
    PublicStaticFinal,
    Private,
    PrivateFinal,
    PrivateStatic,
    PrivateStaticFinal,
    Protected,
    ProtectedFinal,
    ProtectedStatic,
    ProtectedStaticFinal,
    Undefined,
    UndefinedFinal,
    UndefinedStatic,
    UndefinedStaticFinal,
}
impl AstAvailability {
    pub fn to_static(&self) -> Self {
        match self {
            AstAvailability::Public => Self::PublicStatic,
            AstAvailability::PublicStatic => Self::PublicStatic,
            AstAvailability::Private => Self::PrivateStatic,
            AstAvailability::PrivateStatic => Self::PrivateStatic,
            AstAvailability::Protected => Self::ProtectedStatic,
            AstAvailability::ProtectedStatic => Self::ProtectedStatic,
            AstAvailability::Undefined => Self::UndefinedStatic,
            AstAvailability::UndefinedStatic => Self::UndefinedStatic,
            AstAvailability::PublicFinal => Self::PublicStaticFinal,
            AstAvailability::PublicStaticFinal => Self::PublicStaticFinal,
            AstAvailability::PrivateFinal => Self::PublicStaticFinal,
            AstAvailability::PrivateStaticFinal => Self::PrivateStaticFinal,
            AstAvailability::ProtectedFinal => Self::ProtectedStaticFinal,
            AstAvailability::ProtectedStaticFinal => Self::ProtectedStaticFinal,
            AstAvailability::UndefinedFinal => Self::UndefinedStaticFinal,
            AstAvailability::UndefinedStaticFinal => Self::UndefinedStaticFinal,
        }
    }

    pub fn to_final(&self) -> AstAvailability {
        match self {
            AstAvailability::Public => Self::PublicFinal,
            AstAvailability::PublicStatic => Self::PublicStaticFinal,
            AstAvailability::Private => Self::PrivateFinal,
            AstAvailability::PrivateStatic => Self::PrivateStaticFinal,
            AstAvailability::Protected => Self::ProtectedFinal,
            AstAvailability::ProtectedStatic => Self::ProtectedStaticFinal,
            AstAvailability::Undefined => Self::UndefinedFinal,
            AstAvailability::UndefinedStatic => Self::UndefinedStaticFinal,
            AstAvailability::PublicFinal => Self::PublicFinal,
            AstAvailability::PublicStaticFinal => Self::PublicStaticFinal,
            AstAvailability::PrivateFinal => Self::PrivateFinal,
            AstAvailability::PrivateStaticFinal => Self::PrivateStaticFinal,
            AstAvailability::ProtectedFinal => Self::ProtectedFinal,
            AstAvailability::ProtectedStaticFinal => Self::ProtectedStaticFinal,
            AstAvailability::UndefinedFinal => Self::UndefinedFinal,
            AstAvailability::UndefinedStaticFinal => Self::UndefinedStaticFinal,
        }
    }
}

#[derive(Debug)]
pub struct AstClass {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub superclass: AstSuperClass,
    pub block: AstClassBlock,
}
#[derive(Debug)]
pub struct AstClassBlock {
    pub variables: Vec<AstClassVariable>,
    pub methods: Vec<AstClassMethod>,
    pub constructors: Vec<AstClassConstructor>,
}
#[derive(Debug, PartialEq)]
pub enum AstStaticFinal {
    None,
    Static,
    Final,
    StaticFinal,
}

#[derive(Debug)]
pub struct AstClassVariable {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub expression: Option<AstExpression>,
    pub static_final: AstStaticFinal,
}

#[derive(Debug)]
pub struct AstClassMethod {
    pub range: AstRange,
    pub annotated: Vec<AstAnnotated>,
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
    pub annotated: Vec<AstAnnotated>,
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
    For(Box<AstFor>),
    ForEnhanced(Box<AstForEnhanced>),
    Break(AstBlockBreak),
    Continue(AstBlockContinue),
    Switch(AstSwitch),
    SwitchCase(AstSwitchCase),
    SwitchDefault(AstSwitchDefault),
    TryCatch(AstTryCatch),
    Throw(AstThrow),
    SwitchCaseArrow(AstSwitchCaseArrow),
    Yield(AstBlockYield),
}
#[derive(Debug)]
pub struct AstWhile {
    pub range: AstRange,
    pub control: AstRecursiveExpression,
    pub block: AstBlock,
    pub lable: Option<AstIdentifier>,
}
#[derive(Debug)]
pub struct AstFor {
    pub range: AstRange,
    pub var: AstBlockVariable,
    pub check: AstRecursiveExpression,
    pub change: AstRecursiveExpression,
    pub block: AstBlock,
    pub lable: Option<AstIdentifier>,
}
#[derive(Debug)]
pub struct AstSwitch {
    pub range: AstRange,
    pub check: AstRecursiveExpression,
    pub block: AstBlock,
}
#[derive(Debug)]
pub struct AstSwitchCase {
    pub range: AstRange,
    pub value: AstValue,
}
#[derive(Debug)]
pub struct AstSwitchCaseArrow {
    pub range: AstRange,
    pub values: Vec<AstValue>,
    pub block: AstBlock,
}
#[derive(Debug)]
pub struct AstSwitchDefault {
    pub range: AstRange,
}
#[derive(Debug)]
pub struct AstForEnhanced {
    pub range: AstRange,
    pub var: AstBlockVariable,
    pub rhs: AstRecursiveExpression,
    pub block: AstBlock,
    pub lable: Option<AstIdentifier>,
}
#[derive(Debug)]
pub enum AstIf {
    If {
        range: AstRange,
        control: Box<AstRecursiveExpression>,
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
    Expression(Box<AstRecursiveExpression>),
    None,
}
#[derive(Debug)]
pub struct AstThrow {
    pub range: AstRange,
    pub value: AstNewClass,
}
#[derive(Debug)]
pub struct AstTryCatch {
    pub range: AstRange,
    pub resources_block: Option<AstBlock>,
    pub block: AstBlock,
    pub cases: Vec<AstTryCatchCase>,
    pub finally_block: Option<AstBlock>,
}
#[derive(Debug)]
pub struct AstTryCatchCase {
    pub range: AstRange,
    pub variable: AstBlockVariableMutliType,
    pub block: AstBlock,
}
#[derive(Debug)]
pub struct AstBlockAssign {
    pub range: AstRange,
    pub key: AstRecursiveExpression,
    pub expression: Option<AstExpression>,
}
#[derive(Debug)]
pub struct AstBlockExpression {
    pub range: AstRange,
    pub value: AstRecursiveExpression,
}
#[derive(Debug)]
pub struct AstBlock {
    pub range: AstRange,
    pub entries: Vec<AstBlockEntry>,
}
#[derive(Debug)]
pub struct AstBlockVariable {
    pub range: AstRange,
    pub fin: bool,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub expression: Option<AstExpression>,
}
#[derive(Debug)]
pub struct AstBlockVariableMutliType {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub jtypes: Vec<AstJType>,
    pub expression: Option<AstExpression>,
}

#[derive(Debug)]
pub struct AstBlockReturn {
    pub range: AstRange,
    pub expression: AstExpressionOrValue,
}

#[derive(Debug)]
pub enum AstExpressionOrValue {
    None,
    Expression(Box<AstExpression>),
    Value(AstValue),
}
#[derive(Debug)]
pub struct AstBlockYield {
    pub range: AstRange,
    pub expression: AstExpressionOrValue,
}
#[derive(Debug)]
pub struct AstBlockBreak {
    pub range: AstRange,
}
#[derive(Debug)]
pub struct AstBlockContinue {
    pub range: AstRange,
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
pub struct AstInt {
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
/// Usage of a Annotation
pub struct AstAnnotated {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub parameters: Vec<AstExpression>,
}

#[derive(Debug)]
/// Definition of a new Annotation
pub struct AstAnnotation {
    pub avaliability: AstAvailability,
    pub annotated: Vec<AstAnnotated>,
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
    pub annotated: Vec<AstAnnotated>,
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
    /// Untyped variable
    Var,
}

#[derive(Debug)]
pub enum AstValue {
    Variable(AstIdentifier),
    Nuget(AstValueNuget),
    Array(AstValues),
    NewClass(AstNewClass),
}
#[derive(Debug)]
pub enum AstValueNuget {
    Int(AstInt),
    Double(AstDouble),
    Float(AstDouble),
    StringLiteral(AstIdentifier),
    CharLiteral(AstIdentifier),
    BooleanLiteral(AstBoolean),
}
#[derive(Debug)]
pub enum AstExpression {
    Casted(AstCastedExpression),
    Recursive(AstRecursiveExpression),
    Lambda(AstLambda),
    InlineSwitch(AstSwitch),
    NewClass(AstNewClass),
}
#[derive(Debug)]
pub struct AstCastedExpression {
    pub range: AstRange,
    pub cast: AstJType,
    pub expression: AstRecursiveExpression,
}
#[derive(Debug)]
pub struct AstRecursiveExpression {
    pub range: AstRange,
    pub ident: Option<AstExpressionIdentifier>,
    pub values: Option<AstValues>,
    pub operator: AstExpressionOperator,
    pub next: Option<Box<AstRecursiveExpression>>,
}
#[derive(Debug)]
pub enum AstExpressionIdentifier {
    Identifier(AstIdentifier),
    Nuget(AstValueNuget),
    Value(AstValue),
    ArrayAccess(AstValue),
}

#[derive(Debug)]
pub struct AstValues {
    pub range: AstRange,
    pub values: Vec<AstExpression>,
}
impl AstRecursiveExpression {
    pub fn has_content(&self) -> bool {
        self.ident.is_some() || self.next.is_some() || self.values.is_some()
    }
}
#[derive(Debug)]
pub struct AstLambda {
    pub range: AstRange,
    pub parameters: AstLambdaParameters,
    pub rhs: AstLambdaRhs,
}

#[derive(Debug)]
pub enum AstLambdaRhs {
    None,
    Block(AstBlock),
    Expr(Box<AstExpression>),
}

#[derive(Debug, Default)]
pub struct AstLambdaParameters {
    pub range: AstRange,
    pub values: Vec<AstIdentifier>,
}

#[derive(Debug)]
pub struct AstNewClass {
    pub range: AstRange,
    pub jtype: AstJType,
    pub rhs: Box<AstNewRhs>,
}
#[derive(Debug)]
pub enum AstNewRhs {
    None,
    Parameters(Vec<AstExpression>),
    Block(AstClassBlock),
    ParametersAndBlock(Vec<AstExpression>, AstClassBlock),
    Array(AstValues),
}

#[derive(Debug)]
pub struct AstBoolean {
    pub range: AstRange,
    pub value: bool,
}

#[derive(Debug, PartialEq)]
pub enum AstExpressionOperator {
    None,
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
    Dot(AstRange),
    ExclemationMark(AstRange),
    Ampersand(AstRange),
    AmpersandAmpersand(AstRange),
    VerticalBar(AstRange),
    VerticalBarVerticalBar(AstRange),
    QuestionMark(AstRange),
    Colon(AstRange),
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
    pub annotated: Vec<AstAnnotated>,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub value: AstValue,
}
#[derive(Debug)]
pub struct AstInterfaceMethod {
    pub range: AstRange,
    pub annotated: Vec<AstAnnotated>,
    pub header: AstMethodHeader,
}
#[derive(Debug)]
pub struct AstInterfaceMethodDefault {
    pub range: AstRange,
    pub annotated: Vec<AstAnnotated>,
    pub header: AstMethodHeader,
    pub block: AstBlock,
}

#[derive(Debug)]
pub struct AstEnumeration {
    pub avaliability: AstAvailability,
    pub annotated: Vec<AstAnnotated>,
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
