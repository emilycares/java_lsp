#![allow(missing_docs)]

use core::fmt;
use std::fmt::Debug;

use bitflags::bitflags;
use my_string::MyString;

use crate::lexer::PositionToken;

#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub struct AstRange {
    pub start: AstPoint,
    pub end: AstPoint,
}

impl AstRange {
    #[must_use]
    pub fn from_position_token(start: &PositionToken, end: &PositionToken) -> Self {
        Self {
            start: start.start_point(),
            end: end.end_point(),
        }
    }

    #[must_use]
    pub fn is_in_range(&self, point: &AstPoint) -> bool {
        point >= &self.start && point <= &self.end
    }
    #[must_use]
    pub fn is_contained_in(&self, other: &Self) -> bool {
        self.start >= other.start && self.end <= other.end
    }
    #[must_use]
    pub fn is_after_range(&self, point: &AstPoint) -> bool {
        let after = AstPoint {
            line: self.end.line,
            col: self.end.col + 1,
        };
        point == &after
    }
}

#[derive(Debug, PartialEq, Eq, Default, Clone, Copy, PartialOrd)]
pub struct AstPoint {
    pub line: usize,
    pub col: usize,
}

impl AstPoint {
    #[must_use]
    pub const fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

#[derive(Debug)]
pub struct AstFile {
    pub package: Option<AstPackage>,
    pub imports: Option<AstImports>,
    pub things: Vec<AstThing>,
    pub modules: Vec<AstModule>,
}
#[derive(Debug)]
pub struct AstPackage {
    pub range: AstRange,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
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

bitflags! {
   #[derive(Debug, Clone)]
   pub struct AstThingAttributes: u8 {
        const Sealed       = 0b0000_0001;
        const NonSealed    = 0b0000_0010;
    }
}

bitflags! {
   #[derive(Debug, Clone)]
   pub struct AstAvailability: u16 {
        const Public       = 0b0000_0000_0000_0001;
        const Synchronized = 0b0000_0000_0000_0010;
        const Final        = 0b0000_0000_0000_0100;
        const Static       = 0b0000_0000_0000_1000;
        const Private      = 0b0000_0000_0001_0000;
        const Protected    = 0b0000_0000_0010_0000;
        const Abstract     = 0b0000_0000_0100_0000;
        const Native       = 0b0000_0000_1000_0000;
        const StaticFp     = 0b0000_0001_0000_0000;
    }
}
#[derive(Debug, Clone)]
pub struct AstModule {
    pub range: AstRange,
    pub annotated: Vec<AstAnnotated>,
    pub open: bool,
    pub name: AstIdentifier,
    pub exports: Vec<AstModuleExports>,
    pub opens: Vec<AstModuleOpens>,
    pub uses: Vec<AstModuleUses>,
    pub provides: Vec<AstModuleProvides>,
    pub requires: Vec<AstModuleRequires>,
}
#[derive(Debug, Clone)]
pub struct AstModuleExports {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub to: Vec<AstIdentifier>,
}
#[derive(Debug, Clone)]
pub struct AstModuleOpens {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub to: Vec<AstIdentifier>,
}
#[derive(Debug, Clone)]
pub struct AstModuleUses {
    pub range: AstRange,
    pub name: AstIdentifier,
}
#[derive(Debug, Clone)]
pub struct AstModuleRequires {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub flags: AstModuleRequiresFlags,
}
bitflags! {
   #[derive(Debug, Clone)]
   pub struct AstModuleRequiresFlags: u8 {
        const Transitive   = 0b0000_0001;
        const Static       = 0b0000_0010;
    }
}
#[derive(Debug, Clone)]
pub struct AstModuleProvides {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub with: Vec<AstIdentifier>,
}

#[derive(Debug, Clone)]
pub struct AstClass {
    pub range: AstRange,
    pub availability: AstAvailability,
    pub attributes: AstThingAttributes,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub type_parameters: Option<AstTypeParameters>,
    pub superclass: Vec<AstSuperClass>,
    pub implements: Vec<AstJType>,
    pub permits: Vec<AstJType>,
    pub block: AstClassBlock,
}
#[derive(Debug, Clone)]
pub struct AstRecord {
    pub range: AstRange,
    pub availability: AstAvailability,
    pub attributes: AstThingAttributes,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub type_parameters: Option<AstTypeParameters>,
    pub record_entries: AstRecordEntries,
    pub superclass: Vec<AstSuperClass>,
    pub implements: Vec<AstJType>,
    pub block: AstClassBlock,
}
#[derive(Debug, Clone)]
pub struct AstRecordEntries {
    pub range: AstRange,
    pub entries: Vec<AstRecordEntry>,
}
#[derive(Debug, Clone)]
pub struct AstRecordEntry {
    pub annotated: Vec<AstAnnotated>,
    pub jtype: AstJType,
    pub variadic: bool,
    pub name: AstIdentifier,
}
#[derive(Debug, Clone)]
pub struct AstClassBlock {
    pub variables: Vec<AstClassVariable>,
    pub methods: Vec<AstClassMethod>,
    pub constructors: Vec<AstClassConstructor>,
    pub static_blocks: Vec<AstStaticBlock>,
    pub inner: Vec<AstThing>,
    pub blocks: Vec<AstBlock>,
}
bitflags! {
   #[derive(PartialEq, Eq, Debug, Clone)]
   pub struct AstVolatileTransient: u8 {
     const Volatile  = 0b0000_0001;
     const Transient = 0b0000_0010;
   }
}

#[derive(Debug, Clone)]
pub struct AstClassVariable {
    pub range: AstRange,
    pub availability: AstAvailability,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub expression: Option<AstExpression>,
    pub volatile_transient: AstVolatileTransient,
}

#[derive(Debug, Clone)]
pub struct AstClassMethod {
    pub range: AstRange,
    pub header: AstMethodHeader,
    pub block: Option<AstBlock>,
}
#[derive(Debug, Clone)]
pub struct AstStaticBlock {
    pub range: AstRange,
    pub block: AstBlock,
}
#[derive(Debug, Clone)]
pub struct AstMethodHeader {
    pub range: AstRange,
    pub availability: AstAvailability,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub parameters: AstMethodParameters,
    pub throws: Option<AstThrowsDeclaration>,
    pub type_parameters: Option<AstTypeParameters>,
    pub annotated: Vec<AstAnnotated>,
}
#[derive(Debug, Clone)]
pub struct AstThrowsDeclaration {
    pub range: AstRange,
    pub parameters: Vec<AstJType>,
}
#[derive(Debug, Clone)]
pub struct AstClassConstructor {
    pub range: AstRange,
    pub header: AstConstructorHeader,
    pub block: AstBlock,
}
#[derive(Debug, Clone)]
pub struct AstConstructorHeader {
    pub range: AstRange,
    pub availability: AstAvailability,
    pub name: AstIdentifier,
    pub parameters: AstMethodParameters,
    pub throws: Option<AstThrowsDeclaration>,
    pub type_parameters: Option<AstTypeParameters>,
    pub annotated: Vec<AstAnnotated>,
}
#[derive(Debug, Clone)]
pub struct AstMethodParameters {
    pub range: AstRange,
    pub parameters: Vec<AstMethodParameter>,
}
#[derive(Debug, Clone)]
pub struct AstMethodParameter {
    pub range: AstRange,
    pub annotated: Vec<AstAnnotated>,
    pub jtype: AstJType,
    pub name: AstIdentifier,
    pub fin: bool,
    pub variatic: bool,
}
#[derive(Debug, Clone)]
pub enum AstBlockEntry {
    Return(AstBlockReturn),
    Variable(Vec<AstBlockVariable>),
    Expression(AstBlockExpression),
    Assign(Box<AstBlockAssign>),
    If(AstIf),
    While(AstWhile),
    For(Box<AstFor>),
    ForEnhanced(Box<AstForEnhanced>),
    Break(AstBlockBreak),
    Continue(AstBlockContinue),
    Switch(AstSwitch),
    SwitchCase(AstSwitchCase),
    SwitchDefault(AstSwitchDefault),
    SwitchCaseArrowValues(AstSwitchCaseArrowValues),
    SwitchCaseArrowType(AstSwitchCaseArrowType),
    SwitchCaseArrowDefault(AstSwitchCaseArrowDefault),
    TryCatch(AstTryCatch),
    Throw(AstThrow),
    Yield(AstBlockYield),
    SynchronizedBlock(AstSynchronizedBlock),
    Thing(Box<AstThing>),
    InlineBlock(AstInlineBlock),
    Semicolon(AstRange),
    Assert(AstBlockAssert),
}

#[derive(Debug, Clone)]
pub struct AstWhile {
    pub range: AstRange,
    pub control: AstExpression,
    pub content: AstWhileContent,
    pub label: Option<AstIdentifier>,
}
#[derive(Debug, Clone)]
pub struct AstFor {
    pub range: AstRange,
    pub vars: Vec<AstBlockEntry>,
    pub check: Vec<AstBlockEntry>,
    pub changes: Vec<AstBlockEntry>,
    pub content: AstForContent,
    pub label: Option<AstIdentifier>,
}
#[derive(Debug, Clone)]
pub struct AstSwitch {
    pub range: AstRange,
    pub check: AstExpression,
    pub block: AstBlock,
}
#[derive(Debug, Clone)]
pub struct AstSwitchCase {
    pub range: AstRange,
    pub expressions: Vec<AstExpressionOrDefault>,
}
#[derive(Debug, Clone)]
pub enum AstExpressionOrDefault {
    Default,
    Expression(AstExpression),
}
#[derive(Debug, Clone)]
pub struct AstSwitchCaseArrowValues {
    pub range: AstRange,
    pub values: Vec<AstExpressionOrDefault>,
    pub content: Box<AstSwitchCaseArrowContent>,
}
#[derive(Debug, Clone)]
pub struct AstSwitchCaseArrowType {
    pub range: AstRange,
    pub var: AstSwitchCaseArrowVar,
    pub content: Box<AstSwitchCaseArrowContent>,
}
#[derive(Debug, Clone)]
pub struct AstSwitchCaseArrowVar {
    pub range: AstRange,
    pub jtype: AstJType,
    pub name: AstIdentifier,
}
#[derive(Debug, Clone)]
pub struct AstSwitchCaseArrowDefault {
    pub range: AstRange,
    pub content: Box<AstSwitchCaseArrowContent>,
}
#[derive(Debug, Clone)]
pub enum AstSwitchCaseArrowContent {
    Block(AstBlock),
    Entry(Box<AstBlockEntry>),
}
#[derive(Debug, Clone)]
pub struct AstSwitchDefault {
    pub range: AstRange,
}
#[derive(Debug, Clone)]
pub struct AstForEnhanced {
    pub range: AstRange,
    pub var: Vec<AstBlockVariable>,
    pub rhs: AstExpression,
    pub content: AstForContent,
    pub label: Option<AstIdentifier>,
}
#[derive(Debug, Clone)]
pub enum AstIf {
    If {
        range: AstRange,
        control: AstExpression,
        control_range: AstRange,
        content: AstIfContent,
    },
    ElseIf {
        range: AstRange,
        control: AstExpression,
        control_range: AstRange,
        content: AstIfContent,
    },
    Else {
        range: AstRange,
        content: AstIfContent,
    },
}
#[derive(Debug, Clone)]
pub enum AstWhileContent {
    None,
    Block(AstBlock),
    BlockEntry(Box<AstBlockEntry>),
}
#[derive(Debug, Clone)]
pub enum AstIfContent {
    Block(AstBlock),
    BlockEntry(Box<AstBlockEntry>),
}
#[derive(Debug, Clone)]
pub enum AstForContent {
    None,
    Block(AstBlock),
    BlockEntry(Box<AstBlockEntry>),
}
#[derive(Debug, Clone)]
pub struct AstThrow {
    pub range: AstRange,
    pub expression: AstExpression,
}
#[derive(Debug, Clone)]
pub struct AstSynchronizedBlock {
    pub range: AstRange,
    pub expression: AstExpression,
    pub block: AstBlock,
}
#[derive(Debug, Clone)]
pub struct AstTryCatch {
    pub range: AstRange,
    pub resources_block: Option<AstBlock>,
    pub block: AstBlock,
    pub cases: Vec<AstTryCatchCase>,
    pub finally_block: Option<AstBlock>,
}
#[derive(Debug, Clone)]
pub struct AstTryCatchCase {
    pub range: AstRange,
    pub variable: AstBlockVariableMultiType,
    pub block: AstBlock,
}
#[derive(Debug, Clone)]
pub struct AstBlockAssign {
    pub range: AstRange,
    pub key: AstExpression,
    pub expression: AstExpression,
}
#[derive(Debug, Clone)]
pub struct AstBlockExpression {
    pub range: AstRange,
    pub value: AstExpression,
}
#[derive(Debug, Clone)]
pub struct AstInlineBlock {
    pub range: AstRange,
    pub label: Option<AstIdentifier>,
    pub block: AstBlock,
}
#[derive(Debug, Clone)]
pub struct AstBlock {
    pub range: AstRange,
    pub entries: Vec<AstBlockEntry>,
}
#[derive(Debug, Clone)]
pub struct AstBlockVariable {
    pub range: AstRange,
    pub fin: bool,
    pub annotated: Vec<AstAnnotated>,
    pub jtype: AstJType,
    pub name: AstIdentifier,
    pub value: Option<AstExpression>,
}

#[derive(Debug, Clone)]
pub struct AstBlockVariableMultiType {
    pub range: AstRange,
    pub fin: bool,
    pub name: AstIdentifier,
    pub jtypes: Vec<AstJType>,
    pub expression: Option<AstExpression>,
}

#[derive(Debug, Clone)]
pub struct AstBlockReturn {
    pub range: AstRange,
    pub expression: AstExpressionOrValue,
}

#[derive(Debug, Clone)]
pub enum AstExpressionOrValue {
    None,
    Expression(AstExpression),
    Value(AstValue),
}
#[derive(Debug, Clone)]
pub struct AstBlockYield {
    pub range: AstRange,
    pub expression: AstExpressionOrValue,
}
#[derive(Debug, Clone)]
pub struct AstBlockBreak {
    pub range: AstRange,
    pub label: Option<AstIdentifier>,
}
#[derive(Debug, Clone)]
pub struct AstBlockAssert {
    pub range: AstRange,
    pub expression: AstExpression,
}
#[derive(Debug, Clone)]
pub struct AstBlockContinue {
    pub range: AstRange,
    pub label: Option<AstIdentifier>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AstIdentifier {
    pub range: AstRange,
    pub value: MyString,
}

impl From<AstIdentifier> for MyString {
    fn from(value: AstIdentifier) -> Self {
        value.value
    }
}

impl From<&AstIdentifier> for MyString {
    fn from(value: &AstIdentifier) -> Self {
        value.value.clone()
    }
}
// impl From<AstIdentifier> for MyString {
//     fn from(value: AstIdentifier) -> Self {
//         value.value
//     }
// }

// impl From<&AstIdentifier> for MyString {
//     fn from(value: &AstIdentifier) -> Self {
//         value.value.clone()
//     }
// }

#[derive(Debug, Clone)]
pub struct AstInt {
    pub range: AstRange,
    pub value: MyString,
}
#[derive(Debug, Clone)]
pub struct AstHexLiteral {
    pub range: AstRange,
    pub value: MyString,
}
#[derive(Debug, Clone)]
pub struct AstBinaryLiteral {
    pub range: AstRange,
    pub value: MyString,
}
#[derive(Debug, Clone)]
pub struct AstDouble {
    pub range: AstRange,
    pub value: MyString,
}

#[derive(Debug, Clone)]
pub enum AstSuperClass {
    None,
    Name(AstIdentifier),
}
#[derive(Debug, Clone)]
/// Usage of a Annotation
pub struct AstAnnotated {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub parameters: AstAnnotatedParameterKind,
}
#[derive(Debug, Clone)]
pub enum AstAnnotatedParameterKind {
    None,
    Parameter(Vec<AstAnnotatedParameter>),
    Array(AstValuesWithAnnotated),
}

/// Definition of a new Annotation
#[derive(Debug, Clone)]
pub struct AstAnnotation {
    pub range: AstRange,
    pub availability: AstAvailability,
    pub attributes: AstThingAttributes,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub fields: Vec<AstAnnotationField>,
    pub inner: Vec<AstThing>,
}
#[derive(Debug, Clone)]
pub struct AstAnnotationField {
    pub range: AstRange,
    pub availability: AstAvailability,
    pub annotated: Vec<AstAnnotated>,
    pub jtype: AstJType,
    pub name: AstIdentifier,
    pub expression: Option<AstExpression>,
}

#[derive(Debug, Clone)]
pub struct AstInterface {
    pub range: AstRange,
    pub availability: AstAvailability,
    pub attributes: AstThingAttributes,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub type_parameters: Option<AstTypeParameters>,
    pub extends: Option<AstExtends>,
    pub constants: Vec<AstInterfaceConstant>,
    pub methods: Vec<AstInterfaceMethod>,
    pub default_methods: Vec<AstInterfaceMethodDefault>,
    pub inner: Vec<AstThing>,
    pub permits: Vec<AstJType>,
}

#[derive(Debug, Clone)]
pub enum AstThing {
    Class(AstClass),
    Record(AstRecord),
    Interface(AstInterface),
    Enumeration(AstEnumeration),
    Annotation(AstAnnotation),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AstJType {
    pub range: AstRange,
    pub value: AstJTypeKind,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub enum AstJTypeKind {
    #[default]
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
    /// Untyped variable
    Var,
    Access {
        base: Box<AstJType>,
        inner: Box<AstJType>,
    },
}
impl AstJTypeKind {
    #[must_use]
    pub fn is_array(&self) -> bool {
        let mut c = self;
        while let Self::Access { base: _, inner } = &c {
            c = &inner.value;
        }

        if let Self::Array(_) = c {
            return true;
        }
        false
    }
}
impl fmt::Display for AstJTypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Void => write!(f, "void"),
            Self::Byte => write!(f, "byte"),
            Self::Char => write!(f, "char"),
            Self::Double => write!(f, "double"),
            Self::Float => write!(f, "float"),
            Self::Int => write!(f, "int"),
            Self::Long => write!(f, "long"),
            Self::Short => write!(f, "short"),
            Self::Boolean => write!(f, "boolean"),
            Self::Wildcard => write!(f, "?"),
            Self::Var => write!(f, "var"),
            Self::Class(ast_identifier) => write!(f, "{}", ast_identifier.value),
            Self::Array(ast_jtype) => {
                std::fmt::Display::fmt(&ast_jtype.value, f)?;
                write!(f, "[]")
            }
            Self::Generic(ast_identifier, ast_jtypes) => {
                write!(f, "{}", ast_identifier.value)?;
                write!(f, "<")?;
                for t in ast_jtypes {
                    t.fmt(f)?;
                    write!(f, ", ")?;
                }

                write!(f, ">")
            }
            Self::Access { base, inner } => {
                fmt::Display::fmt(&base.value, f)?;
                write!(f, ".")?;
                fmt::Display::fmt(&inner.value, f)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum AstValue {
    Variable(AstIdentifier),
    Nuget(AstValueNuget),
}
#[derive(Debug, Clone)]
pub enum AstValueNuget {
    Int(AstInt),
    Double(AstDouble),
    Float(AstDouble),
    StringLiteral(AstIdentifier),
    CharLiteral(AstIdentifier),
    BooleanLiteral(AstBoolean),
    HexLiteral(AstHexLiteral),
    BinaryLiteral(AstBinaryLiteral),
}
pub type AstExpression = Vec<AstExpressionKind>;

#[derive(Debug, Clone)]
pub enum AstExpressionKind {
    Casted(AstCastedExpression),
    Recursive(AstRecursiveExpression),
    Lambda(AstLambda),
    InlineSwitch(AstSwitch),
    NewClass(AstNewClass),
    Generics(AstGenerics),
    Array(AstValues),
    JType(AstCastedExpression),
    InstanceOf(AstInstanceOf),
}
impl AstExpressionKind {
    #[must_use]
    pub fn has_content(&self) -> bool {
        match self {
            Self::Recursive(ast_recursive_expression) => ast_recursive_expression.has_content(),
            Self::Casted(_)
            | Self::JType(_)
            | Self::Lambda(_)
            | Self::InlineSwitch(_)
            | Self::NewClass(_)
            | Self::Array(_)
            | Self::Generics(_)
            | Self::InstanceOf(_) => true,
        }
    }
}
#[derive(Debug, Clone)]
pub struct AstCastedExpression {
    pub range: AstRange,
    pub cast: AstJType,
}
#[derive(Debug, Clone)]
pub struct AstInstanceOf {
    pub range: AstRange,
    pub annotated: Vec<AstAnnotated>,
    pub availability: AstAvailability,
    pub jtype: AstJType,
}
#[derive(Debug, Clone)]
pub struct AstRecursiveExpression {
    pub range: AstRange,
    pub ident: Option<AstExpressionIdentifier>,
    pub values: Option<AstValues>,
    pub operator: AstExpressionOperator,
}
#[derive(Debug, Clone)]
pub enum AstExpressionIdentifier {
    Identifier(AstIdentifier),
    Nuget(AstValueNuget),
    Value(AstValue),
    ArrayAccess(AstExpression),
    EmptyArrayAccess,
}

#[derive(Debug, Clone)]
pub struct AstValues {
    pub range: AstRange,
    pub values: Vec<AstExpression>,
}
#[derive(Debug, Clone)]
pub struct AstValuesWithAnnotated {
    pub range: AstRange,
    pub values: Vec<AstExpressionOrAnnotated>,
}
#[derive(Debug, Clone)]
pub enum AstExpressionOrAnnotated {
    Expression(AstExpression),
    Annotated(AstAnnotated),
}
impl AstRecursiveExpression {
    #[must_use]
    pub fn has_content(&self) -> bool {
        self.ident.is_some()
            || self.values.is_some()
            || self.operator != AstExpressionOperator::None
    }
}
#[derive(Debug, Clone)]
pub struct AstLambda {
    pub range: AstRange,
    pub parameters: AstLambdaParameters,
    pub rhs: AstLambdaRhs,
}

#[derive(Debug, Clone)]
pub enum AstLambdaRhs {
    None,
    Block(AstBlock),
    Expr(AstExpression),
}

#[derive(Debug, Default, Clone)]
pub struct AstLambdaParameters {
    pub range: AstRange,
    pub values: Vec<AstLambdaParameter>,
}
#[derive(Debug, Default, Clone)]
pub struct AstLambdaParameter {
    pub range: AstRange,
    pub jtype: Option<AstJType>,
    pub name: AstIdentifier,
}

#[derive(Debug, Clone)]
pub struct AstNewClass {
    pub range: AstRange,
    pub jtype: AstJType,
    pub rhs: Box<AstNewRhs>,
}
#[derive(Debug, Clone)]
pub struct AstGenerics {
    pub range: AstRange,
    pub jtypes: Vec<AstJType>,
}
#[derive(Debug, Clone)]
pub enum AstNewRhs {
    None,
    ArrayParameters(Vec<Vec<AstExpression>>),
    Parameters(Vec<AstExpression>),
    Block(AstClassBlock),
    ParametersAndBlock(Vec<AstExpression>, AstClassBlock),
    Array(AstValues),
}

#[derive(Debug, Clone)]
pub struct AstBoolean {
    pub range: AstRange,
    pub value: bool,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AstExpressionOperator {
    None,
    Plus(AstRange),
    PlusPlus(AstRange),
    Minus(AstRange),
    MinusMinus(AstRange),
    Equal(AstRange),
    NotEqual(AstRange),
    Multiply(AstRange),
    Divide(AstRange),
    Modulo(AstRange),
    Le(AstRange),
    Lt(AstRange),
    Ge(AstRange),
    Gt(AstRange),
    Dot(AstRange),
    ExclamationMark(AstRange),
    Ampersand(AstRange),
    AmpersandAmpersand(AstRange),
    VerticalBar(AstRange),
    VerticalBarVerticalBar(AstRange),
    QuestionMark(AstRange),
    Colon(AstRange),
    ColonColon(AstRange),
    Assign(AstRange),
    Tilde(AstRange),
    Caret(AstRange),
}

#[derive(Debug, Clone)]
pub struct AstTypeParameters {
    pub range: AstRange,
    pub parameters: Vec<AstTypeParameter>,
}
#[derive(Debug, Clone)]
pub struct AstTypeParameter {
    pub range: AstRange,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub supperclass: Option<Vec<AstSuperClass>>,
}
#[derive(Debug, Clone)]
pub struct AstExtends {
    pub range: AstRange,
    pub parameters: Vec<AstJType>,
}
#[derive(Debug, Clone)]
pub struct AstInterfaceConstant {
    pub range: AstRange,
    pub annotated: Vec<AstAnnotated>,
    pub availability: AstAvailability,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub expression: Option<AstExpression>,
}
#[derive(Debug, Clone)]
pub struct AstInterfaceMethod {
    pub range: AstRange,
    pub annotated: Vec<AstAnnotated>,
    pub header: AstMethodHeader,
}
#[derive(Debug, Clone)]
pub struct AstInterfaceMethodDefault {
    pub range: AstRange,
    pub annotated: Vec<AstAnnotated>,
    pub header: AstMethodHeader,
    pub block: AstBlock,
}

#[derive(Debug, Clone)]
pub struct AstEnumeration {
    pub range: AstRange,
    pub availability: AstAvailability,
    pub attributes: AstThingAttributes,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub implements: Vec<AstJType>,
    pub permits: Vec<AstJType>,
    pub superclass: Vec<AstSuperClass>,
    pub variants: Vec<AstEnumerationVariant>,
    pub methods: Vec<AstClassMethod>,
    pub variables: Vec<AstClassVariable>,
    pub constructors: Vec<AstClassConstructor>,
    pub static_blocks: Vec<AstStaticBlock>,
    pub inner: Vec<AstThing>,
}
#[derive(Debug, Clone)]
pub struct AstEnumerationVariant {
    pub range: AstRange,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub parameters: Vec<AstExpression>,
}

#[derive(Debug, Clone)]
pub enum AstAnnotatedParameter {
    Expression(AstExpression),
    NamedExpression {
        range: AstRange,
        name: AstIdentifier,
        expression: AstExpression,
    },
    Annotated(AstAnnotated),
    NamedArray {
        range: AstRange,
        name: AstIdentifier,
        values: AstValuesWithAnnotated,
    },
}
