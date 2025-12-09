//! Range type and methods
use crate::types::{
    AstAnnotated, AstAnnotatedParameter, AstAnnotation, AstBlockEntry, AstClass, AstClassMethod,
    AstEnumeration, AstExpression, AstExpressionIdentifier, AstExpressionKind, AstForContent,
    AstIf, AstIfContent, AstInterface, AstPoint, AstRange, AstRecord, AstSuperClass, AstThing,
    AstValue, AstValueNuget, AstValues,
};

/// Join two ranges a must be before b
#[must_use]
pub fn add_ranges(a: AstRange, b: AstRange) -> AstRange {
    debug_assert!(a.end < b.start);
    AstRange {
        start: a.start,
        end: b.end,
    }
}
/// Helper to check if ranged type is after of point
pub trait AstAfterRange {
    /// if ranged type is after of point
    fn is_after_range(&self, point: &AstPoint) -> bool;
}
/// Helper to check if ranged type is inside of point
pub trait AstInRange {
    /// if ranged type is inside of point
    fn is_in_range(&self, point: &AstPoint) -> bool;
}
/// Return range
pub trait GetRange {
    /// Return range
    fn get_range(&self) -> AstRange;
}

impl AstInRange for &AstThing {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            AstThing::Class(ast_class) => ast_class.is_in_range(point),
            AstThing::Record(ast_record) => ast_record.is_in_range(point),
            AstThing::Interface(ast_interface) => ast_interface.is_in_range(point),
            AstThing::Enumeration(ast_enumeration) => ast_enumeration.is_in_range(point),
            AstThing::Annotation(ast_annotation) => ast_annotation.is_in_range(point),
        }
    }
}
impl AstInRange for &AstClass {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        self.range.is_in_range(point)
    }
}
impl AstInRange for &AstRecord {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        self.range.is_in_range(point)
    }
}
impl AstInRange for &AstInterface {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        self.range.is_in_range(point)
    }
}
impl AstInRange for &AstEnumeration {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        self.range.is_in_range(point)
    }
}
impl AstInRange for &AstAnnotation {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        self.range.is_in_range(point)
    }
}

impl AstInRange for &AstClassMethod {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        self.range.is_in_range(point)
    }
}

impl AstInRange for &AstBlockEntry {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            AstBlockEntry::Return(ast_block_return) => ast_block_return.range.is_in_range(point),
            AstBlockEntry::Variable(ast_block_variable) => {
                for v in ast_block_variable {
                    if v.range.is_in_range(point) {
                        return true;
                    }
                }
                false
            }
            AstBlockEntry::Expression(ast_block_expression) => {
                ast_block_expression.range.is_in_range(point)
            }
            AstBlockEntry::Assign(ast_block_assign) => ast_block_assign.range.is_in_range(point),
            AstBlockEntry::If(ast_if) => ast_if.is_in_range(point),
            AstBlockEntry::While(ast_while) => ast_while.range.is_in_range(point),
            AstBlockEntry::For(ast_while) => ast_while.range.is_in_range(point),
            AstBlockEntry::ForEnhanced(ast_for_enhanced) => {
                ast_for_enhanced.range.is_in_range(point)
            }
            AstBlockEntry::Break(ast_block_break) => ast_block_break.range.is_in_range(point),
            AstBlockEntry::Continue(ast_block_continue) => {
                ast_block_continue.range.is_in_range(point)
            }
            AstBlockEntry::Switch(ast_switch) => ast_switch.range.is_in_range(point),
            AstBlockEntry::Assert(assert) => assert.range.is_in_range(point),
            AstBlockEntry::SwitchCase(ast_switch_case) => ast_switch_case.range.is_in_range(point),
            AstBlockEntry::SwitchCaseArrowType(ast_switch_case_arrow_type) => {
                ast_switch_case_arrow_type.range.is_in_range(point)
            }
            AstBlockEntry::SwitchDefault(ast_switch_case) => {
                ast_switch_case.range.is_in_range(point)
            }
            AstBlockEntry::TryCatch(ast_try_catch) => ast_try_catch.range.is_in_range(point),
            AstBlockEntry::Throw(ast_throw) => ast_throw.range.is_in_range(point),
            AstBlockEntry::SwitchCaseArrowValues(ast_switch_case_arrow) => {
                ast_switch_case_arrow.range.is_in_range(point)
            }
            AstBlockEntry::Yield(ast_block_yield) => ast_block_yield.range.is_in_range(point),
            AstBlockEntry::SynchronizedBlock(ast_synchronized_block) => {
                ast_synchronized_block.range.is_in_range(point)
            }
            AstBlockEntry::SwitchCaseArrowDefault(ast_switch_case_arrow_default) => {
                ast_switch_case_arrow_default.range.is_in_range(point)
            }
            AstBlockEntry::Thing(ast_thing) => ast_thing.get_range().is_in_range(point),
            AstBlockEntry::InlineBlock(ast_block) => ast_block.range.is_in_range(point),
            AstBlockEntry::Semicolon(ast_range) => ast_range.is_in_range(point),
        }
    }
}
impl AstInRange for &AstIf {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            AstIf::If {
                range,
                control: _,
                control_range: _,
                content: _,
            }
            | AstIf::Else { range, content: _ }
            | AstIf::ElseIf {
                range,
                control: _,
                control_range: _,
                content: _,
            } => range.is_in_range(point),
        }
    }
}
impl AstInRange for &AstIfContent {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            AstIfContent::Block(ast_block) => ast_block.range.is_in_range(point),
            AstIfContent::BlockEntry(ast_block_entry) => (&**ast_block_entry).is_in_range(point),
        }
    }
}
impl AstInRange for &AstForContent {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            AstForContent::Block(ast_block) => ast_block.range.is_in_range(point),
            AstForContent::BlockEntry(ast_block_entry) => (&**ast_block_entry).is_in_range(point),
            AstForContent::None => false,
        }
    }
}
impl AstInRange for &AstValue {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            AstValue::Variable(ast_identifier) => ast_identifier.range.is_in_range(point),
            AstValue::Nuget(ast_value_nuget) => ast_value_nuget.is_in_range(point),
        }
    }
}

impl AstAfterRange for &AstValue {
    fn is_after_range(&self, point: &AstPoint) -> bool {
        match self {
            AstValue::Variable(ast_identifier) => ast_identifier.range.is_after_range(point),
            AstValue::Nuget(ast_value_nuget) => ast_value_nuget.is_after_range(point),
        }
    }
}
impl AstInRange for &AstValueNuget {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            AstValueNuget::Int(ast_number) => ast_number.range.is_in_range(point),
            AstValueNuget::HexLiteral(h) => h.range.is_in_range(point),
            AstValueNuget::BinaryLiteral(b) => b.range.is_in_range(point),
            AstValueNuget::Double(ast_double) | AstValueNuget::Float(ast_double) => {
                ast_double.range.is_in_range(point)
            }
            AstValueNuget::StringLiteral(ast_identifier)
            | AstValueNuget::CharLiteral(ast_identifier) => ast_identifier.range.is_in_range(point),
            AstValueNuget::BooleanLiteral(ast_boolean) => ast_boolean.range.is_in_range(point),
        }
    }
}
impl GetRange for &AstValueNuget {
    fn get_range(&self) -> AstRange {
        match self {
            AstValueNuget::Int(ast_number) => ast_number.range,
            AstValueNuget::HexLiteral(h) => h.range,
            AstValueNuget::BinaryLiteral(b) => b.range,
            AstValueNuget::Double(ast_double) | AstValueNuget::Float(ast_double) => {
                ast_double.range
            }
            AstValueNuget::StringLiteral(ast_identifier)
            | AstValueNuget::CharLiteral(ast_identifier) => ast_identifier.range,
            AstValueNuget::BooleanLiteral(ast_boolean) => ast_boolean.range,
        }
    }
}
impl AstAfterRange for &AstValueNuget {
    fn is_after_range(&self, point: &AstPoint) -> bool {
        match self {
            AstValueNuget::Int(ast_number) => ast_number.range.is_after_range(point),
            AstValueNuget::HexLiteral(h) => h.range.is_after_range(point),
            AstValueNuget::BinaryLiteral(b) => b.range.is_after_range(point),
            AstValueNuget::Double(ast_double) | AstValueNuget::Float(ast_double) => {
                ast_double.range.is_after_range(point)
            }
            AstValueNuget::StringLiteral(ast_identifier)
            | AstValueNuget::CharLiteral(ast_identifier) => {
                ast_identifier.range.is_after_range(point)
            }
            AstValueNuget::BooleanLiteral(ast_boolean) => ast_boolean.range.is_after_range(point),
        }
    }
}
impl GetRange for AstBlockEntry {
    fn get_range(&self) -> AstRange {
        match &self {
            Self::Return(ast_block_return) => ast_block_return.range,
            Self::Variable(ast_block_variable) => {
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
            Self::Expression(ast_block_expression) => ast_block_expression.range,
            Self::Assign(ast_block_assign) => ast_block_assign.range,
            Self::If(ast_if) => match ast_if {
                AstIf::If {
                    range,
                    control: _,
                    control_range: _,
                    content: _,
                }
                | AstIf::Else { range, content: _ }
                | AstIf::ElseIf {
                    range,
                    control: _,
                    control_range: _,
                    content: _,
                } => *range,
            },
            Self::While(ast_while) => ast_while.range,
            Self::For(ast_for) => ast_for.range,
            Self::ForEnhanced(ast_for_enhanced) => ast_for_enhanced.range,
            Self::Break(ast_block_break) => ast_block_break.range,
            Self::Continue(ast_block_continue) => ast_block_continue.range,
            Self::Switch(ast_switch) => ast_switch.range,
            Self::SwitchCase(ast_switch_case) => ast_switch_case.range,
            Self::SwitchDefault(ast_switch_default) => ast_switch_default.range,
            Self::TryCatch(ast_try_catch) => ast_try_catch.range,
            Self::Throw(ast_throw) => ast_throw.range,
            Self::SwitchCaseArrowValues(ast_switch_case_arrow) => ast_switch_case_arrow.range,
            Self::Yield(ast_block_yield) => ast_block_yield.range,
            Self::SynchronizedBlock(ast_synchronized_block) => ast_synchronized_block.range,
            Self::SwitchCaseArrowDefault(ast_switch_case_arrow_default) => {
                ast_switch_case_arrow_default.range
            }
            Self::Thing(ast_thing) => ast_thing.get_range(),
            Self::InlineBlock(ast_block) => ast_block.range,
            Self::Semicolon(ast_range) => *ast_range,
            Self::SwitchCaseArrowType(ast_switch_case_arrow_type) => {
                ast_switch_case_arrow_type.range
            }
            Self::Assert(ast_block_assert) => ast_block_assert.range,
        }
    }
}
impl GetRange for AstThing {
    fn get_range(&self) -> AstRange {
        match self {
            Self::Class(ast_class) => ast_class.range,
            Self::Record(ast_record) => ast_record.range,
            Self::Interface(ast_interface) => ast_interface.range,
            Self::Enumeration(ast_enumeration) => ast_enumeration.range,
            Self::Annotation(ast_annotation) => ast_annotation.range,
        }
    }
}
impl GetRange for AstValue {
    fn get_range(&self) -> AstRange {
        match self {
            Self::Variable(ast_identifier) => ast_identifier.range,
            Self::Nuget(ast_value_nuget) => match ast_value_nuget {
                AstValueNuget::Int(ast_int) => ast_int.range,
                AstValueNuget::HexLiteral(h) => h.range,
                AstValueNuget::BinaryLiteral(b) => b.range,
                AstValueNuget::Double(ast_double) | AstValueNuget::Float(ast_double) => {
                    ast_double.range
                }
                AstValueNuget::StringLiteral(ast_identifier)
                | AstValueNuget::CharLiteral(ast_identifier) => ast_identifier.range,
                AstValueNuget::BooleanLiteral(ast_boolean) => ast_boolean.range,
            },
        }
    }
}
impl GetRange for AstExpression {
    fn get_range(&self) -> AstRange {
        if self.is_empty() {
            return AstRange::default();
        }
        if let Some(first) = self.first()
            && let Some(last) = self.last()
        {
            return AstRange {
                start: first.get_range().start,
                end: last.get_range().end,
            };
        }
        AstRange::default()
    }
}
impl GetRange for AstValues {
    fn get_range(&self) -> AstRange {
        if self.values.is_empty() {
            return AstRange::default();
        }
        if let Some(first) = self.values.first()
            && let Some(last) = self.values.last()
        {
            return AstRange {
                start: first.get_range().start,
                end: last.get_range().end,
            };
        }
        AstRange::default()
    }
}
impl GetRange for AstExpressionKind {
    fn get_range(&self) -> AstRange {
        match self {
            Self::Casted(ast_casted_expression) | Self::JType(ast_casted_expression) => {
                ast_casted_expression.range
            }
            Self::Recursive(ast_recursive_expression) => ast_recursive_expression.range,
            Self::Lambda(ast_lambda) => ast_lambda.range,
            Self::InlineSwitch(ast_switch) => ast_switch.range,
            Self::NewClass(ast_new_class) => ast_new_class.range,
            Self::Generics(ast_generics) => ast_generics.range,
            Self::Array(ast_values) => ast_values.range,
            Self::InstanceOf(ast_instance_of) => ast_instance_of.range,
        }
    }
}
impl GetRange for AstExpressionIdentifier {
    fn get_range(&self) -> AstRange {
        match &self {
            Self::Identifier(ast_identifier) => ast_identifier.range,
            Self::Nuget(ast_value_nuget) => ast_value_nuget.get_range(),
            Self::Value(ast_value) => ast_value.get_range(),
            Self::ArrayAccess(expr) => expr.get_range(),
            Self::EmptyArrayAccess => AstRange::default(),
        }
    }
}

impl GetRange for &AstAnnotatedParameter {
    fn get_range(&self) -> AstRange {
        match self {
            AstAnnotatedParameter::Expression(ast_expression) => ast_expression.get_range(),
            AstAnnotatedParameter::NamedExpression {
                range: _,
                name: _,
                expression,
            } => expression.get_range(),
            AstAnnotatedParameter::Annotated(ast_annotated) => ast_annotated.range,
            AstAnnotatedParameter::NamedArray {
                range,
                name: _,
                values: _,
            } => *range,
        }
    }
}
impl AstInRange for &[AstAnnotated] {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        let Some(first) = self.first().map(|i| i.range) else {
            return false;
        };
        if self.len() == 1 {
            return first.is_in_range(point);
        }
        let Some(last) = self.last().map(|i| i.range) else {
            return false;
        };
        let range = add_ranges(first, last);
        range.is_in_range(point)
    }
}

impl AstInRange for AstSuperClass {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            Self::None => false,
            Self::Name(ast_identifier) => ast_identifier.range.is_in_range(point),
        }
    }
}
