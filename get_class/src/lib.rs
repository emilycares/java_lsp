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
        AstAnnotated, AstAnnotatedParameter, AstAnnotatedParameterKind, AstAnnotation, AstBlock,
        AstBlockEntry, AstClassBlock, AstEnumeration, AstExpressionIdentifier, AstExpressionKind,
        AstExpressionOrAnnotated, AstExpressionOrValue, AstFile, AstImportUnit, AstJType,
        AstJTypeKind, AstLambdaRhs, AstNewRhs, AstPoint, AstRange, AstRecursiveExpression,
        AstThing, AstValuesWithAnnotated,
    },
};
pub struct FoundClass {
    pub name: String,
    pub range: AstRange,
}

/// Get class name under cursor
#[must_use]
pub fn get_class(ast: &AstFile, point: &AstPoint) -> Option<FoundClass> {
    if let Some(imports) = &ast.imports
        && imports.range.is_in_range(point)
    {
        for im in &imports.imports {
            if !im.range.is_in_range(point) {
                continue;
            }
            let o = match &im.unit {
                AstImportUnit::StaticClass(ast_identifier)
                | AstImportUnit::StaticClassMethod(ast_identifier, _)
                | AstImportUnit::Class(ast_identifier) => Some(FoundClass {
                    name: ast_identifier.value.clone(),
                    range: ast_identifier.range,
                }),
                AstImportUnit::Prefix(_) | AstImportUnit::StaticPrefix(_) => None,
            };
            if o.is_some() {
                return o;
            }
        }
    }
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
        AstThing::Class(ast_class) => {
            if let Some(value) = get_class_annotated_vec(&ast_class.annotated, point) {
                return Some(value);
            }
            get_class_cblock(&ast_class.block, point)
        }
        AstThing::Record(ast_record) => {
            if let Some(value) = get_class_annotated_vec(&ast_record.annotated, point) {
                return Some(value);
            }
            get_class_cblock(&ast_record.block, point)
        }
        AstThing::Interface(interface) => get_class_interface(interface, point),
        AstThing::Enumeration(ast_enumeration) => get_class_enumeration(ast_enumeration, point),
        AstThing::Annotation(ast_annotation) => get_class_annotation(ast_annotation, point),
    }
}

fn get_class_annotation(annotation: &AstAnnotation, point: &AstPoint) -> Option<FoundClass> {
    if let Some(o) = get_class_annotated_vec(&annotation.annotated, point) {
        return Some(o);
    }
    for f in &annotation.fields {
        if let Some(value) = get_class_annotated_vec(&f.annotated, point) {
            return Some(value);
        }
        if let Some(o) = get_class_jtype(&f.jtype, point) {
            return Some(o);
        }
        if let Some(expr) = &f.expression
            && let Some(o) = get_class_expression(expr, point)
        {
            return Some(o);
        }
    }
    None
}

fn get_class_enumeration(enumeration: &AstEnumeration, point: &AstPoint) -> Option<FoundClass> {
    if let Some(value) = get_class_annotated_vec(&enumeration.annotated, point) {
        return Some(value);
    }
    for v in &enumeration.variables {
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
    for m in &enumeration.methods {
        if !m.range.is_in_range(point) {
            continue;
        }
        if let Some(value) = get_class_annotated_vec(&m.header.annotated, point) {
            return Some(value);
        }

        if let Some(o) = get_class_jtype(&m.header.jtype, point) {
            return Some(o);
        }
        if m.header.parameters.range.is_in_range(point) {
            for p in &m.header.parameters.parameters {
                if let Some(o) = get_class_annotated_vec(&p.annotated, point) {
                    return Some(o);
                }
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

fn get_class_annotated_vec(annotated: &[AstAnnotated], point: &AstPoint) -> Option<FoundClass> {
    for ano in annotated {
        if let Some(c) = get_class_annotated(ano, point) {
            return Some(c);
        }
    }
    None
}
fn get_class_annotated(annotated: &AstAnnotated, point: &AstPoint) -> Option<FoundClass> {
    if !annotated.range.is_in_range(point) {
        return None;
    }

    if let Some(c) = get_class_identifier(&annotated.name, point) {
        return Some(c);
    }
    match &annotated.parameters {
        AstAnnotatedParameterKind::None => None,
        AstAnnotatedParameterKind::Parameter(ast_annotated_parameters) => {
            get_class_annotated_parameters(ast_annotated_parameters, point)
        }
        AstAnnotatedParameterKind::Array(ast_values_with_annotated) => {
            get_class_values_with_annotated(ast_values_with_annotated, point)
        }
    }
}

fn get_class_values_with_annotated(
    ast_values_with_annotated: &AstValuesWithAnnotated,
    point: &AstPoint,
) -> Option<FoundClass> {
    if !ast_values_with_annotated.range.is_in_range(point) {
        return None;
    }
    for val in &ast_values_with_annotated.values {
        let o = match val {
            AstExpressionOrAnnotated::Expression(expression) => {
                get_class_expression(expression, point)
            }
            AstExpressionOrAnnotated::Annotated(ast_annotated) => {
                get_class_annotated(ast_annotated, point)
            }
        };
        if o.is_some() {
            return o;
        }
    }
    None
}

fn get_class_annotated_parameters(
    ast_annotated_parameters: &[AstAnnotatedParameter],
    point: &AstPoint,
) -> Option<FoundClass> {
    for p in ast_annotated_parameters {
        let o = match &p {
            AstAnnotatedParameter::NamedExpression {
                range: _,
                name: _,
                expression,
            }
            | AstAnnotatedParameter::Expression(expression) => {
                get_class_expression(expression, point)
            }
            AstAnnotatedParameter::Annotated(ast_annotated) => {
                get_class_annotated(ast_annotated, point)
            }
            AstAnnotatedParameter::NamedArray {
                range: _,
                name: _,
                values,
            } => get_class_values_with_annotated(values, point),
        };
        if o.is_some() {
            return o;
        }
    }
    None
}

fn get_class_interface(
    interface: &ast::types::AstInterface,
    point: &AstPoint,
) -> Option<FoundClass> {
    if let Some(value) = get_class_annotated_vec(&interface.annotated, point) {
        return Some(value);
    }
    if let Some(extends) = interface.extends.as_ref() {
        for p in &extends.parameters {
            if let Some(o) = get_class_jtype(p, point) {
                return Some(o);
            }
        }
    }
    for m in &interface.methods {
        if !m.range.is_in_range(point) {
            continue;
        }
        if let Some(value) = get_class_annotated_vec(&m.header.annotated, point) {
            return Some(value);
        }

        if let Some(throws) = &m.header.throws {
            for j in &throws.parameters {
                if let Some(o) = get_class_jtype(j, point) {
                    return Some(o);
                }
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
    }
    for m in &interface.default_methods {
        if !m.range.is_in_range(point) {
            continue;
        }
        if let Some(value) = get_class_annotated_vec(&m.header.annotated, point) {
            return Some(value);
        }

        if let Some(throws) = &m.header.throws {
            for j in &throws.parameters {
                if let Some(o) = get_class_jtype(j, point) {
                    return Some(o);
                }
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

        if let Some(b) = get_class_block(&m.block, point) {
            return Some(b);
        }
    }
    for v in &interface.constants {
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
    None
}

fn get_class_cblock(block: &AstClassBlock, point: &AstPoint) -> Option<FoundClass> {
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
        if let Some(value) = get_class_annotated_vec(&m.header.annotated, point) {
            return Some(value);
        }

        if let Some(o) = get_class_jtype(&m.header.jtype, point) {
            return Some(o);
        }
        if m.header.parameters.range.is_in_range(point) {
            for p in &m.header.parameters.parameters {
                if let Some(o) = get_class_annotated_vec(&p.annotated, point) {
                    return Some(o);
                }
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
                AstNewRhs::Parameters(_, ast_expressions) => {
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
                AstNewRhs::ParametersAndBlock(_, ast_expressions, ast_class_block) => {
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
        AstExpressionKind::Array(ast_values) => {
            if ast_values.range.is_in_range(point) {
                for i in &ast_values.values {
                    if let Some(o) = get_class_expression(i, point) {
                        return Some(o);
                    }
                }
            }
            None
        }
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
        AstJTypeKind::Class(ast_identifier) => {
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
