use crate::types::{AstBlockEntry, AstClassMethod, AstPoint, AstValue, AstValueNuget};

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
