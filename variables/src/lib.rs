use ast::types::{
    AstBlockEntry, AstClassMethod, AstInterfaceConstant, AstPoint, AstRange, AstThing,
};
use document::Document;
use itertools::interleave;
use parser::{dto, java::ParseJavaError};

/// document::Documentut a variable or function in a Document
#[derive(Debug, PartialEq, Clone)]
pub struct LocalVariable {
    pub level: usize,
    pub jtype: dto::JType,
    pub name: String,
    pub is_fun: bool,
    pub range: AstRange,
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
    match &document.ast.thing {
        AstThing::Class(ast_class) => {
            out.extend(get_class_variables(&ast_class.variables));
            out.extend(get_class_methods(&ast_class.methods, point));
        }
        AstThing::Interface(ast_interface) => {
            out.extend(get_interface_constats(&ast_interface.constants));
        }
    }

    // let n = cursor.goto_first_child_for_point(*point);
    Ok(out)
}

fn get_interface_constats(
    contants: &[AstInterfaceConstant],
) -> impl Iterator<Item = LocalVariable> {
    contants.iter().map(|i| LocalVariable {
        level: 0,
        jtype: (&i.jtype).into(),
        name: (&i.name).into(),
        is_fun: false,
        range: i.range.clone(),
    })
}

fn get_class_methods(
    methods: &[AstClassMethod],
    point: &AstPoint,
) -> impl Iterator<Item = LocalVariable> {
    methods
        .iter()
        .filter(|i| i.range.is_in_range(point))
        .flat_map(|i| {
            interleave(
                i.header
                    .parameters
                    .parameters
                    .iter()
                    .map(|i| LocalVariable {
                        level: 0,
                        jtype: (&i.jtype).into(),
                        name: (&i.name).into(),
                        is_fun: false,
                        range: i.range.clone(),
                    }),
                get_block_vars(&i.block, point),
            )
        })
}

fn get_block_vars(
    block: &ast::types::AstBlock,
    _point: &AstPoint,
) -> impl Iterator<Item = LocalVariable> {
    block.entries.iter().filter_map(|i| match i {
        AstBlockEntry::Variable(i) => Some(LocalVariable {
            level: 1,
            jtype: (&i.jtype).into(),
            name: (&i.name).into(),
            is_fun: false,
            range: i.range.clone(),
        }),
        _ => None,
    })
}

fn get_class_variables(
    variables: &[ast::types::AstClassVariable],
) -> impl Iterator<Item = LocalVariable> {
    variables.iter().map(|i| LocalVariable {
        level: 0,
        jtype: (&i.jtype).into(),
        name: (&i.name).into(),
        is_fun: false,
        range: i.range.clone(),
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
    fn this_context() {
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
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();

        let out = get_vars(&doc, &AstPoint::new(12, 17)).unwrap();
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "hello".to_owned(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 5, col: 11 },
                        end: AstPoint { line: 5, col: 16 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "se".to_owned(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 6, col: 11 },
                        end: AstPoint { line: 6, col: 13 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "other".to_owned(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 8, col: 19 },
                        end: AstPoint { line: 8, col: 24 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Void,
                    name: "hello".to_owned(),
                    is_fun: true,
                    range: AstRange {
                        start: AstPoint { line: 10, col: 16 },
                        end: AstPoint { line: 10, col: 21 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "a".to_owned(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 10, col: 29 },
                        end: AstPoint { line: 10, col: 30 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "local".to_owned(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 11, col: 15 },
                        end: AstPoint { line: 11, col: 20 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("var".to_owned()),
                    name: "lo".to_owned(),
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
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();

        let out = get_vars(&doc, &AstPoint::new(4, 6)).unwrap();
        assert_eq!(
            out,
            vec![LocalVariable {
                level: 2,
                jtype: dto::JType::Class("Logger".to_string()),
                name: "logger".to_string(),
                is_fun: false,
                range: AstRange {
                    start: AstPoint { line: 3, col: 26 },
                    end: AstPoint { line: 3, col: 32 },
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
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();

        let out = get_vars(&doc, &AstPoint::new(12, 17)).unwrap();
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "hello".to_owned(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 5, col: 13 },
                        end: AstPoint { line: 5, col: 18 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "se".to_owned(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 6, col: 13 },
                        end: AstPoint { line: 6, col: 15 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "other".to_owned(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 8, col: 21 },
                        end: AstPoint { line: 8, col: 26 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Void,
                    name: "hello".to_owned(),
                    is_fun: true,
                    range: AstRange {
                        start: AstPoint { line: 10, col: 16 },
                        end: AstPoint { line: 10, col: 21 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "a".to_owned(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 10, col: 31 },
                        end: AstPoint { line: 10, col: 32 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "local".to_owned(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 11, col: 17 },
                        end: AstPoint { line: 11, col: 22 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("var".to_owned()),
                    name: "lo".to_owned(),
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
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();

        let out = get_vars(&doc, &AstPoint::new(8, 54)).unwrap();
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Void,
                    name: "hello".to_string(),
                    is_fun: true,
                    range: AstRange {
                        start: AstPoint { line: 3, col: 16 },
                        end: AstPoint { line: 3, col: 21 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Generic(
                        "List".to_owned(),
                        vec![dto::JType::Class("String".to_string())]
                    ),
                    name: "names".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 4, col: 21 },
                        end: AstPoint { line: 4, col: 26 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Int,
                    name: "i".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 5, col: 17 },
                        end: AstPoint { line: 5, col: 18 },
                    },
                },
                LocalVariable {
                    level: 7,
                    jtype: dto::JType::Class("String".to_owned(),),
                    name: "name".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 6, col: 22 },
                        end: AstPoint { line: 6, col: 26 },
                    },
                },
                LocalVariable {
                    level: 12,
                    jtype: dto::JType::Void,
                    name: "n".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 7, col: 32 },
                        end: AstPoint { line: 7, col: 33 },
                    },
                },
                LocalVariable {
                    level: 12,
                    jtype: dto::JType::Void,
                    name: "m".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 7, col: 35 },
                        end: AstPoint { line: 7, col: 36 },
                    },
                },
                LocalVariable {
                    level: 17,
                    jtype: dto::JType::Void,
                    name: "c".to_string(),
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
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();

        let out = get_vars(&doc, &AstPoint::new(8, 54)).unwrap();
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Void,
                    name: "hello".to_string(),
                    is_fun: true,
                    range: AstRange {
                        start: AstPoint { line: 3, col: 16 },
                        end: AstPoint { line: 3, col: 21 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "fast1".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 5, col: 19 },
                        end: AstPoint { line: 5, col: 24 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "second1".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 6, col: 19 },
                        end: AstPoint { line: 6, col: 26 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "ty1".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 8, col: 19 },
                        end: AstPoint { line: 8, col: 22 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("IOException".to_string()),
                    name: "eio1".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 9, col: 29 },
                        end: AstPoint { line: 9, col: 33 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "ca1".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 10, col: 19 },
                        end: AstPoint { line: 10, col: 22 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "fin".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 12, col: 19 },
                        end: AstPoint { line: 12, col: 22 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "some2".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 16, col: 19 },
                        end: AstPoint { line: 16, col: 24 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("Exception".to_string()),
                    name: "e2".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 17, col: 27 },
                        end: AstPoint { line: 17, col: 29 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "other2".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 18, col: 19 },
                        end: AstPoint { line: 18, col: 25 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "some3".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 22, col: 19 },
                        end: AstPoint { line: 22, col: 24 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("Exception".to_string()),
                    name: "e3".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 23, col: 41 },
                        end: AstPoint { line: 23, col: 43 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "other3".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 24, col: 19 },
                        end: AstPoint { line: 24, col: 25 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("IOException".to_string()),
                    name: "e3".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 25, col: 29 },
                        end: AstPoint { line: 25, col: 31 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "other3".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 26, col: 19 },
                        end: AstPoint { line: 26, col: 25 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "fin3".to_string(),
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
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();

        let out = get_vars(&doc, &AstPoint::new(6, 46)).unwrap();
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Void,
                    name: "ioStuff".to_string(),
                    is_fun: true,
                    range: AstRange {
                        start: AstPoint { line: 3, col: 19 },
                        end: AstPoint { line: 3, col: 26 },
                    },
                },
                LocalVariable {
                    level: 4,
                    jtype: dto::JType::Class("IOException".to_string()),
                    name: "eoeoeoeooe".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 5, col: 29 },
                        end: AstPoint { line: 5, col: 39 },
                    },
                },
            ]
        );
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
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();

        let out = get_vars(&doc, &AstPoint::new(5, 22)).unwrap();
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "options".to_string(),
                    is_fun: true,
                    range: AstRange {
                        start: AstPoint { line: 4, col: 18 },
                        end: AstPoint { line: 4, col: 25 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "outer".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 4, col: 39 },
                        end: AstPoint { line: 4, col: 44 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("String".to_string()),
                    name: "inner".to_string(),
                    is_fun: false,
                    range: AstRange {
                        start: AstPoint { line: 5, col: 13 },
                        end: AstPoint { line: 5, col: 18 },
                    },
                },
            ]
        );
    }
}
