use crate::types::{
    AstBlockEntry, AstClassMethod, AstIf, AstIfContent, AstPoint, AstRange, AstValue, AstValueNuget,
};

pub fn add_ranges(a: AstRange, b: AstRange) -> AstRange {
    AstRange {
        start: a.start,
        end: b.end,
    }
}
pub trait AstRangeHelper {
    fn is_in_range(&self, point: &AstPoint) -> bool;
}

impl AstRangeHelper for &AstClassMethod {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        self.range.is_in_range(point)
    }
}

impl AstRangeHelper for &AstBlockEntry {
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
        }
    }
}
impl AstRangeHelper for &AstIf {
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
impl AstRangeHelper for &AstIfContent {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            AstIfContent::Block(ast_block) => ast_block.range.is_in_range(point),
            AstIfContent::Value(ast_value) => ast_value.is_in_range(point),
            AstIfContent::None => false,
        }
    }
}

impl AstRangeHelper for &AstValue {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            AstValue::NewClass(ast_value_new_class) => ast_value_new_class.range.is_in_range(point),
            AstValue::Equasion(ast_value_equasion) => ast_value_equasion.range.is_in_range(point),
            AstValue::Variable(ast_identifier) => ast_identifier.range.is_in_range(point),
            AstValue::Expression(ast_expression) => ast_expression.range.is_in_range(point),
            AstValue::Nuget(ast_value_nuget) => ast_value_nuget.is_in_range(point),
            AstValue::Array(ast_values) => ast_values.range.is_in_range(point),
        }
    }
}
impl AstRangeHelper for &AstValueNuget {
    fn is_in_range(&self, point: &AstPoint) -> bool {
        match self {
            AstValueNuget::Number(ast_number) => ast_number.range.is_in_range(point),
            AstValueNuget::Double(ast_double) => ast_double.range.is_in_range(point),
            AstValueNuget::Float(ast_double) => ast_double.range.is_in_range(point),
            AstValueNuget::StringLiteral(ast_identifier) => ast_identifier.range.is_in_range(point),
            AstValueNuget::CharLiteral(ast_identifier) => ast_identifier.range.is_in_range(point),
            AstValueNuget::BooleanLiteral(ast_boolean) => ast_boolean.range.is_in_range(point),
        }
    }
}
