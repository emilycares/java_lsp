use std::cmp::{self, max, min};

use ast::range::{AstInRange, add_ranges};
use ast::types::{
    AstAnnotated, AstBlock, AstBlockEntry, AstBlockVariable, AstClassBlock, AstExpression,
    AstExpressionIdentifier, AstExpressionOperator, AstExpressionOrValue, AstFile, AstIdentifier,
    AstIf, AstIfContent, AstJTypeKind, AstLambdaRhs, AstNewClass, AstNewRhs, AstPoint, AstRange,
    AstRecursiveExpression, AstThing, AstValue, AstValueNuget, AstValues,
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
pub fn get_call_chain(ast: &AstFile, point: &AstPoint) -> Vec<CallItem> {
    let mut out = vec![];
    match &ast.thing {
        AstThing::Class(ast_class) => {
            cc_class_block(&ast_class.block, point, &mut out);
        }
        AstThing::Interface(ast_interface) => ast_interface
            .default_methods
            .iter()
            .filter(|i| i.range.is_in_range(point))
            .for_each(|i| {
                if i.block.range.is_in_range(point) {
                    cc_block(&i.block, point, &mut out)
                } else {
                    cc_annotated(&i.annotated, point, &mut out)
                }
            }),
        AstThing::Enumeration(_) => todo!(),
        AstThing::Annotation(_) => todo!(),
    }
    out
}

fn cc_class_block(block: &AstClassBlock, point: &AstPoint, out: &mut Vec<CallItem>) {
    block
        .variables
        .iter()
        .filter(|i| i.range.is_in_range(point))
        .for_each(|i| {
            if let Some(expr) = &i.expression {
                return cc_expr(expr, point, false, out);
            }
        });
    block
        .methods
        .iter()
        .filter(|i| i.range.is_in_range(point))
        .for_each(|i| {
            if i.block.range.is_in_range(point) {
                cc_block(&i.block, point, out)
            } else {
                cc_annotated(&i.annotated, point, out)
            }
        });
    block
        .constructors
        .iter()
        .filter(|i| i.range.is_in_range(point))
        .for_each(|i| {
            if i.block.range.is_in_range(point) {
                cc_block(&i.block, point, out)
            } else {
                cc_annotated(&i.annotated, point, out)
            }
        });
}

fn cc_annotated(annotated: &[AstAnnotated], point: &AstPoint, out: &mut Vec<CallItem>) {
    if let Some(a) = annotated.iter().find(|i| i.range.is_in_range(point)) {
        let param = a
            .parameters
            .iter()
            .min_by_key(|expression| dist(point, ast_expression_get_range(expression)));

        if let Some(p) = param {
            cc_expr(p, point, false, out);
        }
    }
}

fn ast_expression_get_range(expression: &AstExpression) -> &AstRange {
    match expression {
        AstExpression::Casted(ast_casted_expression) => &ast_casted_expression.range,
        AstExpression::Recursive(ast_recursive_expression) => &ast_recursive_expression.range,
        AstExpression::Lambda(ast_lambda) => &ast_lambda.range,
        AstExpression::InlineSwitch(ast_switch) => &ast_switch.range,
        AstExpression::NewClass(ast_new_class) => &ast_new_class.range,
    }
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
                if let Some(r) = prevs
                    && r.is_in_range(point)
                {
                    return true;
                }
                false
            }
        })
        .map(|i| i.0)
        .unwrap_or_default();

    let relevat = &call_chain[0..cmp::min(item + 1, call_chain.len())];
    (item, relevat.to_vec())
}

fn cc_block(block: &AstBlock, point: &AstPoint, out: &mut Vec<CallItem>) {
    if !block.range.is_in_range(point) {
        return;
    }

    if let Some(entry) = block
        .entries
        .iter()
        .min_by_key(|expression| dist_block_entry(point, expression))
    {
        cc_block_entrie(entry, point, out);
        return;
    }
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
        AstBlockEntry::ForEnhanced(ast_for_enhanced) => dist(point, &ast_for_enhanced.range),
        AstBlockEntry::Break(ast_block_break) => dist(point, &ast_block_break.range),
        AstBlockEntry::Continue(ast_block_continue) => dist(point, &ast_block_continue.range),
        AstBlockEntry::Switch(ast_switch) => dist(point, &ast_switch.range),
        AstBlockEntry::SwitchCase(ast_switch_case) => dist(point, &ast_switch_case.range),
        AstBlockEntry::SwitchDefault(ast_switch_default) => dist(point, &ast_switch_default.range),
        AstBlockEntry::TryCatch(ast_try_catch) => dist(point, &ast_try_catch.range),
        AstBlockEntry::Throw(ast_throw) => dist(point, &ast_throw.range),
        AstBlockEntry::SwitchCaseArrow(ast_switch_case_arrow) => {
            dist(point, &ast_switch_case_arrow.range)
        }
        AstBlockEntry::Yield(ast_block_yield) => dist(point, &ast_block_yield.range),
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
        } => dist(point, range),
        AstIf::Else { range, content: _ } => dist(point, range),
    }
}

fn cc_block_entrie(entry: &AstBlockEntry, point: &AstPoint, out: &mut Vec<CallItem>) {
    match entry {
        AstBlockEntry::Return(ast_block_return) => {
            if let AstExpressionOrValue::Expression(ref expression) = ast_block_return.expression {
                cc_expr(expression, point, false, out);
            }
        }
        AstBlockEntry::Yield(ast_block_yield) => {
            if let AstExpressionOrValue::Expression(ref expression) = ast_block_yield.expression {
                cc_expr(expression, point, false, out);
            }
        }
        AstBlockEntry::Variable(ast_block_variable) => {
            cc_block_variable(ast_block_variable, point, out)
        }
        AstBlockEntry::Expression(ast_block_expression) => {
            cc_expr_recursive(&ast_block_expression.value, point, false, out)
        }
        AstBlockEntry::Assign(ast_block_assign) => {
            if let Some(expr) = &ast_block_assign.expression {
                cc_expr(expr, point, false, out);
            }
        }
        AstBlockEntry::If(ast_if) => cc_if(ast_if, point, out),
        AstBlockEntry::While(ast_while) => {
            if ast_while.control.range.is_in_range(point) {
                return cc_expr_recursive(&ast_while.control, point, false, out);
            }
            if ast_while.block.range.is_in_range(point) {
                return cc_block(&ast_while.block, point, out);
            }
        }
        AstBlockEntry::For(ast_for) => {
            if ast_for.var.range.is_in_range(point) {
                return cc_block_variable(&ast_for.var, point, out);
            }
            if ast_for.check.range.is_in_range(point) {
                return cc_expr_recursive(&ast_for.check, point, false, out);
            }
            if ast_for.change.range.is_in_range(point) {
                return cc_expr_recursive(&ast_for.change, point, false, out);
            }
            if ast_for.block.range.is_in_range(point) {
                return cc_block(&ast_for.block, point, out);
            }
        }
        AstBlockEntry::ForEnhanced(ast_for_enhanced) => {
            if ast_for_enhanced.var.range.is_in_range(point) {
                return cc_block_variable(&ast_for_enhanced.var, point, out);
            }
            if ast_for_enhanced.rhs.range.is_in_range(point) {
                return cc_expr_recursive(&ast_for_enhanced.rhs, point, false, out);
            }
            if ast_for_enhanced.block.range.is_in_range(point) {
                return cc_block(&ast_for_enhanced.block, point, out);
            }
        }
        AstBlockEntry::Break(_ast_block_break) => (),
        AstBlockEntry::Continue(_ast_block_continue) => (),
        AstBlockEntry::Switch(ast_switch) => {
            if ast_switch.check.range.is_in_range(point) {
                return cc_expr_recursive(&ast_switch.check, point, false, out);
            }
            if ast_switch.block.range.is_in_range(point) {
                return cc_block(&ast_switch.block, point, out);
            }
        }
        AstBlockEntry::SwitchCase(ast_switch_case) => cc_value(&ast_switch_case.value, point, out),
        AstBlockEntry::SwitchDefault(_ast_switch_default) => (),
        AstBlockEntry::TryCatch(ast_try_catch) => {
            if let Some(res) = &ast_try_catch.resources_block {
                cc_block(&res, point, out);
            }

            cc_block(&ast_try_catch.block, point, out);

            if let Some(case) = &ast_try_catch
                .cases
                .iter()
                .find(|i| i.range.is_in_range(point))
            {
                cc_block(&case.block, point, out);
            }

            if let Some(res) = &ast_try_catch.finally_block {
                cc_block(&res, point, out);
            }
        }
        AstBlockEntry::Throw(ast_throw) => cc_new_class(&ast_throw.value, point, out),
        AstBlockEntry::SwitchCaseArrow(_ast_switch_case_arrow) => todo!(),
    }
}

fn cc_block_variable(
    ast_block_variable: &AstBlockVariable,
    point: &AstPoint,
    out: &mut Vec<CallItem>,
) {
    if let Some(ref expression) = ast_block_variable.expression {
        return cc_expr(expression, point, false, out);
    }
}

fn cc_if(ast_if: &AstIf, point: &AstPoint, out: &mut Vec<CallItem>) {
    match ast_if {
        AstIf::If {
            range,
            control,
            control_range,
            content,
            el,
        } => {
            if !range.is_in_range(point) {
                return;
            }
            if control_range.is_in_range(point) {
                return cc_expr_recursive(control, point, false, out);
            }
            if content.is_in_range(point) {
                return cc_if_content(content, point, out);
            }
            if let Some(el) = el {
                return cc_if(el, point, out);
            }
        }
        AstIf::Else { range, content } => {
            if !range.is_in_range(point) {
                return;
            }
            cc_if_content(content, point, out)
        }
    }
}

fn cc_if_content(content: &AstIfContent, point: &AstPoint, out: &mut Vec<CallItem>) {
    match content {
        AstIfContent::Block(ast_block) => cc_block(ast_block, point, out),
        AstIfContent::None => (),
        AstIfContent::Expression(ast_expression) => {
            cc_expr_recursive(ast_expression, point, false, out)
        }
    }
}

fn cc_value(value: &AstValue, point: &AstPoint, out: &mut Vec<CallItem>) {
    match value {
        AstValue::Variable(ast_identifier) => cc_variable(ast_identifier, out),
        AstValue::Nuget(ast_nuget) => cc_value_nuget(ast_nuget, out),
        AstValue::NewClass(ast_new_class) => cc_new_class(ast_new_class, point, out),
        AstValue::Array(_ast_values) => todo!(),
    }
}

fn cc_new_class(ast_new_class: &AstNewClass, point: &AstPoint, out: &mut Vec<CallItem>) {
    if let AstJTypeKind::Class(c) = &ast_new_class.jtype.value {
        out.push(CallItem::Class {
            name: c.value.clone(),
            range: ast_new_class.range,
        });
    }
    match ast_new_class.rhs.as_ref() {
        AstNewRhs::None => (),
        AstNewRhs::Parameters(ast_expressions) => ast_expressions
            .iter()
            .for_each(|i| cc_expr(i, point, false, out)),
        AstNewRhs::Block(ast_class_block) => {
            cc_class_block(ast_class_block, point, out);
        }
        AstNewRhs::ParametersAndBlock(ast_expressions, ast_class_block) => {
            ast_expressions
                .iter()
                .for_each(|i| cc_expr(i, point, false, out));

            cc_class_block(ast_class_block, point, out);
        }
        AstNewRhs::Array(ast_values) => cc_array(ast_values, point, out),
    }
}

fn cc_array(ast_values: &AstValues, point: &AstPoint, out: &mut Vec<CallItem>) {
    if !ast_values.range.is_in_range(point) {
        return;
    }

    ast_values
        .values
        .iter()
        .for_each(|i| cc_expr(i, point, false, out))
}
fn cc_value_nuget(ast_nuget: &AstValueNuget, out: &mut Vec<CallItem>) {
    match ast_nuget {
        AstValueNuget::Int(ast_number) => out.push(CallItem::Class {
            name: "Integer".into(),
            range: ast_number.range,
        }),
        AstValueNuget::Double(ast_double) => out.push(CallItem::Class {
            name: "Double".into(),
            range: ast_double.range,
        }),
        AstValueNuget::Float(ast_double) => out.push(CallItem::Class {
            name: "Float".into(),
            range: ast_double.range,
        }),
        AstValueNuget::StringLiteral(ast_identifier) => out.push(CallItem::Class {
            name: "String".into(),
            range: ast_identifier.range,
        }),
        AstValueNuget::CharLiteral(ast_identifier) => out.push(CallItem::Class {
            name: "Char".into(),
            range: ast_identifier.range,
        }),
        AstValueNuget::BooleanLiteral(ast_boolean) => out.push(CallItem::Class {
            name: "Boolean".into(),
            range: ast_boolean.range,
        }),
    }
}

fn cc_variable(ast_identifier: &AstIdentifier, out: &mut Vec<CallItem>) {
    out.push(CallItem::ClassOrVariable {
        name: ast_identifier.into(),
        range: ast_identifier.range,
    });
}

fn cc_expr(
    ast_expression: &AstExpression,
    point: &AstPoint,
    has_parent: bool,
    out: &mut Vec<CallItem>,
) {
    match ast_expression {
        AstExpression::Casted(ast_casted_expression) => {
            cc_expr_recursive(&ast_casted_expression.expression, point, has_parent, out)
        }
        AstExpression::Recursive(ast_recursive_expression) => {
            cc_expr_recursive(ast_recursive_expression, point, has_parent, out)
        }
        AstExpression::Lambda(ast_lambda) => match &ast_lambda.rhs {
            AstLambdaRhs::None => (),
            AstLambdaRhs::Block(ast_block) => cc_block(&ast_block, point, out),
            AstLambdaRhs::Expr(ast_base_expression) => {
                cc_expr(&ast_base_expression, point, has_parent, out)
            }
        },
        AstExpression::InlineSwitch(_ast_switch) => (),
        AstExpression::NewClass(ast_new_class) => cc_new_class(ast_new_class, point, out),
    }
}

fn cc_expr_recursive(
    ast_expression: &AstRecursiveExpression,
    point: &AstPoint,
    has_parent: bool,
    out: &mut Vec<CallItem>,
) {
    if let Some(next) = &ast_expression.next {
        match &next.operator {
            AstExpressionOperator::Plus(_)
            | AstExpressionOperator::PlusPlus(_)
            | AstExpressionOperator::Minus(_)
            | AstExpressionOperator::MinusMinus(_)
            | AstExpressionOperator::Equal(_)
            | AstExpressionOperator::NotEqual(_)
            | AstExpressionOperator::Multiply(_)
            | AstExpressionOperator::Devide(_)
            | AstExpressionOperator::Modulo(_)
            | AstExpressionOperator::Le(_)
            | AstExpressionOperator::Lt(_)
            | AstExpressionOperator::Ge(_)
            | AstExpressionOperator::Gt(_)
            | AstExpressionOperator::Ampersand(_)
            | AstExpressionOperator::AmpersandAmpersand(_)
            | AstExpressionOperator::VerticalBar(_)
            | AstExpressionOperator::VerticalBarVerticalBar(_) => {
                if let Some(ident) = &ast_expression.ident {
                    let a = dist(point, &ident_range(ident));
                    let b = dist(point, &next.range);

                    dbg!(a, b);
                    if a < b {
                        let mut has_args = false;
                        if let Some(n) = &ast_expression.next {
                            has_args = n.values.is_some();
                        }
                        cc_expr_ident(ident, has_args, false, point, out);
                    } else {
                        cc_expr_recursive(next.as_ref(), point, false, out);
                    }
                }
            }
            AstExpressionOperator::None
            | AstExpressionOperator::QuestionMark(_)
            | AstExpressionOperator::Colon(_)
            | AstExpressionOperator::Dot(_)
            | AstExpressionOperator::ExclemationMark(_) => {
                if let Some(ident) = &ast_expression.ident {
                    let mut has_args = false;
                    if let Some(n) = &ast_expression.next {
                        has_args = n.values.is_some();
                    }
                    cc_expr_ident(ident, has_args, has_parent, point, out);
                }
                if let Some(next) = &ast_expression.next {
                    cc_expr_recursive(next.as_ref(), point, true, out);
                }
                if let Some(values) = &ast_expression.values {
                    cc_arugments(point, out, values);
                }
            }
        }
    } else {
        if let Some(ident) = &ast_expression.ident {
            let mut has_args = false;
            if let Some(n) = &ast_expression.next {
                has_args = n.values.is_some();
            }
            cc_expr_ident(ident, has_args, has_parent, point, out);
        }
        if let Some(values) = &ast_expression.values {
            cc_arugments(point, out, values);
        }
    }
}

fn ident_range(ident: &AstExpressionIdentifier) -> AstRange {
    match ident {
        AstExpressionIdentifier::Identifier(ast_identifier) => ast_identifier.range,
        AstExpressionIdentifier::Nuget(ast_value_nuget) => match ast_value_nuget {
            AstValueNuget::Int(ast_int) => ast_int.range,
            AstValueNuget::Double(ast_double) => ast_double.range,
            AstValueNuget::Float(ast_double) => ast_double.range,
            AstValueNuget::StringLiteral(ast_identifier) => ast_identifier.range,
            AstValueNuget::CharLiteral(ast_identifier) => ast_identifier.range,
            AstValueNuget::BooleanLiteral(ast_boolean) => ast_boolean.range,
        },
        AstExpressionIdentifier::Value(ast_value) => fun_name(ast_value),
        AstExpressionIdentifier::ArrayAccess(ast_value) => fun_name(ast_value),
    }
}

fn fun_name(ast_value: &AstValue) -> AstRange {
    match ast_value {
        AstValue::Variable(ast_identifier) => ast_identifier.range,
        AstValue::Nuget(ast_value_nuget) => match ast_value_nuget {
            AstValueNuget::Int(ast_int) => ast_int.range,
            AstValueNuget::Double(ast_double) => ast_double.range,
            AstValueNuget::Float(ast_double) => ast_double.range,
            AstValueNuget::StringLiteral(ast_identifier) => ast_identifier.range,
            AstValueNuget::CharLiteral(ast_identifier) => ast_identifier.range,
            AstValueNuget::BooleanLiteral(ast_boolean) => ast_boolean.range,
        },
        AstValue::Array(ast_values) => ast_values.range,
        AstValue::NewClass(ast_new_class) => ast_new_class.range,
    }
}

fn cc_arugments(point: &AstPoint, out: &mut Vec<CallItem>, values: &AstValues) {
    if values.range.is_in_range(point) {
        let active_param = get_active_param(values, point);
        let mut filled_params: Vec<Vec<CallItem>> = values
            .values
            .iter()
            .map(|i| {
                let mut out = vec![];
                cc_expr(i, point, false, &mut out);
                out
            })
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

fn get_active_param(values: &AstValues, point: &AstPoint) -> usize {
    values
        .values
        .iter()
        .enumerate()
        .min_by_key(|(_, expression)| dist(point, ast_expression_get_range(expression)))
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
    point: &AstPoint,
    out: &mut Vec<CallItem>,
) {
    match ident {
        AstExpressionIdentifier::Identifier(ast_identifier) => {
            let is_empty = out.is_empty();
            if has_args {
                out.push(CallItem::MethodCall {
                    name: ast_identifier.into(),
                    range: ast_identifier.range,
                });
            } else if has_parent && !is_empty {
                out.push(CallItem::FieldAccess {
                    name: ast_identifier.into(),
                    range: ast_identifier.range,
                });
            } else {
                let val = match ast_identifier.value.as_str() {
                    "this" => CallItem::This {
                        range: ast_identifier.range,
                    },
                    _ => CallItem::ClassOrVariable {
                        name: ast_identifier.into(),
                        range: ast_identifier.range,
                    },
                };
                out.push(val);
            }
        }
        AstExpressionIdentifier::Nuget(ast_value_nuget) => cc_value_nuget(ast_value_nuget, out),
        AstExpressionIdentifier::Value(ast_value) => cc_value(ast_value, point, out),
        AstExpressionIdentifier::ArrayAccess(_ast_value) => todo!(),
    }
}

pub fn flatten_argument_lists(call_chain: &[CallItem]) -> Vec<CallItem> {
    let mut out = vec![];
    for ci in call_chain {
        if let CallItem::ArgumentList {
            prev,
            active_param,
            filled_params: _,
            range: _,
        } = ci
        {
            if active_param.is_none() {
                out.extend(prev.iter().map(Clone::clone));
            }
        } else {
            out.push(ci.clone());
        }
    }
    out
}
