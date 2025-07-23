use std::cmp::{self, max, min};

use ast::range::{AstInRange, add_ranges};
use ast::types::{
    AstBlock, AstBlockEntry, AstExpression, AstExpressionIdentifier, AstExpressionOperator,
    AstFile, AstIf, AstIfContent, AstPoint, AstRange, AstThing, AstValue, AstValueNuget,
};
use smol_str::SmolStr;

#[derive(Debug, PartialEq, Clone)]
pub enum CallItem {
    MethodCall {
        name: SmolStr,
        range: AstRange,
    },
    FieldAccess {
        name: SmolStr,
        range: AstRange,
    },
    Variable {
        name: SmolStr,
        range: AstRange,
    },
    This {
        range: AstRange,
    },
    Class {
        name: SmolStr,
        range: AstRange,
    },
    ClassOrVariable {
        name: SmolStr,
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

pub fn validate(call_chain: &[CallItem], point: &AstPoint) -> (usize, Vec<CallItem>) {
    let item = call_chain
        .iter()
        .enumerate()
        .find(|(_n, ci)| match ci {
            CallItem::MethodCall { name: _, range } => range.is_in_range(point),
            CallItem::FieldAccess { name: _, range } => range.is_in_range(point),
            CallItem::Variable { name: _, range } => range.is_in_range(point),
            CallItem::This { range } => range.is_in_range(point),
            CallItem::ClassOrVariable { name: _, range } => range.is_in_range(point),
            CallItem::Class { name: _, range } => range.is_in_range(point),
            CallItem::ArgumentList {
                prev,
                range,
                filled_params: _,
                active_param: _,
            } => {
                if range.is_in_range(point) {
                    return true;
                }
                let mut prevs = None;
                for p in prev {
                    match prevs {
                        None => {
                            prevs = Some(*p.get_range());
                        }
                        Some(pr) => prevs = Some(add_ranges(pr, *p.get_range())),
                    }
                }
                if let Some(r) = prevs {
                    if r.is_in_range(point) {
                        return true;
                    }
                }
                false
            }
        })
        .map(|i| i.0)
        .unwrap_or_default();

    let relevat = &call_chain[0..cmp::min(item + 1, call_chain.len())];
    (item, relevat.to_vec())
}

fn cc_block(block: &AstBlock, point: &AstPoint) -> Option<Vec<CallItem>> {
    if !block.range.is_in_range(point) {
        return None;
    }

    if let Some(entry) = block
        .entries
        .iter()
        .min_by_key(|expression| dist_block_entry(point, expression.as_ref()))
    {
        return cc_block_entrie(entry, point);
    }
    None
}

fn dist_block_entry(point: &AstPoint, entry: &AstBlockEntry) -> usize {
    match entry {
        AstBlockEntry::Return(ast_block_return) => dist(point, &ast_block_return.range),
        AstBlockEntry::Variable(ast_block_variable) => dist(point, &ast_block_variable.range),
        AstBlockEntry::Expression(ast_block_expression) => dist(point, &ast_block_expression.range),
        AstBlockEntry::Assign(ast_block_assign) => dist(point, &ast_block_assign.range),
        AstBlockEntry::If(ast_if) => dist_if(point, ast_if),
        AstBlockEntry::While(ast_while) => dist(point, &ast_while.range),
        AstBlockEntry::For(ast_for) => dist(point, &ast_for.range),
    }
}

fn dist_if(point: &AstPoint, ast_if: &AstIf) -> usize {
    match ast_if {
        AstIf::If {
            range,
            control: _,
            control_range: _,
            content: _,
            el: _,
        } => dist(&point, range),
        AstIf::Else { range, content: _ } => dist(&point, range),
    }
}

fn cc_block_entrie(entry: &AstBlockEntry, point: &AstPoint) -> Option<Vec<CallItem>> {
    match entry {
        AstBlockEntry::Return(ast_block_return) => {
            if let Some(ref expression) = ast_block_return.expression {
                return cc_expression(expression, point);
            }
            None
        }
        AstBlockEntry::Variable(ast_block_variable) => {
            if let Some(ref expression) = ast_block_variable.expression {
                return cc_expression(expression, point);
            }
            None
        }
        AstBlockEntry::Expression(ast_block_expression) => {
            cc_expression(&ast_block_expression.value, point)
        }
        AstBlockEntry::Assign(_ast_block_assign) => todo!(),
        AstBlockEntry::If(ast_if) => cc_if(ast_if, point),
        AstBlockEntry::While(_ast_while) => todo!(),
        AstBlockEntry::For(_ast_for) => todo!(),
    }
}

fn cc_if(ast_if: &AstIf, point: &AstPoint) -> Option<Vec<CallItem>> {
    match ast_if {
        AstIf::If {
            range,
            control,
            control_range,
            content,
            el,
        } => {
            if !range.is_in_range(point) {
                return None;
            }
            if control_range.is_in_range(point) {
                return cc_value(control);
            }
            if content.is_in_range(point) {
                return cc_if_content(content, point);
            }
            if let Some(el) = el {
                return cc_if(el, point);
            }
            None
        }
        AstIf::Else { range, content } => {
            if !range.is_in_range(point) {
                return None;
            }
            cc_if_content(content, point)
        }
    }
}

fn cc_if_content(content: &AstIfContent, point: &AstPoint) -> Option<Vec<CallItem>> {
    match content {
        AstIfContent::Block(ast_block) => cc_block(ast_block, point),
        AstIfContent::None => None,
        AstIfContent::Expression(ast_expression) => cc_expression(ast_expression, point),
    }
}

fn cc_value(value: &AstValue) -> Option<Vec<CallItem>> {
    match value {
        AstValue::NewClass(_ast_value_new_class) => todo!(),
        AstValue::Variable(ast_identifier) => cc_variable(ast_identifier),
        AstValue::Nuget(ast_nuget) => cc_value_nuget(ast_nuget),
        AstValue::Array(_ast_values) => todo!(),
    }
}
fn cc_value_nuget(ast_nuget: &AstValueNuget) -> Option<Vec<CallItem>> {
    match ast_nuget {
        AstValueNuget::Number(_ast_number) => todo!(),
        AstValueNuget::Double(_ast_double) => todo!(),
        AstValueNuget::Float(_ast_double) => todo!(),
        AstValueNuget::StringLiteral(ast_identifier) => Some(vec![CallItem::Class {
            name: "String".into(),
            range: ast_identifier.range,
        }]),
        AstValueNuget::CharLiteral(_ast_identifier) => todo!(),
        AstValueNuget::BooleanLiteral(_ast_boolean) => todo!(),
    }
}

fn cc_variable(ast_identifier: &ast::types::AstIdentifier) -> Option<Vec<CallItem>> {
    Some(vec![CallItem::ClassOrVariable {
        name: ast_identifier.into(),
        range: ast_identifier.range,
    }])
}

fn cc_expression(ast_expression: &AstExpression, point: &AstPoint) -> Option<Vec<CallItem>> {
    dbg!(ast_expression);
    // if !(ast_expression.range.is_in_range(point) || ast_expression.range.is_after_range(point)) {
    //     return None;
    // }
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
        let mut filled_params: Vec<Vec<CallItem>> = values
            .values
            .iter()
            .map(|i| cc_expression(i, point).unwrap_or_default())
            .collect();

        if filled_params.is_empty() {
            filled_params.push(vec![]);
        }
        let selected_arg = filled_params.get(active_param).cloned();
        let args = CallItem::ArgumentList {
            prev: out.clone(),
            active_param: Some(active_param),
            filled_params,
            range: values.range,
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
        .min_by_key(|(_, expression)| dist(point, &expression.range))
        .map(|i| i.0)
        .unwrap_or_default()
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
                    range: ast_identifier.range,
                });
            } else if has_parent {
                out.push(CallItem::FieldAccess {
                    name: ast_identifier.into(),
                    range: ast_identifier.range,
                });
            } else {
                out.push(CallItem::ClassOrVariable {
                    name: ast_identifier.into(),
                    range: ast_identifier.range,
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
                    range: ast_identifier.range,
                });
            }
            AstValueNuget::CharLiteral(_ast_identifier) => todo!(),
            AstValueNuget::BooleanLiteral(_ast_boolean) => todo!(),
        },
        AstExpressionIdentifier::Value(ast_value) => {
            if let Some(val) = cc_value(&ast_value) {
                out.extend(val);
            }
        }
        AstExpressionIdentifier::ArrayAccess(_ast_value) => todo!(),
    }
}
