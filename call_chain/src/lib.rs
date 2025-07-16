use std::cmp::{max, min};

use ast::range::AstRangeHelper;
use ast::types::{
    AstBlock, AstBlockEntry, AstExpression, AstExpressionIdentifier, AstFile, AstPoint, AstRange,
    AstThing, AstValue, AstValueNuget,
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
        AstValue::Equasion(ast_value_equasion) => cc_equasion(ast_value_equasion, point),
        AstValue::Variable(ast_identifier) => cc_variable(ast_identifier),
        AstValue::Nuget(_ast_value_nuget) => todo!(),
        AstValue::Expression(ast_expression) => cc_expression(ast_expression, point),
    }
}

fn cc_variable(ast_identifier: &ast::types::AstIdentifier) -> Option<Vec<CallItem>> {
    Some(vec![CallItem::ClassOrVariable {
        name: ast_identifier.into(),
        range: ast_identifier.range.clone(),
    }])
}

fn cc_equasion(
    ast_value_equasion: &ast::types::AstValueEquasion,
    point: &AstPoint,
) -> Option<Vec<CallItem>> {
    let lhs = dist_value(point, ast_value_equasion.lhs.as_ref());
    let rhs = dist_value(point, ast_value_equasion.rhs.as_ref());
    let i = lhs.min(rhs);
    if i == lhs {
        cc_value(&ast_value_equasion.lhs, point)
    } else {
        cc_value(&ast_value_equasion.rhs, point)
    }
}

fn cc_expression(ast_expression: &AstExpression, point: &AstPoint) -> Option<Vec<CallItem>> {
    let mut out = vec![];
    cc_expr(ast_expression, point, false, &mut out);
    Some(out)
}

fn cc_expr(
    ast_expression: &AstExpression,
    point: &AstPoint,
    has_parent: bool,
    out: &mut Vec<CallItem>,
) {
    if let Some(ident) = &ast_expression.ident {
        let mut has_args = false;
        if let Some(n) = &ast_expression.next {
            has_args = n.values.is_some();
        }
        cc_expr_ident(ident, has_args, has_parent, out);
    }
    if let Some(next) = &ast_expression.next {
        cc_expr(next.as_ref(), point, true, out);
    }
    if let Some(values) = &ast_expression.values {
        cc_arugments(point, out, values);
    }
}

fn cc_arugments(point: &AstPoint, out: &mut Vec<CallItem>, values: &ast::types::AstValues) {
    if values.range.is_in_range(point) {
        let active_param = get_active_param(values, point);
        dbg!(&values.values);
        let mut filled_params: Vec<Vec<CallItem>> = values
            .values
            .iter()
            .map(|i| cc_value(i, point).unwrap_or_default())
            .collect();

        if filled_params.is_empty() {
            filled_params.push(vec![]);
        }
        let selected_arg = filled_params.get(active_param).cloned();
        let args = CallItem::ArgumentList {
            prev: out.clone(),
            active_param: Some(active_param),
            filled_params,
            range: values.range.clone(),
        };
        out.clear();

        out.push(args);
        if let Some(sel) = selected_arg {
            out.extend(sel);
        }
    }
}

fn get_active_param(values: &ast::types::AstValues, point: &AstPoint) -> usize {
    values
        .values
        .iter()
        .enumerate()
        .min_by_key(|(_, value)| dist_value(point, value))
        .map(|i| i.0)
        .unwrap_or_default()
}

fn dist_value(point: &AstPoint, value: &AstValue) -> usize {
    match value {
        AstValue::NewClass(ast_value_new_class) => dist(point, &ast_value_new_class.range),
        AstValue::Equasion(ast_value_equasion) => dist(point, &ast_value_equasion.range),
        AstValue::Variable(ast_identifier) => dist(point, &ast_identifier.range),
        AstValue::Expression(ast_expression) => dist(point, &ast_expression.range),
        AstValue::Nuget(ast_value_nuget) => match ast_value_nuget {
            AstValueNuget::Number(ast_number) => dist(point, &ast_number.range),
            AstValueNuget::Double(ast_double) => dist(point, &ast_double.range),
            AstValueNuget::Float(ast_double) => dist(point, &ast_double.range),
            AstValueNuget::StringLiteral(ast_identifier) => dist(point, &ast_identifier.range),
            AstValueNuget::CharLiteral(ast_identifier) => dist(point, &ast_identifier.range),
            AstValueNuget::BooleanLiteral(ast_boolean) => dist(point, &ast_boolean.range),
        },
    }
}
fn dist(point: &AstPoint, range: &AstRange) -> usize {
    if point < &range.start {
        line_col_diff(point, &range.start)
    } else if point > &range.end {
        line_col_diff(point, &range.end)
    } else {
        0
    }
}

fn line_col_diff(a: &AstPoint, b: &AstPoint) -> usize {
    let line_diff = max(a.line, b.line) - min(a.line, b.line);
    let col_diff = max(a.col, b.col) - min(a.col, b.col);
    line_diff * 1000 + col_diff
}

fn cc_expr_ident(
    ident: &AstExpressionIdentifier,
    has_args: bool,
    has_parent: bool,
    out: &mut Vec<CallItem>,
) {
    match ident {
        AstExpressionIdentifier::Identifier(ast_identifier) => {
            if has_args {
                out.push(CallItem::MethodCall {
                    name: ast_identifier.into(),
                    range: ast_identifier.range.clone(),
                });
            } else if has_parent {
                out.push(CallItem::FieldAccess {
                    name: ast_identifier.into(),
                    range: ast_identifier.range.clone(),
                });
            } else {
                out.push(CallItem::ClassOrVariable {
                    name: ast_identifier.into(),
                    range: ast_identifier.range.clone(),
                });
            }
        }
        AstExpressionIdentifier::Nuget(ast_value_nuget) => match ast_value_nuget {
            AstValueNuget::Number(_ast_number) => todo!(),
            AstValueNuget::Double(_ast_double) => todo!(),
            AstValueNuget::Float(_ast_double) => todo!(),
            AstValueNuget::StringLiteral(ast_identifier) => {
                out.push(CallItem::Class {
                    name: "String".into(),
                    range: ast_identifier.range.clone(),
                });
            }
            AstValueNuget::CharLiteral(_ast_identifier) => todo!(),
            AstValueNuget::BooleanLiteral(_ast_boolean) => todo!(),
        },
    }
}
