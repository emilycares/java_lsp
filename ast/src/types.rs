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
   #[derive(Debug, Clone)]
   pub struct AstThingAttributes: u8 {
        const Sealed       = 0b00000001;
        const NonSealed    = 0b00000010;
    }
}

bitflags! {
   #[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct AstRecord {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub attributes: AstThingAttributes,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub type_parameters: Option<AstTypeParameters>,
    pub record_entries: AstRecordEntries,
    pub superclass: AstSuperClass,
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
    pub jtype: AstJType,
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
   #[derive(PartialEq, Debug, Clone)]
   pub struct AstStaticFinal: u8 {
     const None      = 0b00000001;
     const Static    = 0b00000010;
     const Final     = 0b00000100;
     const Volatile  = 0b00001000;
     const Transient = 0b00010000;
   }
}

#[derive(Debug, Clone)]
pub struct AstClassVariable {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub expression: Option<AstExpression>,
    pub static_final: AstStaticFinal,
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
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub parameters: AstMethodParamerters,
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
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub parameters: AstMethodParamerters,
    pub throws: Option<AstThrowsDeclaration>,
    pub type_parameters: Option<AstTypeParameters>,
    pub annotated: Vec<AstAnnotated>,
}
#[derive(Debug, Clone)]
pub struct AstMethodParamerters {
    pub range: AstRange,
    pub parameters: Vec<AstMethodParamerter>,
}
#[derive(Debug, Clone)]
pub struct AstMethodParamerter {
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
    SwitchCaseArrow(AstSwitchCaseArrow),
    SwitchCaseArrowDefault(AstSwitchCaseArrowDefault),
    TryCatch(AstTryCatch),
    Throw(AstThrow),
    Yield(AstBlockYield),
    SynchronizedBlock(AstSynchronizedBlock),
    Thing(Box<AstThing>),
    Block(AstBlock),
}
impl AstBlockEntry {
    pub fn get_range(&self) -> AstRange {
        match &self {
            AstBlockEntry::Return(ast_block_return) => ast_block_return.range,
            AstBlockEntry::Variable(ast_block_variable) => {
                if let Some(first) = ast_block_variable.first()
                    && let Some(last) = ast_block_variable.last()
                {
                    return AstRange {
                        start: first.range.start,
                        end: last.range.end,
                    };
                }
                AstRange::default()
            }
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
            AstBlockEntry::SwitchCaseArrowDefault(ast_switch_case_arrow_default) => {
                ast_switch_case_arrow_default.range
            }
            AstBlockEntry::Thing(ast_thing) => ast_thing.get_range(),
            AstBlockEntry::Block(ast_block) => ast_block.range,
        }
    }
}
#[derive(Debug, Clone)]
pub struct AstWhile {
    pub range: AstRange,
    pub control: Box<AstRecursiveExpression>,
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
    pub check: Box<AstExpression>,
    pub block: AstBlock,
}
#[derive(Debug, Clone)]
pub struct AstSwitchCase {
    pub range: AstRange,
    pub expression: AstExpression,
}
#[derive(Debug, Clone)]
pub struct AstSwitchCaseArrow {
    pub range: AstRange,
    pub values: Vec<AstExpression>,
    pub content: Box<AstSwitchCaseArrowContent>,
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
    pub variable: AstBlockVariableMutliType,
    pub block: AstBlock,
}
#[derive(Debug, Clone)]
pub struct AstBlockAssign {
    pub range: AstRange,
    pub key: Box<AstRecursiveExpression>,
    pub expression: AstExpression,
}
#[derive(Debug, Clone)]
pub struct AstBlockExpression {
    pub range: AstRange,
    pub value: AstExpression,
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
pub struct AstBlockVariableMutliType {
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
    Expression(Box<AstExpression>),
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
pub struct AstBlockContinue {
    pub range: AstRange,
    pub label: Option<AstIdentifier>,
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

#[derive(Debug, Clone)]
pub struct AstInt {
    pub range: AstRange,
    pub value: SmolStr,
}
#[derive(Debug, Clone)]
pub struct AstDouble {
    pub range: AstRange,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub enum AstSuperClass {
    None,
    Name(AstIdentifier),
    ClassPath(AstIdentifier),
}
#[derive(Debug, Clone)]
/// Usage of a Annotation
pub struct AstAnnotated {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub parameters: Vec<AstAnnotatedParameter>,
}

#[derive(Debug, Clone)]
/// Definition of a new Annotation
pub struct AstAnnotation {
    pub range: AstRange,
    pub avaliability: AstAvailability,
    pub attributes: AstThingAttributes,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub fields: Vec<AstAnnotationField>,
}
#[derive(Debug, Clone)]
pub struct AstAnnotationField {
    pub range: AstRange,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub value: Option<AstValue>,
}

#[derive(Debug, Clone)]
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
impl AstThing {
    pub fn get_range(&self) -> AstRange {
        match self {
            AstThing::Class(ast_class) => ast_class.range,
            AstThing::Record(ast_record) => ast_record.range,
            AstThing::Interface(ast_interface) => ast_interface.range,
            AstThing::Enumeration(ast_enumeration) => ast_enumeration.range,
            AstThing::Annotation(ast_annotation) => ast_annotation.range,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AstJType {
    pub range: AstRange,
    pub value: AstJTypeKind,
}
#[derive(Debug, Clone)]
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
        base: Box<AstJType>,
        inner: Box<AstJType>,
    },
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
}
#[derive(Debug, Clone)]
pub enum AstExpression {
    Casted(AstCastedExpression),
    Recursive(Box<AstRecursiveExpression>),
    Lambda(AstLambda),
    InlineSwitch(AstSwitch),
    NewClass(AstNewClass),
    ClassAccess(AstClassAccess),
    Generics(AstGenerics),
    Array(AstValues),
    JType(AstCastedExpression),
}
impl AstExpression {
    pub fn has_content(&self) -> bool {
        match self {
            AstExpression::Recursive(ast_recursive_expression) => {
                ast_recursive_expression.has_content()
            }
            AstExpression::Casted(_)
            | AstExpression::JType(_)
            | AstExpression::Lambda(_)
            | AstExpression::InlineSwitch(_)
            | AstExpression::NewClass(_)
            | AstExpression::Array(_)
            | AstExpression::Generics(_)
            | AstExpression::ClassAccess(_) => true,
        }
    }
}
#[derive(Debug, Clone)]
pub struct AstCastedExpression {
    pub range: AstRange,
    pub cast: AstJType,
    pub expression: Box<AstExpression>,
}
#[derive(Debug, Clone)]
pub struct AstRecursiveExpression {
    pub range: AstRange,
    pub ident: Option<AstExpressionIdentifier>,
    pub values: Option<AstValues>,
    pub operator: AstExpressionOperator,
    pub instance_of: Option<AstJType>,
    pub next: Option<Box<AstExpression>>,
}
#[derive(Debug, Clone)]
pub enum AstExpressionIdentifier {
    Identifier(AstIdentifier),
    Nuget(AstValueNuget),
    Value(AstValue),
    ArrayAccess(Box<AstExpression>),
    EmptyArrayAccess,
}

#[derive(Debug, Clone)]
pub struct AstValues {
    pub range: AstRange,
    pub values: Vec<AstExpression>,
}
impl AstRecursiveExpression {
    pub fn has_content(&self) -> bool {
        self.ident.is_some()
            || self.instance_of.is_some()
            || self.next.is_some()
            || self.values.is_some()
            || self.operator != AstExpressionOperator::None
            || (matches!(self.operator, AstExpressionOperator::Colon(_)) && self.next.is_some())
            || (matches!(self.operator, AstExpressionOperator::QuestionMark(_))
                && self.next.is_some())
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
    Expr(Box<AstExpression>),
}

#[derive(Debug, Default, Clone)]
pub struct AstLambdaParameters {
    pub range: AstRange,
    pub values: Vec<AstIdentifier>,
}

#[derive(Debug, Clone)]
pub struct AstNewClass {
    pub range: AstRange,
    pub jtype: AstJType,
    pub rhs: Box<AstNewRhs>,
    pub next: Option<Box<AstExpression>>,
}
#[derive(Debug, Clone)]
pub struct AstClassAccess {
    pub range: AstRange,
    pub jtype: AstJType,
    pub next: Option<Box<AstExpression>>,
}
#[derive(Debug, Clone)]
pub struct AstGenerics {
    pub range: AstRange,
    pub jtype: AstJType,
    pub next: Option<Box<AstExpression>>,
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

#[derive(Debug, PartialEq, Clone)]
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
    pub name: AstIdentifier,
    pub supperclass: Option<AstSuperClass>,
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
    pub static_final: AstStaticFinal,
    pub avaliability: AstAvailability,
    pub name: AstIdentifier,
    pub jtype: AstJType,
    pub expression: AstExpression,
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
    pub avaliability: AstAvailability,
    pub attributes: AstThingAttributes,
    pub annotated: Vec<AstAnnotated>,
    pub name: AstIdentifier,
    pub implements: Vec<AstJType>,
    pub permits: Vec<AstJType>,
    pub superclass: AstSuperClass,
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
}
