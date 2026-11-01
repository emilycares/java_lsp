use ast::types::{AstClassMethod, AstMethodParameter, AstRange};
use bitflags::bitflags;
use dto::JType;
use dto_extra::ast_jtype_ref_to_jtype;
use my_string::NuVec;

/// variable or function in a ast
#[derive(Debug, PartialEq, Clone)]
pub struct LocalVariable {
    pub jtype: JType,
    pub name: NuVec,
    pub range: AstRange,
    pub flags: VarFlags,
}

bitflags! {
   #[derive(Clone, Eq, PartialEq, Debug, Default)]
   pub struct VarFlags: u8 {
     const Function = 0b0000_0001;
     const Computed = 0b0000_0010;
   }
}

impl LocalVariable {
    #[must_use]
    pub fn from_class_method(i: &AstClassMethod) -> Self {
        Self {
            jtype: ast_jtype_ref_to_jtype(&i.header.jtype),
            name: (&i.header.name).into(),
            range: i.range,
            flags: VarFlags::Function,
        }
    }

    pub fn from_method_parameter(parameter: &AstMethodParameter) -> Self {
        Self {
            jtype: ast_jtype_ref_to_jtype(&parameter.jtype),
            name: (&parameter.name).into(),
            range: parameter.range,
            flags: VarFlags::empty(),
        }
    }
}
