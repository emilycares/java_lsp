use ast::types::{AstClassMethod, AstMethodParameter, AstRange};
use dto::JType;
use my_string::MyString;

/// variable or function in a ast
#[derive(Debug, PartialEq, Clone)]
pub struct LocalVariable {
    pub level: usize,
    pub jtype: JType,
    pub name: MyString,
    pub is_fun: bool,
    pub range: AstRange,
}
impl LocalVariable {
    #[must_use]
    pub fn from_class_method(i: &AstClassMethod, level: usize) -> Self {
        Self {
            level,
            jtype: (&i.header.jtype).into(),
            name: (&i.header.name).into(),
            is_fun: true,
            range: i.range,
        }
    }

    pub fn from_method_parameter(parameter: &AstMethodParameter, level: usize) -> Self {
        Self {
            level,
            jtype: (&parameter.jtype).into(),
            name: (&parameter.name).into(),
            is_fun: false,
            range: parameter.range,
        }
    }
}
