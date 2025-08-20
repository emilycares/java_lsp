use ast::types::{
    AstBaseExpression, AstBlock, AstBlockEntry, AstBlockExpression, AstBlockVariable,
    AstClassMethod, AstFile, AstFor, AstForEnhanced, AstIf, AstIfContent, AstInterfaceConstant,
    AstLambda, AstLambdaRhs, AstMethodParamerter, AstPoint, AstRange, AstRecursiveExpression,
    AstSwitch, AstThing, AstTryCatch, AstWhile,
};
use parser::{dto, java::ParseJavaError};
use smol_str::SmolStr;

/// variable or function in a ast
#[derive(Debug, PartialEq, Clone)]
pub struct LocalVariable {
    pub level: usize,
    pub jtype: dto::JType,
    pub name: SmolStr,
    pub is_fun: bool,
    pub range: AstRange,
}

impl LocalVariable {
    pub fn from_block_variable(i: &AstBlockVariable, level: usize) -> Self {
        LocalVariable {
            level,
            jtype: (&i.jtype).into(),
            name: (&i.name).into(),
            is_fun: false,
            range: i.range,
        }
    }
    pub fn from_class_method(i: &AstClassMethod, level: usize) -> Self {
        LocalVariable {
            level,
            jtype: (&i.header.jtype).into(),
            name: (&i.header.name).into(),
            is_fun: true,
            range: i.range,
        }
    }

    fn from_method_parameter(parameter: &AstMethodParamerter, level: usize) -> LocalVariable {
        LocalVariable {
            level,
            jtype: (&parameter.jtype).into(),
            name: (&parameter.name).into(),
            is_fun: false,
            range: parameter.range,
        }
    }
}

#[derive(Debug)]
pub enum VariablesError {
    Parse(ParseJavaError),
}

/// Get Local Variables and Functions of the current ast
pub fn get_vars(ast: &AstFile, point: &AstPoint) -> Result<Vec<LocalVariable>, VariablesError> {
    let mut out: Vec<LocalVariable> = vec![];
    let level = 0;
    match &ast.thing {
        AstThing::Class(ast_class) => {
            let level = level + 1;
            out.extend(get_class_variables(&ast_class.block.variables, level));
            out.extend(get_class_methods(&ast_class.block.methods, point, level));
        }
        AstThing::Interface(ast_interface) => {
            let level = level + 1;
            out.extend(get_interface_constats(&ast_interface.constants, level));
        }
        AstThing::Enumeration(_) => (),
        AstThing::Annotation(_) => (),
    }

    // let n = cursor.goto_first_child_for_point(*point);
    Ok(out)
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
) -> Vec<LocalVariable> {
    let level = level + 1;
    let mut out = vec![];

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
            out.extend(get_block_vars(&method.block, point, level));
        }
    }
    out
}

fn get_block_vars(block: &AstBlock, point: &AstPoint, level: usize) -> Vec<LocalVariable> {
    let level = level + 1;
    if !block.range.is_in_range(point) {
        return vec![];
    }
    block
        .entries
        .iter()
        .flat_map(|i| match i {
            AstBlockEntry::Return(_)
            | AstBlockEntry::Break(_)
            | AstBlockEntry::Continue(_)
            | AstBlockEntry::Throw(_)
            | AstBlockEntry::SwitchCase(_)
            | AstBlockEntry::SwitchDefault(_)
            | AstBlockEntry::SwitchCaseArrow(_)
            | AstBlockEntry::Yield(_)
            | AstBlockEntry::Assign(_) => vec![],
            AstBlockEntry::Variable(i) => vec![LocalVariable::from_block_variable(i, level)],
            AstBlockEntry::Expression(ast_expression) => block_expr(ast_expression, point, level),
            AstBlockEntry::If(ast_if) => if_vars(ast_if, point, level),
            AstBlockEntry::While(ast_while) => while_vars(ast_while, point, level),
            AstBlockEntry::For(ast_for) => for_vars(ast_for, point, level),
            AstBlockEntry::ForEnhanced(ast_for_enhanced) => {
                for_enanced_vars(ast_for_enhanced, point, level)
            }
            AstBlockEntry::Switch(ast_switch) => switch_vars(ast_switch, point, level),
            AstBlockEntry::TryCatch(ast_try_catch) => try_catch_vars(ast_try_catch, point, level),
        })
        .collect()
}

fn block_expr(
    ast_expression: &AstBlockExpression,
    point: &AstPoint,
    level: usize,
) -> Vec<LocalVariable> {
    if !ast_expression.range.is_in_range(point) {
        return vec![];
    }

    return recursive_expr(&ast_expression.value, point, level);
}

fn recursive_expr(
    expr: &AstRecursiveExpression,
    point: &AstPoint,
    level: usize,
) -> Vec<LocalVariable> {
    if !expr.range.is_in_range(point) {
        return vec![];
    }
    let mut out = vec![];
    if let Some(v) = &expr.values {
        if !v.values.is_empty() {
            out.extend(
                v.values
                    .iter()
                    .flat_map(|i| base_expression(i, point, level)),
            );
        }
    }

    if let Some(next) = &expr.next {
        out.extend(recursive_expr(&next, point, level));
    }

    return out;
}

fn base_expression(i: &AstBaseExpression, point: &AstPoint, level: usize) -> Vec<LocalVariable> {
    match i {
        AstBaseExpression::Casted(c) => recursive_expr(&c.expression, point, level),
        AstBaseExpression::Recursive(ast_recursive_expression) => {
            return recursive_expr(ast_recursive_expression, point, level);
        }
        AstBaseExpression::Lambda(ast_lambda) => {
            if ast_lambda.range.is_in_range(point) {
                return lambda(ast_lambda, point, level);
            }
            return vec![];
        }
        AstBaseExpression::InlineSwitch(ast_switch) => {
            get_block_vars(&ast_switch.block, point, level)
        }
    }
}

fn lambda(lambda: &AstLambda, point: &AstPoint, level: usize) -> Vec<LocalVariable> {
    let mut out = vec![];

    out.extend(lambda.parameters.values.iter().map(|i| LocalVariable {
        level,
        jtype: dto::JType::Void,
        name: i.value.clone(),
        is_fun: false,
        range: i.range,
    }));

    match &lambda.rhs {
        AstLambdaRhs::None => (),
        AstLambdaRhs::Block(ast_block) => out.extend(get_block_vars(&ast_block, point, level)),
        AstLambdaRhs::Expr(ast_base_expression) => {
            out.extend(base_expression(ast_base_expression, point, level))
        }
    }

    out
}

fn try_catch_vars(
    ast_try_catch: &AstTryCatch,
    point: &AstPoint,
    level: usize,
) -> Vec<LocalVariable> {
    if !ast_try_catch.range.is_in_range(point) {
        return vec![];
    }
    let level = level + 1;
    let mut out = vec![];
    if let Some(resources) = &ast_try_catch.resources_block {
        out.extend(get_block_vars(resources, point, level));
    }
    out.extend(get_block_vars(&ast_try_catch.block, point, level));
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
        out.extend(get_block_vars(&case.block, point, level));
    }
    if let Some(finally_block) = &ast_try_catch.finally_block {
        out.extend(get_block_vars(finally_block, point, level));
    }
    out
}

fn switch_vars(ast_for_enhanced: &AstSwitch, point: &AstPoint, level: usize) -> Vec<LocalVariable> {
    if !ast_for_enhanced.range.is_in_range(point) {
        return vec![];
    }
    let level = level + 1;
    let mut out = vec![];
    out.extend(get_block_vars(&ast_for_enhanced.block, point, level));
    out
}

fn for_enanced_vars(
    ast_for_enhanced: &AstForEnhanced,
    point: &AstPoint,
    level: usize,
) -> Vec<LocalVariable> {
    if !ast_for_enhanced.range.is_in_range(point) {
        return vec![];
    }
    let level = level + 1;
    let mut out = vec![];
    out.push(LocalVariable::from_block_variable(
        &ast_for_enhanced.var,
        level,
    ));
    out.extend(get_block_vars(&ast_for_enhanced.block, point, level));
    out
}

fn for_vars(ast_for: &AstFor, point: &AstPoint, level: usize) -> Vec<LocalVariable> {
    if !ast_for.range.is_in_range(point) {
        return vec![];
    }
    let level = level + 1;
    let mut out = vec![];
    out.push(LocalVariable::from_block_variable(&ast_for.var, level));
    out.extend(get_block_vars(&ast_for.block, point, level));
    out
}
fn while_vars(ast_while: &AstWhile, point: &AstPoint, level: usize) -> Vec<LocalVariable> {
    if !ast_while.range.is_in_range(point) {
        return vec![];
    }
    let level = level + 1;
    get_block_vars(&ast_while.block, point, level)
}
fn if_vars(ast_if: &AstIf, point: &AstPoint, level: usize) -> Vec<LocalVariable> {
    let level = level + 1;
    match ast_if {
        AstIf::If {
            range,
            control: _,
            control_range: _,
            content,
            el,
        } => {
            if range.is_in_range(point)
                && let AstIfContent::Block(block) = content
            {
                return get_block_vars(block, point, level);
            }
            if let Some(el) = el.as_ref() {
                return if_vars(el, point, level);
            }
        }
        AstIf::Else { range, content } => {
            if range.is_in_range(point)
                && let AstIfContent::Block(block) = content
            {
                return get_block_vars(block, point, level);
            }
        }
    }
    vec![]
}
fn get_class_variables(
    variables: &[ast::types::AstClassVariable],
    level: usize,
) -> impl Iterator<Item = LocalVariable> {
    variables.iter().map(move |i| LocalVariable {
        range: i.range,
        level,
        jtype: (&i.jtype).into(),
        name: (&i.name).into(),
        is_fun: false,
    })
}

#[cfg(test)]
pub mod tests {
    use ast::{
        error::PrintErr,
        types::{AstPoint, AstRange},
    };
    use parser::dto;
    use pretty_assertions::assert_eq;

    use crate::{LocalVariable, get_vars};

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
        ast.print_err(content);
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
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Void,
                    name: "hello".into(),
                    is_fun: true,
                    range: AstRange {
                        start: AstPoint { line: 3, col: 16 },
                        end: AstPoint { line: 3, col: 21 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("String".into()),
                    name: "fast1".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 5, col: 19 },
                        end: AstPoint { line: 5, col: 24 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("String".into()),
                    name: "second1".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 6, col: 19 },
                        end: AstPoint { line: 6, col: 26 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("String".into()),
                    name: "ty1".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 8, col: 19 },
                        end: AstPoint { line: 8, col: 22 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("IOException".into()),
                    name: "eio1".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 9, col: 29 },
                        end: AstPoint { line: 9, col: 33 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Class("String".into()),
                    name: "ca1".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 10, col: 19 },
                        end: AstPoint { line: 10, col: 22 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Class("String".into()),
                    name: "fin".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 12, col: 19 },
                        end: AstPoint { line: 12, col: 22 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("String".into()),
                    name: "some2".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 16, col: 19 },
                        end: AstPoint { line: 16, col: 24 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("Exception".into()),
                    name: "e2".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 17, col: 27 },
                        end: AstPoint { line: 17, col: 29 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Class("String".into()),
                    name: "other2".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 18, col: 19 },
                        end: AstPoint { line: 18, col: 25 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("String".into()),
                    name: "some3".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 22, col: 19 },
                        end: AstPoint { line: 22, col: 24 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("Exception".into()),
                    name: "e3".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 23, col: 41 },
                        end: AstPoint { line: 23, col: 43 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Class("String".into()),
                    name: "other3".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 24, col: 19 },
                        end: AstPoint { line: 24, col: 25 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("IOException".into()),
                    name: "e3".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 25, col: 29 },
                        end: AstPoint { line: 25, col: 31 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Class("String".into()),
                    name: "other3".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 26, col: 19 },
                        end: AstPoint { line: 26, col: 25 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Class("String".into()),
                    name: "fin3".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 28, col: 19 },
                        end: AstPoint { line: 28, col: 23 },
                    },
                },
            ]
        );
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
