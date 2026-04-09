#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ast::{
    range::{GetRange, is_in_range_c},
    types::{
        AstBaseExpression, AstBlock, AstBlockEntry, AstBlockExpression, AstBlockVariable,
        AstClassMethod, AstExpression, AstExpressionKind, AstExpressionOrValue, AstFile, AstFor,
        AstForContent, AstForEnhanced, AstIf, AstIfContent, AstInterfaceConstant, AstJTypeKind,
        AstLambda, AstLambdaRhs, AstPoint, AstSwitch, AstSwitchCaseArrowContent,
        AstSwitchCaseArrowType, AstSwitchCaseArrowVar, AstThing, AstTryCatch, AstWhile,
        AstWhileContent,
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
    pub class_map: Arc<Mutex<HashMap<MyString, Class>>>,
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
    let level = 0;
    for th in &ast.things {
        get_vars_thing(th, level, context, &mut out)?;
    }

    // let n = cursor.goto_first_child_for_point(*point);
    Ok(out)
}

fn get_vars_thing(
    thing: &AstThing,
    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    match &thing {
        AstThing::Class(ast_class) => {
            let level = level + 1;
            out.extend(get_class_variables(&ast_class.block.variables, level));
            get_class_methods(&ast_class.block.methods, level, context, out)
        }
        AstThing::Record(ast_record) => {
            let level = level + 1;
            out.extend(get_class_variables(&ast_record.block.variables, level));
            get_class_methods(&ast_record.block.methods, level, context, out)
        }
        AstThing::Interface(ast_interface) => {
            let level = level + 1;
            out.extend(get_interface_constants(&ast_interface.constants, level));
            Ok(())
        }
        AstThing::Enumeration(_) | AstThing::Annotation(_) => Ok(()),
    }
}

fn get_interface_constants(
    constants: &[AstInterfaceConstant],
    level: usize,
) -> impl Iterator<Item = LocalVariable> {
    constants.iter().map(move |i| LocalVariable {
        level,
        jtype: (&i.jtype).into(),
        name: (&i.name).into(),
        range: i.range,
        flags: VarFlags::empty(),
    })
}

fn get_class_methods(
    methods: &[AstClassMethod],

    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    let level = level + 1;

    for method in methods {
        out.push(LocalVariable::from_class_method(method, level));
        if is_in_range_c(method.range, &context.point) {
            out.extend(
                method
                    .header
                    .parameters
                    .parameters
                    .iter()
                    .map(move |i| LocalVariable::from_method_parameter(i, level)),
            );
            if let Some(block) = &method.block {
                get_block_vars(block, level, context, out)?;
            }
        }
    }
    Ok(())
}

fn get_block_vars(
    block: &AstBlock,
    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    let level = level + 1;
    if !is_in_range_c(block.range, &context.point) {
        return Ok(());
    }
    for e in &block.entries {
        get_block_entry_vars(level, e, context, out)?;
    }
    Ok(())
}

fn get_block_entry_vars(
    level: usize,
    block_entry: &AstBlockEntry,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    match block_entry {
        AstBlockEntry::Break(_)
        | AstBlockEntry::Continue(_)
        | AstBlockEntry::SwitchCase(_)
        | AstBlockEntry::SwitchDefault(_)
        | AstBlockEntry::Yield(_)
        | AstBlockEntry::Assert(_)
        | AstBlockEntry::Assign(_) => Ok(()),
        AstBlockEntry::Variable(i) => {
            for v in i {
                from_block_variable(v, level, context, out)?;
            }
            Ok(())
        }
        AstBlockEntry::Throw(t) => expression(&t.expression, level, context, out),
        AstBlockEntry::Return(r) => expression_or_value(&r.expression, level, context, out),
        AstBlockEntry::Expression(ast_expression) => {
            block_expr(ast_expression, level, context, out)
        }
        AstBlockEntry::If(ast_if) => if_vars(ast_if, level, context, out),
        AstBlockEntry::While(ast_while) => while_vars(ast_while, level, context, out),
        AstBlockEntry::For(ast_for) => for_vars(ast_for, level, context, out),
        AstBlockEntry::ForEnhanced(ast_for_enhanced) => {
            for_enanced_vars(ast_for_enhanced, level, context, out)
        }
        AstBlockEntry::Switch(ast_switch) => switch_vars(ast_switch, level, context, out),
        AstBlockEntry::TryCatch(ast_try_catch) => {
            try_catch_vars(ast_try_catch, level, context, out)
        }
        AstBlockEntry::SynchronizedBlock(ast_synchronized_block) => {
            get_block_vars(&ast_synchronized_block.block, level, context, out)
        }
        AstBlockEntry::SwitchCaseArrowDefault(ast_switch_case_arrow_default) => {
            switch_case_arrow_content(&ast_switch_case_arrow_default.content, level, context, out)
        }
        AstBlockEntry::SwitchCaseArrowValues(ast_switch_case_arrow) => {
            switch_case_arrow_content(&ast_switch_case_arrow.content, level, context, out)
        }
        AstBlockEntry::SwitchCaseArrowType(AstSwitchCaseArrowType {
            var: AstSwitchCaseArrowVar { jtype, name, range },
            content,
            ..
        }) => {
            out.push(LocalVariable {
                level,
                jtype: jtype.into(),
                name: name.into(),
                range: *range,
                flags: VarFlags::empty(),
            });
            switch_case_arrow_content(content, level, context, out)
        }
        AstBlockEntry::Thing(ast_thing) => get_vars_thing(ast_thing, level, context, out),
        AstBlockEntry::InlineBlock(ast_block) => {
            get_block_vars(&ast_block.block, level, context, out)
        }
        AstBlockEntry::Semicolon(_ast_range) => Ok(()),
    }
}

fn switch_case_arrow_content(
    arrow_content: &AstSwitchCaseArrowContent,
    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    match arrow_content {
        AstSwitchCaseArrowContent::Block(ast_block) => {
            get_block_vars(ast_block, level, context, out)
        }
        AstSwitchCaseArrowContent::Entry(ast_block_entry) => {
            get_block_entry_vars(level, ast_block_entry, context, out)
        }
    }
}
fn expression_or_value(
    e: &AstExpressionOrValue,

    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    match e {
        AstExpressionOrValue::None | AstExpressionOrValue::Value(_) => Ok(()),
        AstExpressionOrValue::Expression(ast_expression_kinds) => {
            expression(ast_expression_kinds, level, context, out)
        }
    }
}

fn block_expr(
    ast_expression: &AstBlockExpression,

    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(ast_expression.range, &context.point) {
        return Ok(());
    }

    expression(&ast_expression.value, level, context, out)
}

fn base_expr(
    expr: &AstBaseExpression,
    level: usize,
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
            expression(v, level, context, out)?;
        }
    }
    Ok(())
}

fn expression(
    expression: &AstExpression,
    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    for kind in expression {
        // if let AstExpressionKind::Base(b) = kind
        //     && let Some(values) = &b.values
        //     && kind.get_range().is_in_range(&context.point)
        //     && let Some(AstExpressionKind::Base(prev)) = expression.get(idx - 1)
        //     && prev.ident.is_some()
        // {
        //     dbg!(expression, values);
        // }
        match kind {
            AstExpressionKind::Base(exp) => {
                base_expr(exp, level, context, out)?;
            }

            AstExpressionKind::Lambda(ast_lambda) => {
                if is_in_range_c(ast_lambda.range, &context.point) {
                    lambda(ast_lambda, level, context, out)?;
                }
            }
            AstExpressionKind::InlineSwitch(ast_switch) => {
                get_block_vars(&ast_switch.block, level, context, out)?;
            }
            AstExpressionKind::NewClass(_)
            | AstExpressionKind::InstanceOf(_)
            | AstExpressionKind::Generics(_)
            | AstExpressionKind::JType(_)
            | AstExpressionKind::Casted(_)
            | AstExpressionKind::Array(_) => (),
        }
    }
    Ok(())
}

fn lambda(
    lambda: &AstLambda,
    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    out.extend(lambda.parameters.values.iter().map(|i| LocalVariable {
        level,
        jtype: JType::Var,
        name: i.name.value.clone(),
        range: i.range,
        flags: VarFlags::empty(),
    }));

    match &lambda.rhs {
        AstLambdaRhs::None => Ok(()),
        AstLambdaRhs::Block(ast_block) => get_block_vars(ast_block, level, context, out),
        AstLambdaRhs::Expr(ast_base_expression) => {
            expression(ast_base_expression, level, context, out)
        }
    }
}

fn try_catch_vars(
    ast_try_catch: &AstTryCatch,
    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(ast_try_catch.range, &context.point) {
        return Ok(());
    }
    let level = level + 1;
    if let Some(resources) = &ast_try_catch.resources_block {
        get_block_vars(resources, level, context, out)?;
    }
    get_block_vars(&ast_try_catch.block, level, context, out)?;
    if let Some(case) = ast_try_catch
        .cases
        .iter()
        .find(|i| is_in_range_c(i.range, &context.point))
    {
        for ty in &case.variable.jtypes {
            out.push(LocalVariable {
                level,
                jtype: ty.into(),
                name: case.variable.name.value.clone(),
                range: case.variable.range,
                flags: VarFlags::empty(),
            });
        }
        get_block_vars(&case.block, level, context, out)?;
    }
    if let Some(finally_block) = &ast_try_catch.finally_block {
        get_block_vars(finally_block, level, context, out)?;
    }
    Ok(())
}

fn switch_vars(
    ast_for_enhanced: &AstSwitch,
    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(ast_for_enhanced.range, &context.point) {
        return Ok(());
    }
    let level = level + 1;
    get_block_vars(&ast_for_enhanced.block, level, context, out)
}

fn for_enanced_vars(
    ast_for_enhanced: &AstForEnhanced,
    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(ast_for_enhanced.range, &context.point) {
        return Ok(());
    }
    let level = level + 1;
    for v in &ast_for_enhanced.var {
        from_block_variable(v, level, context, out)?;
    }
    for_content_vars(&ast_for_enhanced.content, level, context, out)
}

pub fn from_block_variable(
    v: &AstBlockVariable,
    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if matches!(v.jtype.value, AstJTypeKind::Var)
        && let Some(value) = &v.value
    {
        let point = value.get_range().end;
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
                    level,
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
        level,
        jtype: (&v.jtype).into(),
        name: v.name.value.clone(),
        range: v.range,
        flags: VarFlags::empty(),
    });
    Ok(())
}
fn for_content_vars(
    fc: &AstForContent,
    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    match fc {
        AstForContent::Block(ast_block) => get_block_vars(ast_block, level, context, out),
        AstForContent::BlockEntry(ast_block_entry) => {
            get_block_entry_vars(level, ast_block_entry, context, out)
        }

        AstForContent::None => Ok(()),
    }
}

fn for_vars(
    ast_for: &AstFor,
    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(ast_for.range, &context.point) {
        return Ok(());
    }
    let level = level + 1;
    for v in &ast_for.vars {
        get_block_entry_vars(level, v, context, out)?;
    }
    for_content_vars(&ast_for.content, level, context, out)
}
fn while_vars(
    ast_while: &AstWhile,
    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    if !is_in_range_c(ast_while.range, &context.point) {
        return Ok(());
    }
    let level = level + 1;
    if let AstWhileContent::Block(b) = &ast_while.content {
        get_block_vars(b, level, context, out)?;
    }
    Ok(())
}
fn if_vars(
    ast_if: &AstIf,
    level: usize,
    context: &VariableContext,
    out: &mut Vec<LocalVariable>,
) -> Result<(), VariablesError> {
    let level = level + 1;
    match ast_if {
        AstIf::ElseIf {
            range,
            control: _,
            control_range: _,
            content,
        }
        | AstIf::If {
            range,
            control: _,
            control_range: _,
            content,
        }
        | AstIf::Else { range, content } => {
            if is_in_range_c(*range, &context.point)
                && let AstIfContent::Block(block) = content
            {
                get_block_vars(block, level, context, out)?;
            }
        }
    }
    Ok(())
}
fn get_class_variables(
    variables: &[ast::types::AstClassVariable],
    level: usize,
) -> impl Iterator<Item = LocalVariable> {
    variables.iter().map(move |i| {
        let jtype: JType = (&i.jtype).into();
        LocalVariable {
            range: i.range,
            level,
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
        sync::{Arc, Mutex},
    };

    use ast::{error::PrintErr, types::AstPoint};
    use dto::{Access, Class};
    use my_string::MyString;

    use crate::{VariableContext, get_vars};

    fn get_class_map() -> Arc<Mutex<HashMap<MyString, Class>>> {
        let mut class_map: HashMap<MyString, Class> = HashMap::new();
        class_map.insert(
            "java.lang.Integer".into(),
            Class {
                access: Access::Public,
                name: "Integer".into(),
                ..Default::default()
            },
        );
        // class_map.insert(
        //     "java.lang.Thing".into(),
        //     Class {
        //         access: Access::Public,
        //         name: "Thing".into(),
        //         methods: vec![Method {
        //             name: Some("dothing".into()),
        //             parameters: vec![Parameter {
        //                 name: None,
        //                 jtype: todo!(),
        //             }],
        //             ..Default::default()
        //         }],
        //         ..Default::default()
        //     },
        // );
        Arc::new(Mutex::new(class_map))
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
        insta::assert_debug_snapshot!(out);
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
                class_map: Default::default(),
            },
        )
        .unwrap();
        insta::assert_debug_snapshot!(out);
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
                class_map: Default::default(),
            },
        )
        .unwrap();
        insta::assert_debug_snapshot!(out);
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
                class_map: Default::default(),
            },
        )
        .unwrap();
        insta::assert_debug_snapshot!(out);
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
                class_map: Default::default(),
            },
        )
        .unwrap();
        insta::assert_debug_snapshot!(out);
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
                class_map: Default::default(),
            },
        )
        .unwrap();
        insta::assert_debug_snapshot!(out);
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
        insta::assert_debug_snapshot!(out);
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
        insta::assert_debug_snapshot!(out);
    }
}
