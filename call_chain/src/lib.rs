#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
use std::cmp::{self, max, min};

use ast::range::{AstInRange, GetRange, add_ranges};
use ast::types::{
    AstAnnotated, AstAnnotatedParameter, AstAnnotatedParameterKind, AstBlock, AstBlockEntry,
    AstBlockVariable, AstCastedExpression, AstClassBlock, AstExpressionIdentifier,
    AstExpressionKind, AstExpressionOperator, AstExpressionOrDefault, AstExpressionOrValue,
    AstExpresssionOrAnnotated, AstFile, AstForContent, AstIdentifier, AstIf, AstIfContent,
    AstJType, AstJTypeKind, AstLambdaRhs, AstNewClass, AstNewRhs, AstPoint, AstRange,
    AstRecursiveExpression, AstSwitchCaseArrowContent, AstThing, AstValue, AstValueNuget,
    AstValues, AstValuesWithAnnotated, AstWhileContent,
};
use my_string::MyString;

#[derive(Debug, PartialEq, Clone)]
pub enum CallItem {
    MethodCall {
        name: MyString,
        range: AstRange,
    },
    FieldAccess {
        name: MyString,
        range: AstRange,
    },
    Variable {
        name: MyString,
        range: AstRange,
    },
    This {
        range: AstRange,
    },
    Package {
        name: MyString,
        range: AstRange,
    },
    Class {
        name: MyString,
        range: AstRange,
    },
    ClassOrVariable {
        name: MyString,
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
            CallItem::Package { name: _, range } => range,
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
    cc_things(&ast.things, point, &mut out);
    out
}
fn cc_things(things: &[AstThing], point: &AstPoint, out: &mut Vec<CallItem>) {
    for thing in things {
        if thing.is_in_range(point) {
            cc_thing(thing, point, out);
            break;
        }
    }
}

fn cc_thing(thing: &AstThing, point: &AstPoint, out: &mut Vec<CallItem>) {
    match &thing {
        AstThing::Class(ast_class) => {
            cc_class_block(&ast_class.block, point, out);
        }
        AstThing::Record(ast_record) => {
            cc_class_block(&ast_record.block, point, out);
        }
        AstThing::Interface(ast_interface) => ast_interface
            .default_methods
            .iter()
            .filter(|i| i.range.is_in_range(point))
            .for_each(|i| {
                if i.block.range.is_in_range(point) {
                    cc_block(&i.block, point, out)
                } else {
                    cc_annotated(&i.annotated, point, out)
                }
            }),
        AstThing::Enumeration(_) => todo!(),
        AstThing::Annotation(_) => todo!(),
    }
}

fn cc_class_block(block: &AstClassBlock, point: &AstPoint, out: &mut Vec<CallItem>) {
    block
        .variables
        .iter()
        .filter(|i| i.range.is_in_range(point))
        .for_each(|i| {
            if let Some(expr) = &i.expression {
                cc_expr(expr, point, false, out)
            }
        });
    block
        .methods
        .iter()
        .filter(|i| i.range.is_in_range(point))
        .for_each(|i| {
            if let Some(block) = &i.block
                && block.range.is_in_range(point)
            {
                cc_block(block, point, out)
            } else {
                cc_annotated(&i.header.annotated, point, out)
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
                cc_annotated(&i.header.annotated, point, out)
            }
        });
}

fn cc_annotated(annotated: &[AstAnnotated], point: &AstPoint, out: &mut Vec<CallItem>) {
    if let Some(a) = annotated.iter().find(|i| i.range.is_in_range(point)) {
        cc_annotated_single(a, point, out)
    }
}
fn cc_annotated_single(annotated: &AstAnnotated, point: &AstPoint, out: &mut Vec<CallItem>) {
    match &annotated.parameters {
        AstAnnotatedParameterKind::None => (),
        AstAnnotatedParameterKind::Parameter(ast_annotated_parameters) => {
            cc_annotated_parameter(ast_annotated_parameters, point, out);
        }
        AstAnnotatedParameterKind::Array(ast_values) => {
            cc_array_with_annotated(ast_values, point, out)
        }
    }
}

fn cc_annotated_parameter(
    parameters: &[AstAnnotatedParameter],
    point: &AstPoint,
    out: &mut Vec<CallItem>,
) {
    let param = parameters
        .iter()
        .min_by_key(|annotated_parameter| dist(*point, annotated_parameter.get_range()));

    if let Some(p) = param {
        match p {
            AstAnnotatedParameter::Expression(ast_expression) => {
                cc_expr(ast_expression, point, false, out);
            }
            AstAnnotatedParameter::NamedExpression {
                range: _,
                name: _,
                expression,
            } => {
                cc_expr(expression, point, false, out);
            }
            AstAnnotatedParameter::Annotated(ast_annotated) => {
                cc_annotated_single(ast_annotated, point, out)
            }
            AstAnnotatedParameter::NamedArray {
                range: _,
                name: _,
                values,
            } => {
                cc_array_with_annotated(values, point, out);
            }
        }
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
            CallItem::Package { name: _, range } => range.is_in_range(point),
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
        cc_block_entry(entry, point, out);
    }
}

fn dist_block_entry(point: &AstPoint, entry: &AstBlockEntry) -> usize {
    dist(*point, entry.get_range())
}

fn cc_block_entry(entry: &AstBlockEntry, point: &AstPoint, out: &mut Vec<CallItem>) {
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
            for v in ast_block_variable {
                cc_block_variable(v, point, out);
            }
        }
        AstBlockEntry::Expression(ast_block_expression) => {
            cc_expr(&ast_block_expression.value, point, false, out)
        }
        AstBlockEntry::Assign(ast_block_assign) => {
            let a = dist(*point, ast_block_assign.key.get_range());
            let b = dist(*point, ast_block_assign.expression.get_range());
            if a > b {
                cc_expr(&ast_block_assign.expression, point, false, out);
            } else {
                cc_expr(&ast_block_assign.key, point, false, out);
            }
        }
        AstBlockEntry::If(ast_if) => cc_if(ast_if, point, out),
        AstBlockEntry::While(ast_while) => {
            if ast_while.control.get_range().is_in_range(point) {
                return cc_expr(&ast_while.control, point, false, out);
            }
            cc_while_content(&ast_while.content, point, out);
        }
        AstBlockEntry::For(ast_for) => {
            for e in &ast_for.vars {
                cc_block_entry(e, point, out);
            }
            for e in &ast_for.check {
                cc_block_entry(e, point, out);
            }
            for e in &ast_for.changes {
                cc_block_entry(e, point, out);
            }
            if (&ast_for.content).is_in_range(point) {
                cc_for_content(&ast_for.content, point, out)
            }
        }
        AstBlockEntry::ForEnhanced(ast_for_enhanced) => {
            for v in &ast_for_enhanced.var {
                cc_block_variable(v, point, out);
            }
            cc_expr(&ast_for_enhanced.rhs, point, false, out);
            if (&ast_for_enhanced.content).is_in_range(point) {
                cc_for_content(&ast_for_enhanced.content, point, out)
            }
        }
        AstBlockEntry::Break(_ast_block_break) => (),
        AstBlockEntry::Continue(_ast_block_continue) => (),
        AstBlockEntry::Switch(ast_switch) => {
            cc_expr(&ast_switch.check, point, false, out);
            if ast_switch.block.range.is_in_range(point) {
                cc_block(&ast_switch.block, point, out)
            }
        }
        AstBlockEntry::SwitchCase(ast_switch_case) => {
            for ex in &ast_switch_case.expressions {
                if let AstExpressionOrDefault::Expression(ex) = ex {
                    cc_expr(ex, point, false, out);
                }
            }
        }
        AstBlockEntry::SwitchDefault(_ast_switch_default) => (),
        AstBlockEntry::TryCatch(ast_try_catch) => {
            if let Some(res) = &ast_try_catch.resources_block {
                cc_block(res, point, out);
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
                cc_block(res, point, out);
            }
        }
        AstBlockEntry::Throw(ast_throw) => cc_expr(&ast_throw.expression, point, false, out),
        AstBlockEntry::SynchronizedBlock(ast_synchronized_block) => {
            cc_expr(&ast_synchronized_block.expression, point, false, out);
            cc_block(&ast_synchronized_block.block, point, out);
        }
        AstBlockEntry::SwitchCaseArrowValues(ast_switch_case_arrow) => {
            cc_swtich_case_arrow_content(&ast_switch_case_arrow.content, point, out)
        }
        AstBlockEntry::SwitchCaseArrowDefault(ast_switch_case_arrow_default) => {
            cc_swtich_case_arrow_content(&ast_switch_case_arrow_default.content, point, out)
        }
        AstBlockEntry::SwitchCaseArrowType(ast_switch_case_arrow_type) => {
            cc_jtype(&ast_switch_case_arrow_type.var.jtype, out);
            cc_swtich_case_arrow_content(&ast_switch_case_arrow_type.content, point, out)
        }
        AstBlockEntry::Thing(ast_thing) => cc_thing(ast_thing, point, out),
        AstBlockEntry::InlineBlock(ast_block) => cc_block(&ast_block.block, point, out),
        AstBlockEntry::Semicolon(_ast_range) => (),
        AstBlockEntry::Assert(ast_block_assert) => {
            cc_expr(&ast_block_assert.expression, point, false, out)
        }
    }
}

fn cc_swtich_case_arrow_content(
    content: &AstSwitchCaseArrowContent,
    point: &AstPoint,
    out: &mut Vec<CallItem>,
) {
    match content {
        AstSwitchCaseArrowContent::Block(ast_block) => cc_block(ast_block, point, out),
        AstSwitchCaseArrowContent::Entry(ast_block_entry) => {
            cc_block_entry(ast_block_entry, point, out)
        }
    }
}

fn cc_block_variable(
    ast_block_variable: &AstBlockVariable,
    point: &AstPoint,
    out: &mut Vec<CallItem>,
) {
    if let Some(ref expression) = ast_block_variable.value {
        cc_expr(expression, point, false, out)
    }
}

fn cc_if(ast_if: &AstIf, point: &AstPoint, out: &mut Vec<CallItem>) {
    match ast_if {
        AstIf::If {
            range,
            control,
            control_range,
            content,
        } => {
            if !range.is_in_range(point) {
                return;
            }
            if control_range.is_in_range(point) {
                return cc_expr(control, point, false, out);
            }
            if content.is_in_range(point) {
                cc_if_content(content, point, out)
            }
        }
        AstIf::Else { range, content } => {
            if !range.is_in_range(point) {
                return;
            }
            cc_if_content(content, point, out)
        }
        AstIf::ElseIf {
            range,
            control,
            control_range,
            content,
        } => {
            if !range.is_in_range(point) {
                return;
            }
            if control_range.is_in_range(point) {
                return cc_expr(control, point, false, out);
            }
            if content.is_in_range(point) {
                cc_if_content(content, point, out)
            }
        }
    }
}

fn cc_if_content(content: &AstIfContent, point: &AstPoint, out: &mut Vec<CallItem>) {
    match content {
        AstIfContent::Block(ast_block) => cc_block(ast_block, point, out),
        AstIfContent::BlockEntry(ast_block_entry) => cc_block_entry(ast_block_entry, point, out),
    }
}
fn cc_for_content(content: &AstForContent, point: &AstPoint, out: &mut Vec<CallItem>) {
    match content {
        AstForContent::Block(ast_block) => cc_block(ast_block, point, out),
        AstForContent::BlockEntry(ast_block_entry) => cc_block_entry(ast_block_entry, point, out),
        AstForContent::None => (),
    }
}
fn cc_while_content(content: &AstWhileContent, point: &AstPoint, out: &mut Vec<CallItem>) {
    match content {
        AstWhileContent::Block(ast_block) => cc_block(ast_block, point, out),
        AstWhileContent::BlockEntry(ast_block_entry) => cc_block_entry(ast_block_entry, point, out),
        AstWhileContent::None => (),
    }
}

fn cc_value(value: &AstValue, _point: &AstPoint, out: &mut Vec<CallItem>) {
    match value {
        AstValue::Variable(ast_identifier) => cc_variable(ast_identifier, out),
        AstValue::Nuget(ast_nuget) => cc_value_nuget(ast_nuget, out),
    }
}

fn cc_new_class(ast_new_class: &AstNewClass, point: &AstPoint, out: &mut Vec<CallItem>) {
    if let AstJTypeKind::Class(c) = &ast_new_class.jtype.value {
        out.push(CallItem::Class {
            name: c.value.clone(),
            range: ast_new_class.range,
        });
    }
    if ast_new_class.range.is_in_range(point) {
        match ast_new_class.rhs.as_ref() {
            AstNewRhs::None => (),
            AstNewRhs::ArrayParameters(ast_expressions) => ast_expressions
                .iter()
                .flatten()
                .for_each(|i| cc_expr(i, point, false, out)),
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
fn cc_array_with_annotated(
    ast_values: &AstValuesWithAnnotated,
    point: &AstPoint,
    out: &mut Vec<CallItem>,
) {
    if !ast_values.range.is_in_range(point) {
        return;
    }

    ast_values
        .values
        .iter()
        .filter_map(|i| match i {
            AstExpresssionOrAnnotated::Expression(ast_expression) => Some(ast_expression),
            AstExpresssionOrAnnotated::Annotated(_) => None,
        })
        .for_each(|i| cc_expr(i, point, false, out))
}
fn cc_value_nuget(ast_nuget: &AstValueNuget, out: &mut Vec<CallItem>) {
    match ast_nuget {
        AstValueNuget::Int(ast_number) => out.push(CallItem::Class {
            name: "Integer".into(),
            range: ast_number.range,
        }),
        AstValueNuget::HexLiteral(hex) => out.push(CallItem::Class {
            name: "Integer".into(),
            range: hex.range,
        }),
        AstValueNuget::BinaryLiteral(hex) => out.push(CallItem::Class {
            name: "Integer".into(),
            range: hex.range,
        }),
        AstValueNuget::Double(ast_double) => out.push(CallItem::Class {
            name: "Double".into(),
            range: ast_double.range,
        }),
        AstValueNuget::Float(float) => out.push(CallItem::Class {
            name: "Float".into(),
            range: float.range,
        }),
        AstValueNuget::StringLiteral(ast_identifier) => out.push(CallItem::Class {
            name: "String".into(),
            range: ast_identifier.range,
        }),
        AstValueNuget::CharLiteral(char) => out.push(CallItem::Class {
            name: "Char".into(),
            range: char.range,
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
    ast_expression: &[AstExpressionKind],
    point: &AstPoint,
    has_parent: bool,
    out: &mut Vec<CallItem>,
) {
    let Some(ex) = ast_expression.first() else {
        return;
    };
    if let AstExpressionKind::Recursive(current) = ex {
        cc_expr_recursive(&ast_expression[0..], current, point, has_parent, out);
        return;
    }
    match ex {
        AstExpressionKind::Recursive(_) => {}
        AstExpressionKind::Lambda(ast_lambda) => match &ast_lambda.rhs {
            AstLambdaRhs::None => (),
            AstLambdaRhs::Block(ast_block) => cc_block(ast_block, point, out),
            AstLambdaRhs::Expr(ast_base_expression) => {
                cc_expr(ast_base_expression, point, has_parent, out)
            }
        },
        AstExpressionKind::InlineSwitch(_ast_switch) => (),
        AstExpressionKind::NewClass(ast_new_class) => cc_new_class(ast_new_class, point, out),
        AstExpressionKind::Array(ast_values) => cc_array(ast_values, point, out),
        AstExpressionKind::Generics(ast_generics) => {
            for j in &ast_generics.jtypes {
                cc_jtype(j, out)
            }
        }
        AstExpressionKind::Casted(c) => cc_casted(c, point, out),
        AstExpressionKind::JType(_) => (),
        AstExpressionKind::InstanceOf(ast_instance_of) => cc_jtype(&ast_instance_of.jtype, out),
    }
    cc_expr(&ast_expression[1..], point, has_parent, out);
}

fn cc_jtype(jtype: &AstJType, out: &mut Vec<CallItem>) {
    match &jtype.value {
        AstJTypeKind::Byte
        | AstJTypeKind::Char
        | AstJTypeKind::Double
        | AstJTypeKind::Float
        | AstJTypeKind::Int
        | AstJTypeKind::Long
        | AstJTypeKind::Short
        | AstJTypeKind::Boolean
        | AstJTypeKind::Var
        | AstJTypeKind::Wildcard
        | AstJTypeKind::Void => out.push(CallItem::Class {
            name: jtype.value.to_string(),
            range: jtype.range,
        }),
        AstJTypeKind::Class(ast_identifier) => out.push(CallItem::Class {
            name: ast_identifier.value.clone(),
            range: jtype.range,
        }),
        AstJTypeKind::Array(ast_jtype) => cc_jtype(ast_jtype, out),
        AstJTypeKind::Generic(ast_identifier, _ast_jtypes) => out.push(CallItem::Class {
            name: ast_identifier.value.clone(),
            range: jtype.range,
        }),
        AstJTypeKind::Parameter(_ast_identifier) => todo!("call_chain jtype parameter"),
        AstJTypeKind::Access { base, inner } => {
            cc_jtype(base, out);
            cc_jtype(inner, out);
        }
    }
}
/// Only used in casted expression
fn cc_jtype_not_sure_class(jtype: &AstJType, out: &mut Vec<CallItem>) {
    match &jtype.value {
        AstJTypeKind::Byte
        | AstJTypeKind::Char
        | AstJTypeKind::Double
        | AstJTypeKind::Float
        | AstJTypeKind::Int
        | AstJTypeKind::Long
        | AstJTypeKind::Short
        | AstJTypeKind::Boolean
        | AstJTypeKind::Var
        | AstJTypeKind::Wildcard
        | AstJTypeKind::Void => out.push(CallItem::Class {
            name: jtype.value.to_string(),
            range: jtype.range,
        }),
        AstJTypeKind::Class(ast_identifier) => out.push(CallItem::ClassOrVariable {
            name: ast_identifier.value.clone(),
            range: jtype.range,
        }),
        AstJTypeKind::Array(ast_jtype) => cc_jtype(ast_jtype, out),
        AstJTypeKind::Generic(ast_identifier, _ast_jtypes) => out.push(CallItem::ClassOrVariable {
            name: ast_identifier.value.clone(),
            range: jtype.range,
        }),
        AstJTypeKind::Parameter(_ast_identifier) => todo!("call_chain jtype parameter"),
        AstJTypeKind::Access { base, inner } => {
            cc_jtype_not_sure_class(base, out);
            cc_jtype_not_sure_class(inner, out);
        }
    }
}

fn cc_expr_recursive(
    ast_expression: &[AstExpressionKind],
    current: &AstRecursiveExpression,
    point: &AstPoint,
    has_parent: bool,
    out: &mut Vec<CallItem>,
) {
    if let Some(next) = &ast_expression.get(1) {
        if let AstExpressionKind::Recursive(next) = next {
            cc_recursive_next_oprerator(ast_expression, current, point, has_parent, out, next);
        } else {
            let has_values = match next {
                AstExpressionKind::Casted(_) => true,
                AstExpressionKind::Recursive(r) => r.values.is_some(),
                AstExpressionKind::Lambda(_)
                | AstExpressionKind::InlineSwitch(_)
                | AstExpressionKind::NewClass(_)
                | AstExpressionKind::Generics(_)
                | AstExpressionKind::Array(_)
                | AstExpressionKind::InstanceOf(_)
                | AstExpressionKind::JType(_) => false,
            };
            cc_recursive_no_next(current, point, has_parent, has_values, out);
            cc_expr(&ast_expression[1..], point, has_parent, out);
        }
    } else {
        cc_recursive_no_next(current, point, has_parent, false, out);
        cc_expr(&ast_expression[1..], point, has_parent, out);
    }
}

fn cc_recursive_next_oprerator(
    ast_expression: &[AstExpressionKind],
    current: &AstRecursiveExpression,
    point: &AstPoint,
    has_parent: bool,
    out: &mut Vec<CallItem>,
    next: &AstRecursiveExpression,
) {
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
        | AstExpressionOperator::Tilde(_)
        | AstExpressionOperator::Caret(_)
        | AstExpressionOperator::Ampersand(_)
        | AstExpressionOperator::AmpersandAmpersand(_)
        | AstExpressionOperator::ColonColon(_)
        | AstExpressionOperator::VerticalBar(_)
        | AstExpressionOperator::VerticalBarVerticalBar(_) => {
            if let Some(ident) = &current.ident {
                let a = dist(*point, ident.get_range());
                let b = dist(*point, next.range);

                if a < b {
                    let has_args = next.values.is_some();
                    cc_expr_ident(ident, has_args, false, point, out);
                } else {
                    cc_expr(&ast_expression[1..], point, false, out);
                }
            }
        }
        AstExpressionOperator::None
        | AstExpressionOperator::QuestionMark(_)
        | AstExpressionOperator::Colon(_)
        | AstExpressionOperator::Dot(_)
        | AstExpressionOperator::ExclemationMark(_) => {
            if let Some(ident) = &current.ident {
                let has_args = next.values.is_some();
                cc_expr_ident(ident, has_args, has_parent, point, out);
            }
            cc_expr(&ast_expression[1..], point, true, out);
            if let Some(values) = &current.values {
                cc_arugments(point, out, values);
            }
        }
        AstExpressionOperator::Assign(_) => {
            if let Some(ident) = &current.ident {
                let a = dist(*point, ident.get_range());
                let b = dist(*point, next.range);

                if a <= b {
                    let has_args = next.values.is_some();
                    cc_expr_ident(ident, has_args, true, point, out);
                } else {
                    cc_expr(&ast_expression[1..], point, true, out);
                }
            }
        }
    }
}

fn cc_recursive_no_next(
    current: &AstRecursiveExpression,
    point: &AstPoint,
    has_parent: bool,
    has_values: bool,
    out: &mut Vec<CallItem>,
) {
    match (&current.ident, &current.values) {
        (None, None) => (),
        (None, Some(values)) => {
            cc_arugments(point, out, values);
        }
        (Some(ident), None) => {
            cc_expr_ident(ident, has_values, has_parent, point, out);
        }
        (Some(ident), Some(values)) => {
            if ident.get_range().is_contained_in(&values.get_range()) {
                cc_expr_ident(ident, has_values, has_parent, point, out);
            }
            cc_arugments(point, out, values);
        }
    }
}
fn cc_casted(casted: &AstCastedExpression, point: &AstPoint, out: &mut Vec<CallItem>) {
    if !casted.range.is_in_range(point) {
        return;
    }
    let mut inner = vec![];

    cc_jtype_not_sure_class(&casted.cast, &mut inner);
    let args = CallItem::ArgumentList {
        prev: out.clone(),
        active_param: Some(0),
        filled_params: vec![inner.clone()],
        range: casted.range,
    };
    out.clear();

    out.push(args);
    out.extend(inner);
}

fn cc_arugments(point: &AstPoint, out: &mut Vec<CallItem>, values: &AstValues) {
    if !values.range.is_in_range(point) {
        return;
    }
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

fn get_active_param(values: &AstValues, point: &AstPoint) -> usize {
    values
        .values
        .iter()
        .enumerate()
        .min_by_key(|(_, expression)| dist(*point, expression.get_range()))
        .map(|i| i.0)
        .unwrap_or_default()
}

fn dist(point: AstPoint, range: AstRange) -> usize {
    if point < range.start {
        line_col_diff(&point, &range.start)
    } else if point > range.end {
        line_col_diff(&point, &range.end)
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
        AstExpressionIdentifier::ArrayAccess(arrayaccess) => {
            cc_expr(arrayaccess, point, has_parent, out)
        }
        AstExpressionIdentifier::EmptyArrayAccess => (),
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
