use ast::types::{
    AstBlock, AstBlockEntry, AstExpression, AstExpressionIdentifier, AstFile, AstJType,
    AstJTypeKind, AstLambdaRhs, AstNewRhs, AstPoint, AstRange, AstRecursiveExpression, AstThing,
};
pub struct FoundClass {
    pub name: String,
    pub range: AstRange,
}

/// Get class name under cursor
pub fn get_class(ast: &AstFile, point: &AstPoint) -> Option<FoundClass> {
    match &ast.thing {
        AstThing::Class(ast_class) => get_class_cblock(&ast_class.block, point),
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
        for ano in &m.annotated {
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

        if let Some(b) = get_class_block(&m.block, point) {
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
            AstBlockEntry::Return(_ast_block_return) => todo!(),
            AstBlockEntry::Variable(_ast_block_variable) => todo!(),
            AstBlockEntry::Expression(_ast_block_expression) => todo!(),
            AstBlockEntry::Assign(_ast_block_assign) => todo!(),
            AstBlockEntry::If(_ast_if) => todo!(),
            AstBlockEntry::While(_ast_while) => todo!(),
            AstBlockEntry::For(_ast_for) => todo!(),
            AstBlockEntry::ForEnhanced(_ast_for_enhanced) => todo!(),
            AstBlockEntry::Break(_ast_block_break) => todo!(),
            AstBlockEntry::Continue(_ast_block_continue) => todo!(),
            AstBlockEntry::Switch(_ast_switch) => todo!(),
            AstBlockEntry::SwitchCase(_ast_switch_case) => todo!(),
            AstBlockEntry::SwitchDefault(_ast_switch_default) => todo!(),
            AstBlockEntry::TryCatch(_ast_try_catch) => todo!(),
            AstBlockEntry::Throw(_ast_throw) => todo!(),
            AstBlockEntry::SwitchCaseArrow(_ast_switch_case_arrow) => todo!(),
            AstBlockEntry::Yield(_ast_block_yield) => todo!(),
        }
    }
    None
}

fn get_class_expression(ex: &AstExpression, point: &AstPoint) -> Option<FoundClass> {
    match &ex {
        AstExpression::Casted(ast_casted_expression) => {
            if !ast_casted_expression.range.is_in_range(point) {
                return None;
            }
            if let Some(o) = get_class_jtype(&ast_casted_expression.cast, point) {
                return Some(o);
            }
            if let Some(o) =
                get_class_recursive_expression(&ast_casted_expression.expression, point)
            {
                return Some(o);
            }
            None
        }
        AstExpression::Recursive(ast_recursive_expression) => {
            get_class_recursive_expression(ast_recursive_expression, point)
        }
        AstExpression::Lambda(ast_lambda) => {
            if !ast_lambda.range.is_in_range(point) {
                return None;
            }
            match &ast_lambda.rhs {
                AstLambdaRhs::None => None,
                AstLambdaRhs::Block(ast_block) => get_class_block(ast_block, point),
                AstLambdaRhs::Expr(ast_expression) => get_class_expression(ast_expression, point),
            }
        }
        AstExpression::InlineSwitch(_ast_switch) => None,
        AstExpression::NewClass(ast_new_class) => {
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
    }
}

fn get_class_recursive_expression(
    expression: &AstRecursiveExpression,
    point: &AstPoint,
) -> Option<FoundClass> {
    if !expression.range.is_in_range(point) {
        return None;
    }
    let mut expression = expression;
    loop {
        if let Some(ident) = &expression.ident
            && let Some(i) = get_class_expression_identifier(ident, point)
        {
            return Some(i);
        }

        if let Some(vals) = &expression.values
            && vals.range.is_in_range(point)
        {
            for val in &vals.values {
                if let Some(s) = get_class_expression(val, point) {
                    return Some(s);
                }
            }
        }

        if let Some(next) = &expression.next {
            expression = next;
            if !expression.range.is_in_range(point) {
                return None;
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
        AstJTypeKind::Void => None,
        AstJTypeKind::Byte => None,
        AstJTypeKind::Char => None,
        AstJTypeKind::Double => None,
        AstJTypeKind::Float => None,
        AstJTypeKind::Int => None,
        AstJTypeKind::Long => None,
        AstJTypeKind::Short => None,
        AstJTypeKind::Boolean => None,
        AstJTypeKind::Wildcard => None,
        AstJTypeKind::Var => None,
        AstJTypeKind::Parameter(ast_identifier) | AstJTypeKind::Class(ast_identifier) => {
            if !ast_identifier.range.is_in_range(point) {
                return None;
            }
            Some(FoundClass {
                name: ast_identifier.value.to_string(),
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
    }
}

fn get_class_identifier(
    ast_identifier: &ast::types::AstIdentifier,
    point: &AstPoint,
) -> Option<FoundClass> {
    if ast_identifier.range.is_in_range(point) {
        return Some(FoundClass {
            name: ast_identifier.value.to_string(),
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
    }
}
