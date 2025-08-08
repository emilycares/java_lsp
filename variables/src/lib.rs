use ast::types::{
    AstBlock, AstBlockEntry, AstBlockVariable, AstClassMethod, AstFor, AstForEnhanced, AstIf,
    AstIfContent, AstInterfaceConstant, AstMethodParamerter, AstPoint, AstRange, AstSwitch,
    AstThing, AstTryCatch, AstWhile,
};
use document::Document;
use parser::{dto, java::ParseJavaError};
use smol_str::SmolStr;

/// document::Documentut a variable or function in a Document
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

/// Get Local Variables and Functions of the current Document
pub fn get_vars(
    document: &Document,
    point: &AstPoint,
) -> Result<Vec<LocalVariable>, VariablesError> {
    let mut out: Vec<LocalVariable> = vec![];
    let level = 0;
    match &document.ast.thing {
        AstThing::Class(ast_class) => {
            let level = level + 1;
            out.extend(get_class_variables(&ast_class.variables, level));
            out.extend(get_class_methods(&ast_class.methods, point, level));
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
            | AstBlockEntry::Expression(_)
            | AstBlockEntry::Break(_)
            | AstBlockEntry::Continue(_)
            | AstBlockEntry::Throw(_)
            | AstBlockEntry::SwitchCase(_)
            | AstBlockEntry::SwitchDefault(_)
            | AstBlockEntry::Assign(_) => vec![],
            AstBlockEntry::Variable(i) => vec![LocalVariable::from_block_variable(i, level)],
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
                && let AstIfContent::Block(block) = content {
                    return get_block_vars(block, point, level);
                }
            if let Some(el) = el.as_ref() {
                return if_vars(el, point, level);
            }
        }
        AstIf::Else { range, content } => {
            if range.is_in_range(point)
                && let AstIfContent::Block(block) = content {
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
    use std::path::PathBuf;

    use ast::types::{AstPoint, AstRange};
    use document::Document;
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
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();

        let out = get_vars(&doc, &AstPoint::new(12, 17)).unwrap();
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 1,
                    jtype: dto::JType::Class("String".into()),
                    name: "hello".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 5, col: 4 },
                        end: AstPoint { line: 5, col: 17 },
                    },
                },
                LocalVariable {
                    level: 1,
                    jtype: dto::JType::Class("String".into()),
                    name: "se".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 6, col: 4 },
                        end: AstPoint { line: 6, col: 14 },
                    },
                },
                LocalVariable {
                    level: 1,
                    jtype: dto::JType::Class("String".into()),
                    name: "other".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 8, col: 4 },
                        end: AstPoint { line: 8, col: 24 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Void,
                    name: "hello".into(),
                    is_fun: true,
                    range: AstRange {
                        start: AstPoint { line: 10, col: 16 },
                        end: AstPoint { line: 10, col: 21 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("String".into()),
                    name: "a".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 10, col: 29 },
                        end: AstPoint { line: 10, col: 30 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("String".into()),
                    name: "local".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 11, col: 15 },
                        end: AstPoint { line: 11, col: 20 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("var".into()),
                    name: "lo".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 13, col: 12 },
                        end: AstPoint { line: 13, col: 14 },
                    },
                },
            ]
        );
    }

    #[test]
    fn class_static_variables() {
        let content = "
package ch.emilycares;
public class Test {
    private static Logger logger = LoggerFactory.getLogger(App.class);
     
}
        ";
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();

        let out = get_vars(&doc, &AstPoint::new(4, 6)).unwrap();
        assert_eq!(
            out,
            vec![LocalVariable {
                level: 1,
                jtype: dto::JType::Class("Logger".into()),
                name: "logger".into(),
                is_fun: false,
                range: AstRange {
                    start: AstPoint { line: 3, col: 4 },
                    end: AstPoint { line: 3, col: 70 },
                },
            },]
        );
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
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();
        dbg!(&doc.ast);

        let out = get_vars(&doc, &AstPoint::new(12, 17)).unwrap();
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 1,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".into()))),
                    name: "hello".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 5, col: 4 },
                        end: AstPoint { line: 5, col: 19 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".into()))),
                    name: "se".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 6, col: 13 },
                        end: AstPoint { line: 6, col: 15 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".into()))),
                    name: "other".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 8, col: 21 },
                        end: AstPoint { line: 8, col: 26 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Void,
                    name: "hello".into(),
                    is_fun: true,
                    range: AstRange {
                        start: AstPoint { line: 10, col: 16 },
                        end: AstPoint { line: 10, col: 21 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".into()))),
                    name: "a".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 10, col: 31 },
                        end: AstPoint { line: 10, col: 32 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".into()))),
                    name: "local".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 11, col: 17 },
                        end: AstPoint { line: 11, col: 22 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("var".into()),
                    name: "lo".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 13, col: 12 },
                        end: AstPoint { line: 13, col: 14 },
                    },
                },
            ]
        );
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
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();

        let out = get_vars(&doc, &AstPoint::new(8, 54)).unwrap();
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
                    level: 3,
                    jtype: dto::JType::Generic(
                        "List".into(),
                        vec![dto::JType::Class("String".into())]
                    ),
                    name: "names".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 4, col: 21 },
                        end: AstPoint { line: 4, col: 26 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Int,
                    name: "i".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 5, col: 17 },
                        end: AstPoint { line: 5, col: 18 },
                    },
                },
                LocalVariable {
                    level: 7,
                    jtype: dto::JType::Class("String".into(),),
                    name: "name".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 6, col: 22 },
                        end: AstPoint { line: 6, col: 26 },
                    },
                },
                LocalVariable {
                    level: 12,
                    jtype: dto::JType::Void,
                    name: "n".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 7, col: 32 },
                        end: AstPoint { line: 7, col: 33 },
                    },
                },
                LocalVariable {
                    level: 12,
                    jtype: dto::JType::Void,
                    name: "m".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 7, col: 35 },
                        end: AstPoint { line: 7, col: 36 },
                    },
                },
                LocalVariable {
                    level: 17,
                    jtype: dto::JType::Void,
                    name: "c".into(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 8, col: 48 },
                        end: AstPoint { line: 8, col: 49 },
                    },
                },
            ]
        );
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
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();

        let out = get_vars(&doc, &AstPoint::new(8, 54)).unwrap();
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
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();

        let out = get_vars(&doc, &AstPoint::new(6, 46)).unwrap();
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
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();

        let out = get_vars(&doc, &AstPoint::new(5, 22)).unwrap();
        insta::assert_debug_snapshot!(out);
    }
}
