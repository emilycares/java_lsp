use ast::range::AstRangeHelper;
use ast::types::{
    AstBlock, AstBlockEntry, AstExpression, AstExpressionIdentifier, AstFile, AstPoint, AstRange,
    AstThing, AstValue,
};

#[derive(Debug, PartialEq, Clone)]
pub enum CallItem {
    MethodCall {
        name: String,
        range: AstRange,
    },
    FieldAccess {
        name: String,
        range: AstRange,
    },
    Variable {
        name: String,
        range: AstRange,
    },
    This {
        range: AstRange,
    },
    Class {
        name: String,
        range: AstRange,
    },
    ClassOrVariable {
        name: String,
        range: AstRange,
    },
    ArgumentList {
        prev: Vec<CallItem>,
        active_param: Option<usize>,
        filled_params: Vec<Vec<CallItem>>,
        range: AstRange,
    },
}

impl CallItem {
    pub fn get_range(&self) -> &AstRange {
        match self {
            CallItem::MethodCall { name: _, range } => range,
            CallItem::FieldAccess { name: _, range } => range,
            CallItem::Variable { name: _, range } => range,
            CallItem::This { range } => range,
            CallItem::Class { name: _, range } => range,
            CallItem::ClassOrVariable { name: _, range } => range,
            CallItem::ArgumentList {
                prev: _,
                active_param: _,
                filled_params: _,
                range,
            } => range,
        }
    }
}

#[allow(unused)]
#[derive(Debug, PartialEq, Clone)]
struct Argument {
    range: Option<AstRange>,
    value: Vec<CallItem>,
}

/// Provides data abuilt the current variable before the cursor
/// ``` java
/// Long other = 1l;
/// other.
///       ^
/// ```
/// Then it would return info about the variable other
pub fn get_call_chain(ast: &AstFile, point: &AstPoint) -> Option<Vec<CallItem>> {
    let mut out: Vec<CallItem> = vec![];

    match &ast.thing {
        AstThing::Class(ast_class) => {
            out.extend(
                ast_class
                    .methods
                    .iter()
                    .filter(|i| i.is_in_range(point))
                    .flat_map(|i| cc_block(&i.block, point))
                    .flatten(),
            );
        }
        AstThing::Interface(_) => todo!(),
        AstThing::Enumeration(_) => todo!(),
        AstThing::Annotation(_) => todo!(),
    }

    if !out.is_empty() {
        return Some(out);
    }
    None
}

fn cc_block(block: &AstBlock, point: &AstPoint) -> Option<Vec<CallItem>> {
    if let Some(entry) = block.entries.iter().find(|i| i.is_in_range(point)) {
        return cc_block_entrie(entry, point);
    }
    None
}

fn cc_block_entrie(entry: &AstBlockEntry, point: &AstPoint) -> Option<Vec<CallItem>> {
    match entry {
        AstBlockEntry::Return(ast_block_return) => {
            if let Some(ref value) = ast_block_return.value {
                return cc_value(value, point);
            }
            None
        }
        AstBlockEntry::Variable(ast_block_variable) => {
            if let Some(ref value) = ast_block_variable.value {
                return cc_value(value, point);
            }
            None
        }
        AstBlockEntry::Expression(ast_block_expression) => {
            cc_expression(&ast_block_expression.value, point)
        }
        AstBlockEntry::Assign(_ast_block_assign) => todo!(),
    }
}

fn cc_value(value: &AstValue, point: &AstPoint) -> Option<Vec<CallItem>> {
    match value {
        AstValue::NewClass(_ast_value_new_class) => todo!(),
        AstValue::Equasion(_ast_value_equasion) => todo!(),
        AstValue::Variable(_ast_identifier) => todo!(),
        AstValue::Nuget(_ast_value_nuget) => todo!(),
        AstValue::Expression(ast_expression) => cc_expression(ast_expression, point),
    }
}

fn cc_expression(ast_expression: &AstExpression, point: &AstPoint) -> Option<Vec<CallItem>> {
    let mut out = vec![];
    cc_expr(ast_expression, point, &mut out);
    Some(out)
}

fn cc_expr(ast_expression: &AstExpression, point: &AstPoint, out: &mut Vec<CallItem>) {
    if let Some(ident) = &ast_expression.ident {
        let mut has_args = false;
        if let Some(n) = &ast_expression.next {
            has_args = n.values.is_some();
        }
            fun_name(ident, has_args, out);
    }
    if let Some(next) = &ast_expression.next {
        cc_expr(next.as_ref(), point, out);
    }
}

fn fun_name(ident: &AstExpressionIdentifier, has_args: bool, out: &mut Vec<CallItem>) {
    match ident {
        AstExpressionIdentifier::Identifier(ast_identifier) => {
            if has_args {
                out.push(CallItem::MethodCall {
                    name: ast_identifier.into(),
                    range: ast_identifier.range.clone(),
                });
            } else {
                out.push(CallItem::ClassOrVariable {
                    name: ast_identifier.into(),
                    range: ast_identifier.range.clone(),
                });
            }
        },
        AstExpressionIdentifier::Nuget(ast_value_nuget) => todo!(),
    }
}
