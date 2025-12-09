#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use ast::{
    range::AstInRange,
    types::{
        AstBlock, AstBlockEntry, AstExpressionIdentifier, AstExpressionKind, AstExpressionOrValue,
        AstFile, AstJType, AstJTypeKind, AstLambdaRhs, AstNewRhs, AstPoint, AstRange,
        AstRecursiveExpression, AstThing,
    },
};
pub struct FoundClass {
    pub name: String,
    pub range: AstRange,
}

/// Get class name under cursor
#[must_use]
pub fn get_class(ast: &AstFile, point: &AstPoint) -> Option<FoundClass> {
    things(&ast.things, point)
}
fn things(things: &[AstThing], point: &AstPoint) -> Option<FoundClass> {
    for th in things {
        if th.is_in_range(point)
            && let Some(t) = thing(th, point)
        {
            return Some(t);
        }
    }
    None
}

fn thing(thing: &AstThing, point: &AstPoint) -> Option<FoundClass> {
    match &thing {
        AstThing::Class(ast_class) => get_class_cblock(&ast_class.block, point),
        AstThing::Record(ast_record) => get_class_cblock(&ast_record.block, point),
        AstThing::Interface(_ast_interface) => todo!(),
        AstThing::Enumeration(_ast_enumeration) => todo!(),
        AstThing::Annotation(_ast_annotation) => todo!(),
    }
}

fn get_class_cblock(block: &ast::types::AstClassBlock, point: &AstPoint) -> Option<FoundClass> {
    for v in &block.variables {
        if !v.range.is_in_range(point) {
            continue;
        }

        if let Some(o) = get_class_jtype(&v.jtype, point) {
            return Some(o);
        }
        if let Some(ex) = &v.expression
            && let Some(o) = get_class_expression(ex, point)
        {
            return Some(o);
        }
    }
    for m in &block.methods {
        if !m.range.is_in_range(point) {
            continue;
        }
        for ano in &m.header.annotated {
            if !ano.range.is_in_range(point) {
                continue;
            }

            if let Some(c) = get_class_identifier(&ano.name, point) {
                return Some(c);
            }
        }

        if let Some(o) = get_class_jtype(&m.header.jtype, point) {
            return Some(o);
        }
        if m.header.parameters.range.is_in_range(point) {
            for p in &m.header.parameters.parameters {
                if let Some(o) = get_class_jtype(&p.jtype, point) {
                    return Some(o);
                }
            }
        }

        if let Some(block) = &m.block
            && let Some(b) = get_class_block(block, point)
        {
            return Some(b);
        }
    }
    None
}

fn get_class_block(block: &AstBlock, point: &AstPoint) -> Option<FoundClass> {
    if !block.range.is_in_range(point) {
        return None;
    }
    for entry in &block.entries {
        match entry {
            AstBlockEntry::Return(ast_block_return) => {
                if let Some(o) = get_class_expression_or_value(&ast_block_return.expression, point)
                {
                    return Some(o);
                }
            }
            AstBlockEntry::Variable(_ast_block_variable) => (),
            AstBlockEntry::Expression(ast_block_expression) => {
                if let Some(o) = get_class_expression(&ast_block_expression.value, point) {
                    return Some(o);
                }
            }
            AstBlockEntry::Assign(ast_block_assign) => {
                if let Some(o) = get_class_expression(&ast_block_assign.expression, point) {
                    return Some(o);
                }
            }
            AstBlockEntry::If(_ast_if) => (),
            AstBlockEntry::While(_ast_while) => (),
            AstBlockEntry::For(_ast_for) => (),
            AstBlockEntry::ForEnhanced(_ast_for_enhanced) => (),
            AstBlockEntry::Break(_ast_block_break) => (),
            AstBlockEntry::Continue(_ast_block_continue) => (),
            AstBlockEntry::Switch(_ast_switch) => (),
            AstBlockEntry::SwitchCase(_ast_switch_case) => (),
            AstBlockEntry::SwitchDefault(_ast_switch_default) => (),
            AstBlockEntry::SwitchCaseArrowValues(_ast_switch_case_arrow) => (),
            AstBlockEntry::SwitchCaseArrowDefault(_ast_switch_case_arrow_default) => (),
            AstBlockEntry::TryCatch(_ast_try_catch) => (),
            AstBlockEntry::Throw(_ast_throw) => (),
            AstBlockEntry::Yield(_ast_block_yield) => (),
            AstBlockEntry::SynchronizedBlock(ast_synchronized_block) => {
                if let Some(o) = get_class_expression(&ast_synchronized_block.expression, point) {
                    return Some(o);
                }
                if let Some(o) = get_class_block(&ast_synchronized_block.block, point) {
                    return Some(o);
                }
            }
            AstBlockEntry::Thing(ast_thing) => {
                if let Some(o) = thing(ast_thing, point) {
                    return Some(o);
                }
            }
            AstBlockEntry::InlineBlock(ast_block) => {
                if let Some(o) = get_class_block(&ast_block.block, point) {
                    return Some(o);
                }
            }
            AstBlockEntry::Semicolon(_ast_range) => (),
            AstBlockEntry::SwitchCaseArrowType(ast_switch_case_arrow_type) => {
                if let Some(o) = get_class_jtype(&ast_switch_case_arrow_type.var.jtype, point) {
                    return Some(o);
                }
            }
            AstBlockEntry::Assert(ast_block_assert) => {
                return get_class_expression(&ast_block_assert.expression, point);
            }
        }
    }
    None
}

fn get_class_expression_or_value(
    expression: &AstExpressionOrValue,
    point: &AstPoint,
) -> Option<FoundClass> {
    match expression {
        AstExpressionOrValue::None => None,
        AstExpressionOrValue::Expression(ast_expression) => {
            get_class_expression(ast_expression, point)
        }
        AstExpressionOrValue::Value(_ast_value) => None,
    }
}

fn get_class_expression(
    ast_expression: &[AstExpressionKind],
    point: &AstPoint,
) -> Option<FoundClass> {
    for e in ast_expression {
        if let Some(c) = get_class_expression_kind(e, point) {
            return Some(c);
        }
    }
    None
}

fn get_class_expression_kind(ex: &AstExpressionKind, point: &AstPoint) -> Option<FoundClass> {
    match &ex {
        AstExpressionKind::Casted(ast_casted_expression)
        | AstExpressionKind::JType(ast_casted_expression) => {
            if !ast_casted_expression.range.is_in_range(point) {
                return None;
            }
            if let Some(o) = get_class_jtype(&ast_casted_expression.cast, point) {
                return Some(o);
            }
            None
        }
        AstExpressionKind::Recursive(ast_recursive_expression) => {
            get_class_recursive_expression(ast_recursive_expression, point)
        }
        AstExpressionKind::Lambda(ast_lambda) => {
            if !ast_lambda.range.is_in_range(point) {
                return None;
            }
            match &ast_lambda.rhs {
                AstLambdaRhs::None => None,
                AstLambdaRhs::Block(ast_block) => get_class_block(ast_block, point),
                AstLambdaRhs::Expr(ast_expression) => get_class_expression(ast_expression, point),
            }
        }
        AstExpressionKind::InlineSwitch(_ast_switch) => None,
        AstExpressionKind::NewClass(ast_new_class) => {
            if !ast_new_class.range.is_in_range(point) {
                return None;
            }
            if let Some(jt) = get_class_jtype(&ast_new_class.jtype, point) {
                return Some(jt);
            }
            match &*ast_new_class.rhs {
                AstNewRhs::None => None,
                AstNewRhs::Parameters(ast_expressions) => {
                    for ex in ast_expressions {
                        if let Some(e) = get_class_expression(ex, point) {
                            return Some(e);
                        }
                    }
                    None
                }
                AstNewRhs::ArrayParameters(ast_expressions) => {
                    for ex in ast_expressions.iter().flatten() {
                        if let Some(e) = get_class_expression(ex, point) {
                            return Some(e);
                        }
                    }
                    None
                }
                AstNewRhs::Block(ast_class_block) => get_class_cblock(ast_class_block, point),
                AstNewRhs::ParametersAndBlock(ast_expressions, ast_class_block) => {
                    for ex in ast_expressions {
                        if let Some(e) = get_class_expression(ex, point) {
                            return Some(e);
                        }
                    }
                    if let Some(e) = get_class_cblock(ast_class_block, point) {
                        return Some(e);
                    }
                    None
                }
                AstNewRhs::Array(ast_values) => {
                    for ex in &ast_values.values {
                        if let Some(e) = get_class_expression(ex, point) {
                            return Some(e);
                        }
                    }
                    None
                }
            }
        }
        AstExpressionKind::Generics(ast_generics) => {
            for j in &ast_generics.jtypes {
                if let Some(o) = get_class_jtype(j, point) {
                    return Some(o);
                }
            }
            None
        }
        AstExpressionKind::InstanceOf(instance) => {
            if let Some(o) = get_class_jtype(&instance.jtype, point) {
                return Some(o);
            }
            None
        }
        AstExpressionKind::Array(_ast_values) => todo!(),
    }
}

fn get_class_recursive_expression(
    expression: &AstRecursiveExpression,
    point: &AstPoint,
) -> Option<FoundClass> {
    if !expression.range.is_in_range(point) {
        return None;
    }
    loop {
        if let Some(ident) = &expression.ident
            && let Some(i) = get_class_expression_identifier(ident, point)
        {
            return Some(i);
        } else if let Some(vals) = &expression.values
            && vals.range.is_in_range(point)
        {
            for val in &vals.values {
                if let Some(s) = get_class_expression(val, point) {
                    return Some(s);
                }
            }
        } else {
            break;
        }
    }
    None
}

fn get_class_jtype(jtype: &AstJType, point: &AstPoint) -> Option<FoundClass> {
    if !jtype.range.is_in_range(point) {
        return None;
    }
    match &jtype.value {
        AstJTypeKind::Void
        | AstJTypeKind::Byte
        | AstJTypeKind::Char
        | AstJTypeKind::Double
        | AstJTypeKind::Float
        | AstJTypeKind::Int
        | AstJTypeKind::Long
        | AstJTypeKind::Short
        | AstJTypeKind::Boolean
        | AstJTypeKind::Wildcard
        | AstJTypeKind::Var => None,
        AstJTypeKind::Parameter(ast_identifier) | AstJTypeKind::Class(ast_identifier) => {
            if !ast_identifier.range.is_in_range(point) {
                return None;
            }
            Some(FoundClass {
                name: ast_identifier.value.clone(),
                range: ast_identifier.range,
            })
        }
        AstJTypeKind::Array(ast_jtype) => get_class_jtype(ast_jtype, point),
        AstJTypeKind::Generic(ast_identifier, ast_jtypes) => {
            if let Some(value) = get_class_identifier(ast_identifier, point) {
                return Some(value);
            }
            for jt in ast_jtypes {
                if let Some(j) = get_class_jtype(jt, point) {
                    return Some(j);
                }
            }
            None
        }
        AstJTypeKind::Access { base, inner } => {
            if let Some(j) = get_class_jtype(base, point) {
                return Some(j);
            }
            if let Some(j) = get_class_jtype(inner, point) {
                return Some(j);
            }
            None
        }
    }
}

fn get_class_identifier(
    ast_identifier: &ast::types::AstIdentifier,
    point: &AstPoint,
) -> Option<FoundClass> {
    if ast_identifier.range.is_in_range(point) {
        return Some(FoundClass {
            name: ast_identifier.value.clone(),
            range: ast_identifier.range,
        });
    }
    None
}
fn get_class_expression_identifier(
    ast_identifier: &AstExpressionIdentifier,
    point: &AstPoint,
) -> Option<FoundClass> {
    match ast_identifier {
        AstExpressionIdentifier::Identifier(ast_identifier) => {
            get_class_identifier(ast_identifier, point)
        }
        AstExpressionIdentifier::Nuget(_ast_value_nuget) => None,
        AstExpressionIdentifier::Value(_ast_value) => None,
        AstExpressionIdentifier::ArrayAccess(_ast_value) => None,
        AstExpressionIdentifier::EmptyArrayAccess => None,
    }
}
