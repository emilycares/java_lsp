#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use ast::{
    range::{GetRange, is_in_range_c},
    types::{
        AstBaseExpression, AstBlock, AstBlockEntry, AstBlockExpression, AstBlockVariable,
        AstClassConstructor, AstClassMethod, AstExpression, AstExpressionKind,
        AstExpressionOrValue, AstFile, AstFor, AstForContent, AstForEnhanced, AstIf, AstIfContent,
        AstInterfaceConstant, AstInterfaceMethod, AstInterfaceMethodDefault, AstJTypeKind,
        AstLambda, AstLambdaRhs, AstNewRhs, AstPoint, AstSwitch, AstSwitchCaseArrowContent,
        AstSwitchCaseArrowType, AstSwitchCaseArrowVar, AstThing, AstTopLevel, AstTryCatch,
        AstWhile, AstWhileContent,
    },
};
use dto::{Class, ImportUnit, JType};
use local_variable::{LocalVariable, VarFlags};
use my_string::MyString;
use tyres::{ResolveState, TyresError};

pub struct VariableContext<'a> {
    pub point: Option<AstPoint>,
    pub imports: &'a [ImportUnit],
    pub class: &'a Class,
    pub class_map: Arc<RwLock<HashMap<MyString, Class>>>,
}

#[derive(Debug)]
pub enum VariablesError {
    Tyres(TyresError),
}

/// Get Local Variables and Functions of the current ast
pub fn get_vars(
    ast: &AstFile,
    context: &VariableContext,
) -> Result<Vec<LocalVariable>, VariablesError> {
    let mut out: Vec<LocalVariable> = vec![];
    for top in &ast.top {
        match top {
            AstTopLevel::Thing(ast_thing) => {
                get_vars_thing(ast_thing, context, &mut out)?;
            }
            AstTopLevel::Method(m) => {
                get_vars_method(m, context, &mut out)?;
            }
            AstTopLevel::Package(_) | AstTopLevel::Import(_) | AstTopLevel::Module(_) => (),
        }
    }

    // let n = cursor.goto_first_child_for_point(*point);
    Ok(out)
}

fn get_vars_thing(
    thing: &AstThing,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    match &thing {
        AstThing::Class(ast_class) => {
            out.extend(variables(&ast_class.block.variables));
            class_block(&ast_class.block, context, out)?;
            Ok(())
        }
        AstThing::Record(ast_record) => {
            out.extend(variables(&ast_record.block.variables));
            methods(&ast_record.block.methods, context, out)?;
            for b in &ast_record.block.static_blocks {
                get_block_vars(&b.block, context, out)?;
            }
            for b in &ast_record.block.blocks {
                get_block_vars(b, context, out)?;
            }
            for b in &ast_record.block.inner {
                get_vars_thing(b, context, out)?;
            }
            Ok(())
        }
        AstThing::Interface(ast_interface) => {
            out.extend(get_interface_constants(&ast_interface.constants));
            interface_methods(&ast_interface.methods, context, out);
            for m in &ast_interface.default_methods {
                interface_default_method(m, context, out)?;
            }
            for b in &ast_interface.inner {
                get_vars_thing(b, context, out)?;
            }
            Ok(())
        }
        AstThing::Enumeration(e) => {
            out.extend(variables(&e.variables));
            methods(&e.methods, context, out)?;
            constructors(&e.constructors, context, out)?;
            for b in &e.static_blocks {
                get_block_vars(&b.block, context, out)?;
            }
            for b in &e.inner {
                get_vars_thing(b, context, out)?;
            }
            Ok(())
        }
        AstThing::Annotation(_) => Ok(()),
    }
}

fn class_block(
    block: &ast::types::AstClassBlock,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    methods(&block.methods, context, out)?;
    constructors(&block.constructors, context, out)?;
    for b in &block.static_blocks {
        get_block_vars(&b.block, context, out)?;
    }
    for b in &block.blocks {
        get_block_vars(b, context, out)?;
    }
    Ok(())
}

fn get_interface_constants(
    constants: &[AstInterfaceConstant],
) -> impl Iterator<Item = LocalVariable> {
    constants.iter().map(move |i| LocalVariable {
        jtype: (&i.jtype).into(),
        name: (&i.name).into(),
        range: i.range,
        flags: VarFlags::empty(),
    })
}

fn methods(
    methods: &[AstClassMethod],

    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    for method in methods {
        get_vars_method(method, context, out)?;
    }
    Ok(())
}

fn interface_methods(
    methods: &[AstInterfaceMethod],

    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) {
    for method in methods {
        interface_method(method, context, out);
    }
}

fn constructors(
    methods: &[AstClassConstructor],

    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    for cons in methods {
        constructor(cons, context, out)?;
    }
    Ok(())
}

fn interface_method(
    method: &AstInterfaceMethod,
    context: &VariableContext<'_>,
    out: &mut Vec<LocalVariable>,
) {
    out.push(LocalVariable {
        jtype: (&method.header.jtype).into(),
        name: (&method.header.name).into(),
        range: method.range,
        flags: VarFlags::Function,
    });
    if is_in_range_c(method.range, &context.point) {
        out.extend(
            method
                .header
                .parameters
                .parameters
                .iter()
                .map(LocalVariable::from_method_parameter),
        );
    }
}

fn interface_default_method(
    method: &AstInterfaceMethodDefault,
    context: &VariableContext<'_>,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    out.push(LocalVariable {
        jtype: (&method.header.jtype).into(),
        name: (&method.header.name).into(),
        range: method.range,
        flags: VarFlags::Function,
    });
    if is_in_range_c(method.range, &context.point) {
        out.extend(
            method
                .header
                .parameters
                .parameters
                .iter()
                .map(LocalVariable::from_method_parameter),
        );
        get_block_vars(&method.block, context, out)?;
    }
    Ok(())
}

fn get_vars_method(
    method: &AstClassMethod,
    context: &VariableContext<'_>,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    out.push(LocalVariable {
        jtype: (&method.header.jtype).into(),
        name: (&method.header.name).into(),
        range: method.range,
        flags: VarFlags::Function,
    });
    if is_in_range_c(method.range, &context.point) {
        out.extend(
            method
                .header
                .parameters
                .parameters
                .iter()
                .map(LocalVariable::from_method_parameter),
        );
        if let Some(block) = &method.block {
            get_block_vars(block, context, out)?;
        }
    }
    Ok(())
}

fn constructor(
    cons: &AstClassConstructor,
    context: &VariableContext<'_>,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    out.push(LocalVariable {
        jtype: JType::Class(cons.header.name.clone().into()),
        name: (&cons.header.name).into(),
        range: cons.range,
        flags: VarFlags::Function,
    });
    if is_in_range_c(cons.range, &context.point) {
        out.extend(
            cons.header
                .parameters
                .parameters
                .iter()
                .map(LocalVariable::from_method_parameter),
        );
        get_block_vars(&cons.block, context, out)?;
    }
    Ok(())
}

fn get_block_vars(
    block: &AstBlock,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(block.range, &context.point) {
        return Ok(());
    }
    for e in &block.entries {
        get_block_entry_vars(e, context, out)?;
    }
    Ok(())
}

fn get_block_entry_vars(
    block_entry: &AstBlockEntry,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    match block_entry {
        AstBlockEntry::Semicolon(_)
        | AstBlockEntry::Break(_)
        | AstBlockEntry::Continue(_)
        | AstBlockEntry::SwitchCase(_)
        | AstBlockEntry::SwitchDefault(_)
        | AstBlockEntry::Yield(_)
        | AstBlockEntry::Assert(_)
        | AstBlockEntry::Assign(_) => Ok(()),
        AstBlockEntry::Variable(i) => {
            for v in i {
                from_block_variable(v, context, out)?;
            }
            Ok(())
        }
        AstBlockEntry::Throw(t) => expression(&t.expression, context, out),
        AstBlockEntry::Return(r) => expression_or_value(&r.expression, context, out),
        AstBlockEntry::Expression(ast_expression) => block_expr(ast_expression, context, out),
        AstBlockEntry::If(ast_if) => if_vars(ast_if, context, out),
        AstBlockEntry::While(ast_while) => while_vars(ast_while, context, out),
        AstBlockEntry::For(ast_for) => for_vars(ast_for, context, out),
        AstBlockEntry::ForEnhanced(ast_for_enhanced) => {
            for_enanced_vars(ast_for_enhanced, context, out)
        }
        AstBlockEntry::Switch(ast_switch) => switch_vars(ast_switch, context, out),
        AstBlockEntry::TryCatch(ast_try_catch) => try_catch_vars(ast_try_catch, context, out),
        AstBlockEntry::SynchronizedBlock(ast_synchronized_block) => {
            get_block_vars(&ast_synchronized_block.block, context, out)
        }
        AstBlockEntry::SwitchCaseArrowDefault(ast_switch_case_arrow_default) => {
            switch_case_arrow_content(&ast_switch_case_arrow_default.content, context, out)
        }
        AstBlockEntry::SwitchCaseArrowValues(ast_switch_case_arrow) => {
            switch_case_arrow_content(&ast_switch_case_arrow.content, context, out)
        }
        AstBlockEntry::SwitchCaseArrowType(AstSwitchCaseArrowType {
            var: AstSwitchCaseArrowVar { jtype, name, range },
            content,
            ..
        }) => {
            out.push(LocalVariable {
                jtype: jtype.into(),
                name: name.into(),
                range: *range,
                flags: VarFlags::empty(),
            });
            switch_case_arrow_content(content, context, out)
        }
        AstBlockEntry::Thing(ast_thing) => get_vars_thing(ast_thing, context, out),
        AstBlockEntry::InlineBlock(ast_block) => get_block_vars(&ast_block.block, context, out),
    }
}

fn switch_case_arrow_content(
    arrow_content: &AstSwitchCaseArrowContent,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    match arrow_content {
        AstSwitchCaseArrowContent::Block(ast_block) => get_block_vars(ast_block, context, out),
        AstSwitchCaseArrowContent::Entry(ast_block_entry) => {
            get_block_entry_vars(ast_block_entry, context, out)
        }
    }
}
fn expression_or_value(
    e: &AstExpressionOrValue,

    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    match e {
        AstExpressionOrValue::None | AstExpressionOrValue::Value(_) => Ok(()),
        AstExpressionOrValue::Expression(ast_expression_kinds) => {
            expression(ast_expression_kinds, context, out)
        }
    }
}

fn block_expr(
    ast_expression: &AstBlockExpression,

    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(ast_expression.range, &context.point) {
        return Ok(());
    }

    expression(&ast_expression.value, context, out)
}

fn base_expr(
    expr: &AstBaseExpression,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(expr.range, &context.point) {
        return Ok(());
    }
    if let Some(v) = &expr.values
        && !v.values.is_empty()
    {
        for v in &v.values {
            expression(v, context, out)?;
        }
    }
    Ok(())
}

fn expression(
    expression: &AstExpression,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    for kind in expression {
        match kind {
            AstExpressionKind::Base(exp) => {
                base_expr(exp, context, out)?;
            }
            AstExpressionKind::Lambda(ast_lambda) => {
                if is_in_range_c(ast_lambda.range, &context.point) {
                    lambda(ast_lambda, context, out)?;
                }
            }
            AstExpressionKind::InlineSwitch(ast_switch) => {
                get_block_vars(&ast_switch.block, context, out)?;
            }
            AstExpressionKind::InstanceOf(i) => {
                if let Some(name) = &i.variable {
                    out.push(LocalVariable {
                        jtype: i.jtype.clone().into(),
                        name: name.value.clone(),
                        range: name.range,
                        flags: VarFlags::empty(),
                    });
                }
            }
            AstExpressionKind::NewClass(nc) => new_class(nc, context, out)?,
            AstExpressionKind::Generics(_)
            | AstExpressionKind::JType(_)
            | AstExpressionKind::Array(_) => (),
        }
    }
    Ok(())
}

fn new_class(
    nc: &ast::types::AstNewClass,
    context: &VariableContext<'_>,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(nc.range, &context.point) {
        return Ok(());
    }
    match &*nc.rhs {
        AstNewRhs::Array(_)
        | AstNewRhs::None
        | AstNewRhs::ArrayParameters(_)
        | AstNewRhs::Parameters(_, _) => Ok(()),
        AstNewRhs::Block(ast_class_block)
        | AstNewRhs::ParametersAndBlock(_, _, ast_class_block) => {
            class_block(ast_class_block, context, out)
        }
    }
}

fn lambda(
    lambda: &AstLambda,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    out.extend(
        lambda
            .parameters
            .values
            .iter()
            .filter(|i| i.name.value != "_")
            .map(|i| LocalVariable {
                jtype: JType::Var,
                name: i.name.value.clone(),
                range: i.range,
                flags: VarFlags::empty(),
            }),
    );

    match &lambda.rhs {
        AstLambdaRhs::None => Ok(()),
        AstLambdaRhs::Block(ast_block) => get_block_vars(ast_block, context, out),
        AstLambdaRhs::Expr(ast_base_expression) => expression(ast_base_expression, context, out),
    }
}

fn try_catch_vars(
    ast_try_catch: &AstTryCatch,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(ast_try_catch.range, &context.point) {
        return Ok(());
    }
    if let Some(resources) = &ast_try_catch.resources_block {
        get_block_vars(resources, context, out)?;
    }
    get_block_vars(&ast_try_catch.block, context, out)?;
    if let Some(case) = ast_try_catch
        .cases
        .iter()
        .find(|i| is_in_range_c(i.range, &context.point))
    {
        for ty in &case.variable.jtypes {
            out.push(LocalVariable {
                jtype: ty.into(),
                name: case.variable.name.value.clone(),
                range: case.variable.range,
                flags: VarFlags::empty(),
            });
        }
        get_block_vars(&case.block, context, out)?;
    }
    if let Some(finally_block) = &ast_try_catch.finally_block {
        get_block_vars(finally_block, context, out)?;
    }
    Ok(())
}

fn switch_vars(
    ast_for_enhanced: &AstSwitch,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(ast_for_enhanced.range, &context.point) {
        return Ok(());
    }
    get_block_vars(&ast_for_enhanced.block, context, out)
}

fn for_enanced_vars(
    ast_for_enhanced: &AstForEnhanced,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(ast_for_enhanced.range, &context.point) {
        return Ok(());
    }
    for v in &ast_for_enhanced.var {
        if v.value.is_none() && matches!(v.jtype.value, AstJTypeKind::Var) {
            let point = ast_for_enhanced.rhs.get_range().end;
            let mut cc = vec![];
            call_chain::cc_expr(&ast_for_enhanced.rhs, &point, false, &mut cc);
            let value_resolve_state = tyres::resolve_call_chain_value(
                &cc,
                out,
                context.imports,
                context.class,
                &context.class_map.clone(),
            );
            match value_resolve_state {
                Ok(ResolveState {
                    jtype: JType::Array(i),
                    ..
                }) => {
                    out.push(LocalVariable {
                        jtype: *i,
                        name: v.name.value.clone(),
                        range: v.range,
                        flags: VarFlags::Computed,
                    });
                }
                Ok(ResolveState { jtype, .. }) => {
                    out.push(LocalVariable {
                        jtype,
                        name: v.name.value.clone(),
                        range: v.range,
                        flags: VarFlags::Computed,
                    });
                }
                Err(e) => return Err(VariablesError::Tyres(e)),
            }
            continue;
        }
        from_block_variable(v, context, out)?;
    }
    for_content_vars(&ast_for_enhanced.content, context, out)
}

pub fn from_block_variable(
    v: &AstBlockVariable,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if matches!(v.jtype.value, AstJTypeKind::Var)
        && let Some(value) = &v.value
    {
        let mut point = value.get_range().end;
        point.col += 1;
        let mut cc = vec![];
        call_chain::cc_expr(value, &point, false, &mut cc);
        let value_resolve_state = tyres::resolve_call_chain_value(
            &cc,
            out,
            context.imports,
            context.class,
            &context.class_map.clone(),
        );
        match value_resolve_state {
            Ok(ResolveState { jtype, .. }) => {
                out.push(LocalVariable {
                    jtype,
                    name: v.name.value.clone(),
                    range: v.range,
                    flags: VarFlags::Computed,
                });
                return Ok(());
            }
            Err(e) => return Err(VariablesError::Tyres(e)),
        }
    }
    out.push(LocalVariable {
        jtype: (&v.jtype).into(),
        name: v.name.value.clone(),
        range: v.range,
        flags: VarFlags::empty(),
    });
    Ok(())
}
fn for_content_vars(
    fc: &AstForContent,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    match fc {
        AstForContent::Block(ast_block) => get_block_vars(ast_block, context, out),
        AstForContent::BlockEntry(ast_block_entry) => {
            get_block_entry_vars(ast_block_entry, context, out)
        }

        AstForContent::None => Ok(()),
    }
}

fn for_vars(
    ast_for: &AstFor,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(ast_for.range, &context.point) {
        return Ok(());
    }
    for v in &ast_for.vars {
        get_block_entry_vars(v, context, out)?;
    }
    for_content_vars(&ast_for.content, context, out)
}
fn while_vars(
    ast_while: &AstWhile,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(ast_while.range, &context.point) {
        return Ok(());
    }
    if let AstWhileContent::Block(b) = &ast_while.content {
        get_block_vars(b, context, out)?;
    }
    Ok(())
}
fn if_vars(
    ast_if: &AstIf,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    match ast_if {
        AstIf::ElseIf {
            range,
            control,
            control_range: _,
            content,
        }
        | AstIf::If {
            range,
            control,
            control_range: _,
            content,
        } => {
            if is_in_range_c(content.get_range(), &context.point) {
                expression(control, context, out)?;
            }
            if is_in_range_c(*range, &context.point)
                && let AstIfContent::Block(block) = content
            {
                get_block_vars(block, context, out)?;
            }
        }
        AstIf::Else { range, content } => {
            if is_in_range_c(*range, &context.point)
                && let AstIfContent::Block(block) = content
            {
                get_block_vars(block, context, out)?;
            }
        }
    }
    Ok(())
}
fn variables(variables: &[ast::types::AstClassVariable]) -> impl Iterator<Item = LocalVariable> {
    variables.iter().map(move |i| {
        let jtype: JType = (&i.jtype).into();
        LocalVariable {
            range: i.range,
            jtype,
            name: i.name.value.clone(),
            flags: VarFlags::empty(),
        }
    })
}

#[cfg(test)]
pub mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock},
    };

    use ast::{error::PrintErr, types::AstPoint};
    use dto::{Access, Class};
    use expect_test::expect;
    use my_string::{MyString, smol_str::SmolStr};

    use crate::{VariableContext, get_vars};

    fn get_class_map() -> Arc<RwLock<HashMap<MyString, Class>>> {
        let mut class_map: HashMap<MyString, Class> = HashMap::new();
        class_map.insert(
            SmolStr::new_inline("java.lang.Integer"),
            Class {
                access: Access::Public,
                name: SmolStr::new_inline("Integer"),
                ..Default::default()
            },
        );
        // class_map.insert(
        //     SmolStr::new_inline("java.lang.Thing"),
        //     Class {
        //         access: Access::Public,
        //         name: SmolStr::new_inline("Thing"),
        //         methods: vec![Method {
        //             name: Some(SmolStr::new_inline("dothing")),
        //             parameters: vec![Parameter {
        //                 name: None,
        //                 jtype: todo!(),
        //             }],
        //             ..Default::default()
        //         }],
        //         ..Default::default()
        //     },
        // );
        Arc::new(RwLock::new(class_map))
    }

    #[test]
    fn this_context_base() {
        let content = "
package ch.emilycares;

public class Test {

    String hello;
    String se;

    private String other = \"\";

    public void hello(String a) {
        String local = \"\";

        var lo = 1;
        return;
    }
}
        ";
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();

        let class = Class::default();

        let out = get_vars(
            &ast,
            &VariableContext {
                point: Some(AstPoint::new(12, 17)),
                imports: Default::default(),
                class: &class,
                class_map: get_class_map(),
            },
        )
        .unwrap();
        let expected = expect![[r#"
            [
                LocalVariable {
                    jtype: Class(
                        "String",
                    ),
                    name: "hello",
                    range: AstRange {
                        start: AstPoint { 5:4 },
                        end: AstPoint { 5:16 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Class(
                        "String",
                    ),
                    name: "se",
                    range: AstRange {
                        start: AstPoint { 6:4 },
                        end: AstPoint { 6:13 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Class(
                        "String",
                    ),
                    name: "other",
                    range: AstRange {
                        start: AstPoint { 8:4 },
                        end: AstPoint { 8:28 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Void,
                    name: "hello",
                    range: AstRange {
                        start: AstPoint { 10:4 },
                        end: AstPoint { 15:5 },
                    },
                    flags: VarFlags(
                        Function,
                    ),
                },
                LocalVariable {
                    jtype: Class(
                        "String",
                    ),
                    name: "a",
                    range: AstRange {
                        start: AstPoint { 10:22 },
                        end: AstPoint { 10:30 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Class(
                        "String",
                    ),
                    name: "local",
                    range: AstRange {
                        start: AstPoint { 11:8 },
                        end: AstPoint { 11:24 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Class(
                        "java.lang.Integer",
                    ),
                    name: "lo",
                    range: AstRange {
                        start: AstPoint { 13:8 },
                        end: AstPoint { 13:19 },
                    },
                    flags: VarFlags(
                        Computed,
                    ),
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }

    #[test]
    fn class_static_variables() {
        let content = "
package ch.emilycares;
public class Test {
    private static Logger logger = LoggerFactory.getLogger(App.class);

}
        ";
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();
        let class = Class::default();

        let out = get_vars(
            &ast,
            &VariableContext {
                point: Some(AstPoint::new(4, 6)),
                imports: Default::default(),
                class: &class,
                class_map: Arc::default(),
            },
        )
        .unwrap();
        let expected = expect![[r#"
            [
                LocalVariable {
                    jtype: Class(
                        "Logger",
                    ),
                    name: "logger",
                    range: AstRange {
                        start: AstPoint { 3:4 },
                        end: AstPoint { 3:69 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }

    #[test]
    fn this_context_array() {
        let content = "
package ch.emilycares;

public class Test {

    String[] hello;
    String[] se;

    private String[] other = \"\";

    public void hello(String[] a) {
        String[] local = \"\";

        var lo =
        return;
    }
}
        ";
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens);
        ast.print_err(content, &tokens);
        let ast = ast.unwrap();

        let class = Class::default();
        let out = get_vars(
            &ast,
            &VariableContext {
                point: Some(AstPoint::new(12, 17)),
                imports: Default::default(),
                class: &class,
                class_map: Arc::default(),
            },
        )
        .unwrap();
        let expected = expect![[r#"
            [
                LocalVariable {
                    jtype: Array(
                        Class(
                            "String",
                        ),
                    ),
                    name: "hello",
                    range: AstRange {
                        start: AstPoint { 5:4 },
                        end: AstPoint { 5:18 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Array(
                        Class(
                            "String",
                        ),
                    ),
                    name: "se",
                    range: AstRange {
                        start: AstPoint { 6:4 },
                        end: AstPoint { 6:15 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Array(
                        Class(
                            "String",
                        ),
                    ),
                    name: "other",
                    range: AstRange {
                        start: AstPoint { 8:4 },
                        end: AstPoint { 8:30 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Void,
                    name: "hello",
                    range: AstRange {
                        start: AstPoint { 10:4 },
                        end: AstPoint { 15:5 },
                    },
                    flags: VarFlags(
                        Function,
                    ),
                },
                LocalVariable {
                    jtype: Array(
                        Class(
                            "String",
                        ),
                    ),
                    name: "a",
                    range: AstRange {
                        start: AstPoint { 10:22 },
                        end: AstPoint { 10:32 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Array(
                        Class(
                            "String",
                        ),
                    ),
                    name: "local",
                    range: AstRange {
                        start: AstPoint { 11:8 },
                        end: AstPoint { 11:26 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Var,
                    name: "lo",
                    range: AstRange {
                        start: AstPoint { 13:8 },
                        end: AstPoint { 13:17 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }

    #[test]
    fn get_loop_vars_base() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        List<String> names = List.of(\"a\", \"b\");
        for (int i = 0; i < 5; i++) {
          for (String name : names) {
            names.stream().map((n, m) -> {
              n.chars().asDoubleStream().filter(c -> true);
             return n + \"_\";
            });
          }
        }
        return;
    }
}
        ";
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();

        let class = Class::default();
        let out = get_vars(
            &ast,
            &VariableContext {
                point: Some(AstPoint::new(8, 54)),
                imports: Default::default(),
                class: &class,
                class_map: Arc::default(),
            },
        )
        .unwrap();
        let expected = expect![[r#"
            [
                LocalVariable {
                    jtype: Void,
                    name: "hello",
                    range: AstRange {
                        start: AstPoint { 3:4 },
                        end: AstPoint { 14:5 },
                    },
                    flags: VarFlags(
                        Function,
                    ),
                },
                LocalVariable {
                    jtype: Generic(
                        "List",
                        [
                            Class(
                                "String",
                            ),
                        ],
                    ),
                    name: "names",
                    range: AstRange {
                        start: AstPoint { 4:8 },
                        end: AstPoint { 4:46 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Int,
                    name: "i",
                    range: AstRange {
                        start: AstPoint { 5:13 },
                        end: AstPoint { 5:23 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Class(
                        "String",
                    ),
                    name: "name",
                    range: AstRange {
                        start: AstPoint { 6:15 },
                        end: AstPoint { 6:26 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Var,
                    name: "n",
                    range: AstRange {
                        start: AstPoint { 7:32 },
                        end: AstPoint { 7:33 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Var,
                    name: "m",
                    range: AstRange {
                        start: AstPoint { 7:35 },
                        end: AstPoint { 7:36 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Var,
                    name: "c",
                    range: AstRange {
                        start: AstPoint { 8:48 },
                        end: AstPoint { 8:49 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }

    #[test]
    fn get_try_vars_base() {
        let content = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        try (
            String fast1 = "1";
            String second1 = "2"
        ) {
            String ty1 = "a";
        } catch (IOException eio1) {
            String ca1 = "a";
        } finally {
            String fin = "a";
        }

        try {
            String some2 = "s";
        } catch (Exception e2) {
            String other2 = "o";
        }

        try {
            String some3 = "s";
        } catch (Exception | IOException e3) {
            String other3 = "o";
        } catch (IOException e3) {
            String other3 = "o";
        } finally {
            String fin3 = "a";
        }
        return;
    }
}
        "#;
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();

        let class = Class::default();
        let out = get_vars(
            &ast,
            &VariableContext {
                point: Some(AstPoint::new(8, 54)),
                imports: Default::default(),
                class: &class,
                class_map: Arc::default(),
            },
        )
        .unwrap();
        let expected = expect![[r#"
            [
                LocalVariable {
                    jtype: Void,
                    name: "hello",
                    range: AstRange {
                        start: AstPoint { 3:4 },
                        end: AstPoint { 31:5 },
                    },
                    flags: VarFlags(
                        Function,
                    ),
                },
                LocalVariable {
                    jtype: Class(
                        "String",
                    ),
                    name: "ty1",
                    range: AstRange {
                        start: AstPoint { 8:12 },
                        end: AstPoint { 8:28 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }
    #[test]
    fn get_catch_val_with_throws_method() {
        let content = "
package ch.emilycares;
public class Test {
    protected void ioStuff() throws IOException {
        try {
        } catch (IOException eoeoeoeooe) {
            printResponse(eoeoeoeooe);
        }
    }
}
        ";
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();

        let class = Class::default();
        let out = get_vars(
            &ast,
            &VariableContext {
                point: Some(AstPoint::new(6, 46)),
                imports: Default::default(),
                class: &class,
                class_map: Arc::default(),
            },
        )
        .unwrap();
        let expected = expect![[r#"
            [
                LocalVariable {
                    jtype: Void,
                    name: "ioStuff",
                    range: AstRange {
                        start: AstPoint { 3:4 },
                        end: AstPoint { 8:5 },
                    },
                    flags: VarFlags(
                        Function,
                    ),
                },
                LocalVariable {
                    jtype: Class(
                        "IOException",
                    ),
                    name: "eoeoeoeooe",
                    range: AstRange {
                        start: AstPoint { 5:17 },
                        end: AstPoint { 5:39 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }

    #[test]
    fn final_argument() {
        let content = r#"
package ch.emilycares;
public class Test {
    @Override
    public String options(final String outer) {
      String inner = "";
      return inner + outer;
    }
}
        "#;
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();

        let class = Class::default();
        let out = get_vars(
            &ast,
            &VariableContext {
                point: Some(AstPoint::new(5, 22)),
                imports: Default::default(),
                class: &class,
                class_map: get_class_map(),
            },
        )
        .unwrap();
        let expected = expect![[r#"
            [
                LocalVariable {
                    jtype: Class(
                        "String",
                    ),
                    name: "options",
                    range: AstRange {
                        start: AstPoint { 3:4 },
                        end: AstPoint { 7:5 },
                    },
                    flags: VarFlags(
                        Function,
                    ),
                },
                LocalVariable {
                    jtype: Class(
                        "String",
                    ),
                    name: "outer",
                    range: AstRange {
                        start: AstPoint { 4:26 },
                        end: AstPoint { 4:44 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Class(
                        "String",
                    ),
                    name: "inner",
                    range: AstRange {
                        start: AstPoint { 5:6 },
                        end: AstPoint { 5:22 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }

    #[test]
    fn in_lambda() {
        let content = "
public class Test {
    public Uni<Response> test() {
        return Thing.dothing(t -> {
                    Definition q = new Definition();

                    });
    }
}
";
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();
        let class = Class::default();
        let out = get_vars(
            &ast,
            &VariableContext {
                point: Some(AstPoint::new(4, 21)),
                imports: Default::default(),
                class: &class,
                class_map: get_class_map(),
            },
        )
        .unwrap();
        let expected = expect![[r#"
            [
                LocalVariable {
                    jtype: Generic(
                        "Uni",
                        [
                            Class(
                                "Response",
                            ),
                        ],
                    ),
                    name: "test",
                    range: AstRange {
                        start: AstPoint { 2:4 },
                        end: AstPoint { 7:5 },
                    },
                    flags: VarFlags(
                        Function,
                    ),
                },
                LocalVariable {
                    jtype: Var,
                    name: "t",
                    range: AstRange {
                        start: AstPoint { 3:29 },
                        end: AstPoint { 3:30 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
                LocalVariable {
                    jtype: Class(
                        "Definition",
                    ),
                    name: "q",
                    range: AstRange {
                        start: AstPoint { 4:20 },
                        end: AstPoint { 4:51 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }

    #[test]
    fn instanceof_base() {
        let content = "
public class Test {
    public void test() {
       if (shape instanceof Circle c) {

       }
    }
}
";
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();
        let class = Class::default();
        let out = get_vars(
            &ast,
            &VariableContext {
                point: Some(AstPoint::new(4, 40)),
                imports: Default::default(),
                class: &class,
                class_map: get_class_map(),
            },
        )
        .unwrap();
        let expected = expect![[r#"
            [
                LocalVariable {
                    jtype: Void,
                    name: "test",
                    range: AstRange {
                        start: AstPoint { 2:4 },
                        end: AstPoint { 6:5 },
                    },
                    flags: VarFlags(
                        Function,
                    ),
                },
                LocalVariable {
                    jtype: Class(
                        "Circle",
                    ),
                    name: "c",
                    range: AstRange {
                        start: AstPoint { 3:35 },
                        end: AstPoint { 3:36 },
                    },
                    flags: VarFlags(
                        0x0,
                    ),
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }
}
