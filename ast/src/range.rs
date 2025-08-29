//! Range type and methods
use crate::types::{
    AstBlockEntry, AstClassMethod, AstIf, AstIfContent, AstPoint, AstRange, AstValue, AstValueNuget,
};

/// Join two ranges a must be before b
pub fn add_ranges(a: AstRange, b: AstRange) -> AstRange {
    assert!(a.end < b.start);
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
                ast_block_variable.range.is_in_range(point)
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
            AstBlockEntry::SwitchCase(ast_switch_case) => ast_switch_case.range.is_in_range(point),
            AstBlockEntry::SwitchDefault(ast_switch_case) => {
                ast_switch_case.range.is_in_range(point)
            }
            AstBlockEntry::TryCatch(ast_try_catch) => ast_try_catch.range.is_in_range(point),
            AstBlockEntry::Throw(ast_throw) => ast_throw.range.is_in_range(point),
            AstBlockEntry::SwitchCaseArrow(ast_switch_case_arrow) => {
                ast_switch_case_arrow.range.is_in_range(point)
            }
            AstBlockEntry::Yield(ast_block_yield) => ast_block_yield.range.is_in_range(point),
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
                el: _,
            } => range.is_in_range(point),
            AstIf::Else { range, content: _ } => range.is_in_range(point),
        }
    }
}
impl AstInRange for &AstIfContent {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            AstIfContent::Block(ast_block) => ast_block.range.is_in_range(point),
            AstIfContent::Expression(ast_expression) => ast_expression.range.is_in_range(point),
            AstIfContent::None => false,
        }
    }
}

impl AstInRange for &AstValue {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            AstValue::Variable(ast_identifier) => ast_identifier.range.is_in_range(point),
            AstValue::Nuget(ast_value_nuget) => ast_value_nuget.is_in_range(point),
            AstValue::Array(ast_values) => ast_values.range.is_in_range(point),
        }
    }
}

impl AstAfterRange for &AstValue {
    fn is_after_range(&self, point: &AstPoint) -> bool {
        match self {
            AstValue::Variable(ast_identifier) => ast_identifier.range.is_after_range(point),
            AstValue::Nuget(ast_value_nuget) => ast_value_nuget.is_after_range(point),
            AstValue::Array(ast_values) => ast_values.range.is_after_range(point),
        }
    }
}
impl AstInRange for &AstValueNuget {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            AstValueNuget::Int(ast_number) => ast_number.range.is_in_range(point),
            AstValueNuget::Double(ast_double) => ast_double.range.is_in_range(point),
            AstValueNuget::Float(ast_double) => ast_double.range.is_in_range(point),
            AstValueNuget::StringLiteral(ast_identifier) => ast_identifier.range.is_in_range(point),
            AstValueNuget::CharLiteral(ast_identifier) => ast_identifier.range.is_in_range(point),
            AstValueNuget::BooleanLiteral(ast_boolean) => ast_boolean.range.is_in_range(point),
        }
    }
}
impl AstAfterRange for &AstValueNuget {
    fn is_after_range(&self, point: &AstPoint) -> bool {
        match self {
            AstValueNuget::Int(ast_number) => ast_number.range.is_after_range(point),
            AstValueNuget::Double(ast_double) => ast_double.range.is_after_range(point),
            AstValueNuget::Float(ast_double) => ast_double.range.is_after_range(point),
            AstValueNuget::StringLiteral(ast_identifier) => {
                ast_identifier.range.is_after_range(point)
            }
            AstValueNuget::CharLiteral(ast_identifier) => {
                ast_identifier.range.is_after_range(point)
            }
            AstValueNuget::BooleanLiteral(ast_boolean) => ast_boolean.range.is_after_range(point),
        }
    }
}
