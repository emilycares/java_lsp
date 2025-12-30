#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use ast::types::{
    AstBlock, AstBlockEntry, AstBlockExpression, AstBlockVariable, AstClassMethod, AstExpression,
    AstExpressionKind, AstFile, AstFor, AstForContent, AstForEnhanced, AstIf, AstIfContent,
    AstInterfaceConstant, AstLambda, AstLambdaRhs, AstMethodParameter, AstPoint, AstRange,
    AstRecursiveExpression, AstSwitch, AstSwitchCaseArrowContent, AstThing, AstTryCatch, AstWhile,
    AstWhileContent,
};
use my_string::MyString;
use parser::dto::JType;

/// variable or function in a ast
#[derive(Debug, PartialEq, Clone)]
pub struct LocalVariable {
    pub level: usize,
    pub jtype: JType,
    pub name: MyString,
    pub is_fun: bool,
    pub range: AstRange,
}

impl LocalVariable {
    #[must_use]
    pub fn from_block_variable(v: &AstBlockVariable, level: usize) -> Self {
        let jtype: JType = (&v.jtype).into();
        Self {
            level,
            jtype,
            name: v.name.value.clone(),
            is_fun: false,
            range: v.range,
        }
    }
    #[must_use]
    pub fn from_class_method(i: &AstClassMethod, level: usize) -> Self {
        Self {
            level,
            jtype: (&i.header.jtype).into(),
            name: (&i.header.name).into(),
            is_fun: true,
            range: i.range,
        }
    }

    fn from_method_parameter(parameter: &AstMethodParameter, level: usize) -> Self {
        Self {
            level,
            jtype: (&parameter.jtype).into(),
            name: (&parameter.name).into(),
            is_fun: false,
            range: parameter.range,
        }
    }
}

#[derive(Debug)]
pub enum VariablesError {}

/// Get Local Variables and Functions of the current ast
pub fn get_vars(ast: &AstFile, point: &AstPoint) -> Result<Vec<LocalVariable>, VariablesError> {
    let mut out: Vec<LocalVariable> = vec![];
    let level = 0;
    for th in &ast.things {
        get_vars_thing(th, point, &mut out, level);
    }

    // let n = cursor.goto_first_child_for_point(*point);
    Ok(out)
}

fn get_vars_thing(thing: &AstThing, point: &AstPoint, out: &mut Vec<LocalVariable>, level: usize) {
    match &thing {
        AstThing::Class(ast_class) => {
            let level = level + 1;
            out.extend(get_class_variables(&ast_class.block.variables, level));
            get_class_methods(&ast_class.block.methods, point, level, out);
        }
        AstThing::Record(ast_record) => {
            let level = level + 1;
            out.extend(get_class_variables(&ast_record.block.variables, level));
            get_class_methods(&ast_record.block.methods, point, level, out);
        }
        AstThing::Interface(ast_interface) => {
            let level = level + 1;
            out.extend(get_interface_constats(&ast_interface.constants, level));
        }
        AstThing::Enumeration(_) | AstThing::Annotation(_) => (),
    }
}

fn get_interface_constats(
    contants: &[AstInterfaceConstant],
    level: usize,
) -> impl Iterator<Item = LocalVariable> {
    contants.iter().map(move |i| LocalVariable {
        level,
        jtype: (&i.jtype).into(),
        name: (&i.name).into(),
        is_fun: false,
        range: i.range,
    })
}

fn get_class_methods(
    methods: &[AstClassMethod],
    point: &AstPoint,
    level: usize,
    out: &mut Vec<LocalVariable>,
) {
    let level = level + 1;

    for method in methods {
        out.push(LocalVariable::from_class_method(method, level));
        if method.range.is_in_range(point) {
            out.extend(
                method
                    .header
                    .parameters
                    .parameters
                    .iter()
                    .map(move |i| LocalVariable::from_method_parameter(i, level)),
            );
            if let Some(block) = &method.block {
                get_block_vars(block, point, level, out);
            }
        }
    }
}

fn get_block_vars(block: &AstBlock, point: &AstPoint, level: usize, out: &mut Vec<LocalVariable>) {
    let level = level + 1;
    if !block.range.is_in_range(point) {
        return;
    }
    block
        .entries
        .iter()
        .for_each(|i| get_block_entry_vars(point, level, i, out));
}

fn get_block_entry_vars(
    point: &AstPoint,
    level: usize,
    block_entry: &AstBlockEntry,
    out: &mut Vec<LocalVariable>,
) {
    match block_entry {
        AstBlockEntry::Return(_)
        | AstBlockEntry::Break(_)
        | AstBlockEntry::Continue(_)
        | AstBlockEntry::Throw(_)
        | AstBlockEntry::SwitchCase(_)
        | AstBlockEntry::SwitchDefault(_)
        | AstBlockEntry::Yield(_)
        | AstBlockEntry::Assert(_)
        | AstBlockEntry::Assign(_) => (),
        AstBlockEntry::Variable(i) => {
            out.extend(
                i.iter()
                    .map(|i| LocalVariable::from_block_variable(i, level)),
            );
        }
        AstBlockEntry::Expression(ast_expression) => block_expr(ast_expression, point, level, out),
        AstBlockEntry::If(ast_if) => if_vars(ast_if, point, level, out),
        AstBlockEntry::While(ast_while) => while_vars(ast_while, point, level, out),
        AstBlockEntry::For(ast_for) => for_vars(ast_for, point, level, out),
        AstBlockEntry::ForEnhanced(ast_for_enhanced) => {
            for_enanced_vars(ast_for_enhanced, point, level, out);
        }
        AstBlockEntry::Switch(ast_switch) => switch_vars(ast_switch, point, level, out),
        AstBlockEntry::TryCatch(ast_try_catch) => try_catch_vars(ast_try_catch, point, level, out),
        AstBlockEntry::SynchronizedBlock(ast_synchronized_block) => {
            get_block_vars(&ast_synchronized_block.block, point, level, out);
        }
        AstBlockEntry::SwitchCaseArrowDefault(ast_switch_case_arrow_default) => {
            switch_case_arrow_content(&ast_switch_case_arrow_default.content, level, point, out);
        }
        AstBlockEntry::SwitchCaseArrowValues(ast_switch_case_arrow) => {
            switch_case_arrow_content(&ast_switch_case_arrow.content, level, point, out);
        }
        AstBlockEntry::Thing(ast_thing) => get_vars_thing(ast_thing, point, out, level),
        AstBlockEntry::InlineBlock(ast_block) => {
            get_block_vars(&ast_block.block, point, level, out);
        }
        AstBlockEntry::Semicolon(_ast_range) => (),
        AstBlockEntry::SwitchCaseArrowType(ast_switch_case_arrow_type) => {
            switch_case_arrow_content(&ast_switch_case_arrow_type.content, level, point, out);
        }
    }
}

fn switch_case_arrow_content(
    content: &AstSwitchCaseArrowContent,
    level: usize,
    point: &AstPoint,
    out: &mut Vec<LocalVariable>,
) {
    match content {
        AstSwitchCaseArrowContent::Block(ast_block) => get_block_vars(ast_block, point, level, out),
        AstSwitchCaseArrowContent::Entry(ast_block_entry) => {
            get_block_entry_vars(point, level, ast_block_entry, out);
        }
    }
}

fn block_expr(
    ast_expression: &AstBlockExpression,
    point: &AstPoint,
    level: usize,
    out: &mut Vec<LocalVariable>,
) {
    if !ast_expression.range.is_in_range(point) {
        return;
    }

    expression(&ast_expression.value, point, level, out);
}

fn recursive_expr(
    expr: &AstRecursiveExpression,
    point: &AstPoint,
    level: usize,
    out: &mut Vec<LocalVariable>,
) {
    if !expr.range.is_in_range(point) {
        return;
    }
    if let Some(v) = &expr.values
        && !v.values.is_empty()
    {
        v.values
            .iter()
            .for_each(|i| expression(i, point, level, out));
    }
}

fn expression_kind(
    i: &AstExpressionKind,
    point: &AstPoint,
    level: usize,
    out: &mut Vec<LocalVariable>,
) {
    match i {
        AstExpressionKind::Recursive(ast_recursive_expression) => {
            recursive_expr(ast_recursive_expression, point, level, out);
        }
        AstExpressionKind::Lambda(ast_lambda) => {
            if ast_lambda.range.is_in_range(point) {
                lambda(ast_lambda, point, level, out);
            }
        }
        AstExpressionKind::InlineSwitch(ast_switch) => {
            get_block_vars(&ast_switch.block, point, level, out);
        }
        AstExpressionKind::NewClass(_)
        | AstExpressionKind::Generics(_)
        | AstExpressionKind::InstanceOf(_)
        | AstExpressionKind::JType(_)
        | AstExpressionKind::Casted(_)
        | AstExpressionKind::Array(_) => (),
    }
}

fn expression(
    expression: &AstExpression,
    point: &AstPoint,
    level: usize,
    out: &mut Vec<LocalVariable>,
) {
    for e in expression {
        expression_kind(e, point, level, out);
    }
}

fn lambda(lambda: &AstLambda, point: &AstPoint, level: usize, out: &mut Vec<LocalVariable>) {
    out.extend(lambda.parameters.values.iter().map(|i| LocalVariable {
        level,
        jtype: JType::Void,
        name: i.name.value.clone(),
        is_fun: false,
        range: i.range,
    }));

    match &lambda.rhs {
        AstLambdaRhs::None => (),
        AstLambdaRhs::Block(ast_block) => get_block_vars(ast_block, point, level, out),
        AstLambdaRhs::Expr(ast_base_expression) => {
            expression(ast_base_expression, point, level, out);
        }
    }
}

fn try_catch_vars(
    ast_try_catch: &AstTryCatch,
    point: &AstPoint,
    level: usize,
    out: &mut Vec<LocalVariable>,
) {
    if !ast_try_catch.range.is_in_range(point) {
        return;
    }
    let level = level + 1;
    if let Some(resources) = &ast_try_catch.resources_block {
        get_block_vars(resources, point, level, out);
    }
    get_block_vars(&ast_try_catch.block, point, level, out);
    if let Some(case) = ast_try_catch
        .cases
        .iter()
        .find(|i| i.block.range.is_in_range(point))
    {
        for ty in &case.variable.jtypes {
            out.push(LocalVariable {
                level,
                jtype: ty.into(),
                name: case.variable.name.value.clone(),
                is_fun: false,
                range: case.variable.range,
            });
        }
        get_block_vars(&case.block, point, level, out);
    }
    if let Some(finally_block) = &ast_try_catch.finally_block {
        get_block_vars(finally_block, point, level, out);
    }
}

fn switch_vars(
    ast_for_enhanced: &AstSwitch,
    point: &AstPoint,
    level: usize,
    out: &mut Vec<LocalVariable>,
) {
    if !ast_for_enhanced.range.is_in_range(point) {
        return;
    }
    let level = level + 1;
    get_block_vars(&ast_for_enhanced.block, point, level, out);
}

fn for_enanced_vars(
    ast_for_enhanced: &AstForEnhanced,
    point: &AstPoint,
    level: usize,
    out: &mut Vec<LocalVariable>,
) {
    if !ast_for_enhanced.range.is_in_range(point) {
        return;
    }
    let level = level + 1;
    out.extend(
        ast_for_enhanced
            .var
            .iter()
            .map(|i| LocalVariable::from_block_variable(i, level)),
    );
    for_content_vars(&ast_for_enhanced.content, point, level, out);
}

fn for_content_vars(
    content: &AstForContent,
    point: &AstPoint,
    level: usize,
    out: &mut Vec<LocalVariable>,
) {
    match content {
        AstForContent::Block(ast_block) => get_block_vars(ast_block, point, level, out),
        AstForContent::BlockEntry(ast_block_entry) => {
            get_block_entry_vars(point, level, ast_block_entry, out);
        }
        AstForContent::None => (),
    }
}

fn for_vars(ast_for: &AstFor, point: &AstPoint, level: usize, out: &mut Vec<LocalVariable>) {
    if !ast_for.range.is_in_range(point) {
        return;
    }
    let level = level + 1;
    for v in &ast_for.vars {
        get_block_entry_vars(point, level, v, out);
    }
    for_content_vars(&ast_for.content, point, level, out);
}
fn while_vars(ast_while: &AstWhile, point: &AstPoint, level: usize, out: &mut Vec<LocalVariable>) {
    if !ast_while.range.is_in_range(point) {
        return;
    }
    let level = level + 1;
    if let AstWhileContent::Block(b) = &ast_while.content {
        get_block_vars(b, point, level, out);
    }
}
fn if_vars(ast_if: &AstIf, point: &AstPoint, level: usize, out: &mut Vec<LocalVariable>) {
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
            if range.is_in_range(point)
                && let AstIfContent::Block(block) = content
            {
                get_block_vars(block, point, level, out);
            }
        }
    }
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
            is_fun: false,
        }
    })
}

#[cfg(test)]
pub mod tests {
    use ast::{error::PrintErr, types::AstPoint};

    use crate::get_vars;

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

        var lo = 
        return;
    }
}
        ";
        let tokens = ast::lexer::lex(content).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();

        let out = get_vars(&ast, &AstPoint::new(12, 17)).unwrap();
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
        let tokens = ast::lexer::lex(content).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();

        let out = get_vars(&ast, &AstPoint::new(4, 6)).unwrap();
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
        let tokens = ast::lexer::lex(content).unwrap();
        let ast = ast::parse_file(&tokens);
        ast.print_err(content, &tokens);
        let ast = ast.unwrap();

        let out = get_vars(&ast, &AstPoint::new(12, 17)).unwrap();
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
        let tokens = ast::lexer::lex(content).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();

        let out = get_vars(&ast, &AstPoint::new(8, 54)).unwrap();
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
        let tokens = ast::lexer::lex(content).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();

        let out = get_vars(&ast, &AstPoint::new(8, 54)).unwrap();
        insta::assert_debug_snapshot!(out);
    }
    #[test]
    fn get_catch_val_with_throws_method() {
        let content = r#"
package ch.emilycares;
public class Test {
    protected void ioStuff() throws IOException {
        try {
        } catch (IOException eoeoeoeooe) {
            printResponse(eoeoeoeooe);
        }
    }
}
        "#;
        let tokens = ast::lexer::lex(content).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();

        let out = get_vars(&ast, &AstPoint::new(6, 46)).unwrap();
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
        let tokens = ast::lexer::lex(content).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();

        let out = get_vars(&ast, &AstPoint::new(5, 22)).unwrap();
        insta::assert_debug_snapshot!(out);
    }
}
