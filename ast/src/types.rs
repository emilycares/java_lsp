#![allow(missing_docs)]

use bitflags::bitflags;
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

bitflags! {
   #[derive(Debug)]
   pub struct AstThingAttributes: u8 {
        const Sealed       = 0b00000001;
        const NonSealed    = 0b00000010;
    }
}

bitflags! {
   #[derive(Debug)]
   pub struct AstAvailability: u8 {
        const Public       = 0b00000001;
        const Synchronized = 0b00000010;
        const Final        = 0b00000100;
        const Static       = 0b00001000;
        const Private      = 0b00010000;
        const Protected    = 0b00100000;
        const Abstract     = 0b01000000;
        const Native       = 0b10000000;
    }
}

#[derive(Debug)]
pub struct AstClass {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub attributes: AstThingAttributes,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub type_parameters: Option<AstTypeParameters>,
    pub superclass: AstSuperClass,
    pub implements: Vec<AstJType>,
    pub permits: Vec<AstJType>,
    pub block: AstClassBlock,
}
#[derive(Debug)]
pub struct AstRecord {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub attributes: AstThingAttributes,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub record_entries: AstRecordEntries,
    pub superclass: AstSuperClass,
    pub implements: Vec<AstJType>,
    pub block: AstClassBlock,
}
#[derive(Debug)]
pub struct AstRecordEntries {
    pub range: AstRange,
    pub entries: Vec<AstRecordEntry>,
}
#[derive(Debug)]
pub struct AstRecordEntry {
    pub jtype: AstJType,
    pub name: AstIdentifier,
}
#[derive(Debug)]
pub struct AstClassBlock {
    pub variables: Vec<AstClassVariable>,
    pub methods: Vec<AstClassMethod>,
    pub constructors: Vec<AstClassConstructor>,
    pub static_blocks: Vec<AstStaticBlock>,
    pub inner: Vec<AstThing>,
}
bitflags! {
   #[derive(PartialEq, Debug)]
   pub struct AstStaticFinal: u8 {
     const None      = 0b00000001;
     const Static    = 0b00000010;
     const Final     = 0b00000100;
     const Volatile  = 0b00001000;
     const Transient = 0b00010000;
   }
}

#[derive(Debug)]
pub struct AstClassVariable {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub annotated: Vec<AstAnnotated>,
    pub names: Vec<AstIdentifier>,
    pub jtype: AstJType,
    pub expression: Option<AstExpression>,
    pub static_final: AstStaticFinal,
}

#[derive(Debug)]
pub struct AstClassMethod {
    pub range: AstRange,
    pub header: AstMethodHeader,
    pub block: Option<AstBlock>,
}
#[derive(Debug)]
pub struct AstStaticBlock {
    pub range: AstRange,
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
    pub annotated: Vec<AstAnnotated>,
}
#[derive(Debug)]
pub struct AstThrowsDeclaration {
    pub range: AstRange,
    pub parameters: Vec<AstJType>,
}
#[derive(Debug)]
pub struct AstClassConstructor {
    pub range: AstRange,
    pub header: AstConstructorHeader,
    pub block: AstBlock,
}
#[derive(Debug)]
pub struct AstConstructorHeader {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub parameters: AstMethodParamerters,
    pub throws: Option<AstThrowsDeclaration>,
    pub type_parameters: Option<AstTypeParameters>,
    pub annotated: Vec<AstAnnotated>,
}
#[derive(Debug)]
pub struct AstMethodParamerters {
    pub range: AstRange,
    pub parameters: Vec<AstMethodParamerter>,
}
#[derive(Debug)]
pub struct AstMethodParamerter {
    pub range: AstRange,
    pub annotated: Vec<AstAnnotated>,
    pub jtype: AstJType,
    pub name: AstIdentifier,
    pub fin: bool,
    pub variatic: bool,
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
    SynchronizedBlock(AstSynchronizedBlock),
}
impl AstBlockEntry {
    pub fn get_range(&self) -> AstRange {
        match &self {
            AstBlockEntry::Return(ast_block_return) => ast_block_return.range,
            AstBlockEntry::Variable(ast_block_variable) => ast_block_variable.range,
            AstBlockEntry::Expression(ast_block_expression) => ast_block_expression.range,
            AstBlockEntry::Assign(ast_block_assign) => ast_block_assign.range,
            AstBlockEntry::If(ast_if) => match ast_if {
                AstIf::If {
                    range,
                    control: _,
                    control_range: _,
                    content: _,
                    el: _,
                } => *range,
                AstIf::Else { range, content: _ } => *range,
            },
            AstBlockEntry::While(ast_while) => ast_while.range,
            AstBlockEntry::For(ast_for) => ast_for.range,
            AstBlockEntry::ForEnhanced(ast_for_enhanced) => ast_for_enhanced.range,
            AstBlockEntry::Break(ast_block_break) => ast_block_break.range,
            AstBlockEntry::Continue(ast_block_continue) => ast_block_continue.range,
            AstBlockEntry::Switch(ast_switch) => ast_switch.range,
            AstBlockEntry::SwitchCase(ast_switch_case) => ast_switch_case.range,
            AstBlockEntry::SwitchDefault(ast_switch_default) => ast_switch_default.range,
            AstBlockEntry::TryCatch(ast_try_catch) => ast_try_catch.range,
            AstBlockEntry::Throw(ast_throw) => ast_throw.range,
            AstBlockEntry::SwitchCaseArrow(ast_switch_case_arrow) => ast_switch_case_arrow.range,
            AstBlockEntry::Yield(ast_block_yield) => ast_block_yield.range,
            AstBlockEntry::SynchronizedBlock(ast_synchronized_block) => {
                ast_synchronized_block.range
            }
        }
    }
}
#[derive(Debug)]
pub struct AstWhile {
    pub range: AstRange,
    pub control: AstRecursiveExpression,
    pub content: AstWhileContent,
    pub lable: Option<AstIdentifier>,
}
#[derive(Debug)]
pub struct AstFor {
    pub range: AstRange,
    pub vars: Vec<AstBlockEntry>,
    pub check: AstExpression,
    pub changes: Vec<AstBlockEntry>,
    pub content: AstForContent,
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
    pub expression: AstExpression,
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
    pub rhs: AstExpression,
    pub content: AstForContent,
    pub lable: Option<AstIdentifier>,
}
#[derive(Debug)]
pub enum AstIf {
    If {
        range: AstRange,
        control: Box<AstExpression>,
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
pub enum AstWhileContent {
    Block(AstBlock),
    BlockEntry(Box<AstBlockEntry>),
    None,
}
#[derive(Debug)]
pub enum AstIfContent {
    Block(AstBlock),
    BlockEntry(Box<AstBlockEntry>),
    None,
}
#[derive(Debug)]
pub enum AstForContent {
    Block(AstBlock),
    BlockEntry(Box<AstBlockEntry>),
    None,
}
#[derive(Debug)]
pub struct AstThrow {
    pub range: AstRange,
    pub expression: AstExpression,
}
#[derive(Debug)]
pub struct AstSynchronizedBlock {
    pub range: AstRange,
    pub expression: AstExpression,
    pub block: AstBlock,
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
    pub entries: Vec<AstBlockEntry>,
}
#[derive(Debug)]
pub struct AstBlockVariable {
    pub range: AstRange,
    pub fin: bool,
    pub annotated: Vec<AstAnnotated>,
    pub names: Vec<AstIdentifier>,
    pub jtype: AstJType,
    pub expression: Option<AstExpression>,
}
#[derive(Debug)]
pub struct AstBlockVariableMutliType {
    pub range: AstRange,
    pub fin: bool,
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
    pub parameters: Vec<AstAnnotatedParameter>,
}

#[derive(Debug)]
/// Definition of a new Annotation
pub struct AstAnnotation {
    pub avaliability: AstAvailability,
    pub attributes: AstThingAttributes,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub fields: Vec<AstAnnotationField>,
}
#[derive(Debug)]
pub struct AstAnnotationField {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub value: Option<AstValue>,
}

#[derive(Debug)]
pub struct AstInterface {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub attributes: AstThingAttributes,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub type_parameters: Option<AstTypeParameters>,
    pub extends: Option<AstExtends>,
    pub constants: Vec<AstInterfaceConstant>,
    pub methods: Vec<AstInterfaceMethod>,
    pub default_methods: Vec<AstInterfaceMethodDefault>,
    pub inner: Vec<AstThing>,
}

#[derive(Debug)]
pub enum AstThing {
    Class(AstClass),
    Record(AstRecord),
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
    Access {
        ident: AstIdentifier,
        inner: Box<AstJType>,
    },
}

#[derive(Debug)]
pub enum AstValue {
    Variable(AstIdentifier),
    Nuget(AstValueNuget),
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
    ClassAccess(AstClassAccess),
    Generics(AstGenerics),
    Array(AstValues),
}
impl AstExpression {
    pub fn has_content(&self) -> bool {
        match self {
            AstExpression::Recursive(ast_recursive_expression) => {
                ast_recursive_expression.has_content()
            }
            AstExpression::Casted(_)
            | AstExpression::Lambda(_)
            | AstExpression::InlineSwitch(_)
            | AstExpression::NewClass(_)
            | AstExpression::Array(_)
            | AstExpression::Generics(_)
            | AstExpression::ClassAccess(_) => true,
        }
    }
}
#[derive(Debug)]
pub struct AstCastedExpression {
    pub range: AstRange,
    pub cast: AstJType,
    pub expression: Box<AstExpression>,
}
#[derive(Debug)]
pub struct AstRecursiveExpression {
    pub range: AstRange,
    pub ident: Option<AstExpressionIdentifier>,
    pub values: Option<AstValues>,
    pub operator: AstExpressionOperator,
    pub instance_of: Option<AstJType>,
    pub next: Option<Box<AstExpression>>,
}
#[derive(Debug)]
pub enum AstExpressionIdentifier {
    Identifier(AstIdentifier),
    Nuget(AstValueNuget),
    Value(AstValue),
    ArrayAccess(Box<AstRecursiveExpression>),
}

#[derive(Debug)]
pub struct AstValues {
    pub range: AstRange,
    pub values: Vec<AstExpression>,
}
impl AstRecursiveExpression {
    pub fn has_content(&self) -> bool {
        self.ident.is_some()
            || self.next.is_some()
            || self.values.is_some()
            || self.operator != AstExpressionOperator::None
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
    pub next: Option<Box<AstExpression>>,
}
#[derive(Debug)]
pub struct AstClassAccess {
    pub range: AstRange,
    pub jtype: AstJType,
    pub next: Option<Box<AstExpression>>,
}
#[derive(Debug)]
pub struct AstGenerics {
    pub range: AstRange,
    pub jtype: AstJType,
    pub next: Option<Box<AstExpression>>,
}
#[derive(Debug)]
pub enum AstNewRhs {
    None,
    ArrayParameters(Vec<Vec<AstExpression>>),
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
    Assign(AstRange),
    Tilde(AstRange),
    Caret(AstRange),
}

#[derive(Debug)]
pub struct AstTypeParameters {
    pub range: AstRange,
    pub parameters: Vec<AstTypeParameter>,
}
#[derive(Debug)]
pub struct AstTypeParameter {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub supperclass: Option<AstSuperClass>,
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
    pub static_final: AstStaticFinal,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub expression: AstExpression,
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
    pub attributes: AstThingAttributes,
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

#[derive(Debug)]
pub enum AstAnnotatedParameter {
    Expression(AstExpression),
    NamedExpression {
        range: AstRange,
        name: AstIdentifier,
        expression: AstExpression,
    },
}
