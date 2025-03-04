use parser::dto;
use tree_sitter::{Point, Range};
use tree_sitter_util::{get_string, CommentSkiper};

use crate::Document;

/// Information about a variable or function in a Document
#[derive(Debug, PartialEq, Clone)]
pub struct LocalVariable {
    pub level: usize,
    pub jtype: dto::JType,
    pub name: String,
    pub is_fun: bool,
    pub range: Range,
}

/// Get Local Variables and Functions of the current Document
pub fn get_vars(document: &Document, point: &Point) -> Vec<LocalVariable> {
    let tree = &document.tree;
    let bytes = document.as_bytes();

    let mut cursor = tree.walk();
    let mut level = 0;
    let mut out: Vec<LocalVariable> = vec![];
    loop {
        match cursor.node().kind() {
            "class_declaration" => {}
            "method_declaration" => {
                get_method_vars(tree, cursor.node(), bytes, &mut out, level);
            }
            "class_body" => {
                get_class_vars(tree, cursor.node(), bytes, &mut out, level);
            }
            "for_statement" => {
                cursor.first_child();
                cursor.sibling();
                cursor.sibling();
                parse_local_variable_declaration(&mut cursor, bytes, level, &mut out);
                cursor.parent();
            }
            // for (String a : list) {
            "enhanced_for_statement" => {
                cursor.first_child();
                cursor.sibling();
                cursor.sibling();
                let ty = get_string(&cursor, bytes);
                cursor.sibling();
                let name = get_string(&cursor, bytes);
                let var = parse_variable(level, ty, name, cursor.node().range());
                out.push(var);
                cursor.parent();
            }
            "lambda_expression" => {
                get_lambda_vars(&mut cursor, bytes, level, &mut out);
            }
            _ => {}
        }

        let n = cursor.goto_first_child_for_point(*point);
        level += 1;
        if n.is_none() {
            break;
        }
        if level >= 200 {
            break;
        }
    }

    out
}

fn get_lambda_vars(
    cursor: &mut tree_sitter::TreeCursor<'_>,
    bytes: &[u8],
    level: usize,
    out: &mut Vec<LocalVariable>,
) {
    cursor.first_child();
    match cursor.node().kind() {
        "identifier" => {
            let name = get_string(&*cursor, bytes);
            let var = parse_variable(level, "void".to_string(), name, cursor.node().range());
            out.push(var);
        }
        "inferred_parameters" => {
            cursor.first_child();
            cursor.sibling();
            let name = get_string(&*cursor, bytes);
            let var = parse_variable(level, "void".to_string(), name, cursor.node().range());
            out.push(var);
            cursor.sibling();
            while cursor.node().kind() == "," {
                cursor.sibling();
                let name = get_string(&*cursor, bytes);
                let var = parse_variable(level, "void".to_string(), name, cursor.node().range());
                out.push(var);
            }
            cursor.parent();
        }
        _ => {}
    }
    cursor.parent();
}

/// Get all vars of class
fn get_class_vars(
    tree: &tree_sitter::Tree,
    start_node: tree_sitter::Node,
    bytes: &[u8],
    out: &mut Vec<LocalVariable>,
    level: usize,
) {
    let mut cursor = tree.walk();
    cursor.reset(start_node);
    cursor.first_child();
    cursor.first_child();
    'class: loop {
        match cursor.node().kind() {
            "field_declaration" => {
                cursor.first_child();
                if cursor.node().kind() == "modifiers" {
                    cursor.sibling();
                }
                let ty = get_string(&cursor, bytes);
                cursor.sibling();
                cursor.first_child();
                let name = get_string(&cursor, bytes);
                let var = parse_variable(level, ty, name, cursor.node().range());
                out.push(var);

                cursor.parent();
                cursor.parent();
            }
            "method_declaration" => {
                cursor.first_child();
                if cursor.node().kind() == "modifiers" {
                    cursor.sibling();
                }
                let ty = get_string(&cursor, bytes);
                cursor.sibling();
                let name = get_string(&cursor, bytes);
                out.push(LocalVariable {
                    level,
                    jtype: parse_jtype(ty),
                    name,
                    is_fun: true,
                    range: cursor.node().range(),
                });

                cursor.parent();
            }
            "{" | "}" => {}
            _ => {}
        }
        if !cursor.sibling() {
            break 'class;
        }
    }
}

fn parse_variable(level: usize, ty: String, name: String, range: Range) -> LocalVariable {
    LocalVariable {
        level,
        jtype: parse_jtype(ty),
        name,
        is_fun: false,
        range,
    }
}

fn parse_jtype(ty: String) -> dto::JType {
    match ty.as_str() {
        "void" => dto::JType::Void,
        "int" => dto::JType::Int,
        ty if ty.ends_with("[]") => {
            let ty = ty[..ty.len() - 2].to_string();
            dto::JType::Array(Box::new(parse_jtype(ty)))
        }
        ty => dto::JType::Class(ty.to_string()),
    }
}

/// Get all vars of method
fn get_method_vars(
    tree: &tree_sitter::Tree,
    start_node: tree_sitter::Node,
    bytes: &[u8],
    out: &mut Vec<LocalVariable>,
    level: usize,
) {
    let mut cursor = tree.walk();
    cursor.reset(start_node);
    cursor.first_child();
    cursor.sibling();
    cursor.sibling();
    cursor.sibling();
    if cursor.node().kind() == "formal_parameters" {
        cursor.first_child();
        while cursor.sibling() {
            if cursor.node().kind() != "formal_parameter" {
                continue;
            }
            cursor.first_child();
            let ty = get_string(&cursor, bytes);
            cursor.sibling();
            let name = get_string(&cursor, bytes);
            out.push(parse_variable(level, ty, name, cursor.node().range()));
            cursor.parent();
        }
        cursor.parent();
    }
    cursor.sibling();
    cursor.first_child();
    'method: loop {
        match cursor.node().kind() {
            "local_variable_declaration" => {
                parse_local_variable_declaration(&mut cursor, bytes, level, out);
            }
            "{" | "}" => {}
            _ => {}
        }
        if !cursor.sibling() {
            break 'method;
        }
    }
    cursor.parent();
}

fn parse_local_variable_declaration(
    cursor: &mut tree_sitter::TreeCursor<'_>,
    bytes: &[u8],
    level: usize,
    out: &mut Vec<LocalVariable>,
) {
    cursor.first_child();
    let ty = get_string(&*cursor, bytes);
    cursor.sibling();
    cursor.first_child();
    let name = get_string(&*cursor, bytes);
    let range = cursor.node().range();
    cursor.sibling();
    let var = parse_variable(level, ty, name, range);
    out.push(var);
    cursor.parent();
    cursor.parent();
}

#[cfg(test)]
pub mod tests {
    use std::path::PathBuf;

    use parser::dto;
    use pretty_assertions::assert_eq;
    use tree_sitter::{Point, Range};

    use crate::{
        variable::{get_vars, LocalVariable},
        Document,
    };

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
        let doc = Document::setup(content, PathBuf::new()).unwrap();

        let out = get_vars(&doc, &Point::new(12, 17));
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "hello".to_owned(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 57,
                        end_byte: 62,
                        start_point: Point { row: 5, column: 11 },
                        end_point: Point { row: 5, column: 16 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "se".to_owned(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 75,
                        end_byte: 77,
                        start_point: Point { row: 6, column: 11 },
                        end_point: Point { row: 6, column: 13 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "other".to_owned(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 99,
                        end_byte: 104,
                        start_point: Point { row: 8, column: 19 },
                        end_point: Point { row: 8, column: 24 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Void,
                    name: "hello".to_owned(),
                    is_fun: true,
                    range: tree_sitter::Range {
                        start_byte: 128,
                        end_byte: 133,
                        start_point: Point {
                            row: 10,
                            column: 16,
                        },
                        end_point: Point {
                            row: 10,
                            column: 21,
                        },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "a".to_owned(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 141,
                        end_byte: 142,
                        start_point: Point {
                            row: 10,
                            column: 29,
                        },
                        end_point: Point {
                            row: 10,
                            column: 30,
                        },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "local".to_owned(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 161,
                        end_byte: 166,
                        start_point: Point {
                            row: 11,
                            column: 15,
                        },
                        end_point: Point {
                            row: 11,
                            column: 20,
                        },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("var".to_owned()),
                    name: "lo".to_owned(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 186,
                        end_byte: 188,
                        start_point: Point {
                            row: 13,
                            column: 12,
                        },
                        end_point: Point {
                            row: 13,
                            column: 14,
                        },
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
        let doc = Document::setup(content, PathBuf::new()).unwrap();

        let out = get_vars(&doc, &Point::new(4, 6));
        assert_eq!(
            out,
            vec![LocalVariable {
                level: 2,
                jtype: dto::JType::Class("Logger".to_string()),
                name: "logger".to_string(),
                is_fun: false,
                range: Range {
                    start_byte: 70,
                    end_byte: 76,
                    start_point: Point { row: 3, column: 26 },
                    end_point: Point { row: 3, column: 32 },
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
        let doc = Document::setup(content, PathBuf::new()).unwrap();

        let out = get_vars(&doc, &Point::new(12, 17));
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "hello".to_owned(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 59,
                        end_byte: 64,
                        start_point: Point { row: 5, column: 13 },
                        end_point: Point { row: 5, column: 18 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "se".to_owned(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 79,
                        end_byte: 81,
                        start_point: Point { row: 6, column: 13 },
                        end_point: Point { row: 6, column: 15 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "other".to_owned(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 105,
                        end_byte: 110,
                        start_point: Point { row: 8, column: 21 },
                        end_point: Point { row: 8, column: 26 },
                    },
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Void,
                    name: "hello".to_owned(),
                    is_fun: true,
                    range: tree_sitter::Range {
                        start_byte: 134,
                        end_byte: 139,
                        start_point: Point {
                            row: 10,
                            column: 16,
                        },
                        end_point: Point {
                            row: 10,
                            column: 21,
                        },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "a".to_owned(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 149,
                        end_byte: 150,
                        start_point: Point {
                            row: 10,
                            column: 31,
                        },
                        end_point: Point {
                            row: 10,
                            column: 32,
                        },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "local".to_owned(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 171,
                        end_byte: 176,
                        start_point: Point {
                            row: 11,
                            column: 17,
                        },
                        end_point: Point {
                            row: 11,
                            column: 22,
                        },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("var".to_owned()),
                    name: "lo".to_owned(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 196,
                        end_byte: 198,
                        start_point: Point {
                            row: 13,
                            column: 12,
                        },
                        end_point: Point {
                            row: 13,
                            column: 14,
                        },
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
        let doc = Document::setup(content, PathBuf::new()).unwrap();

        let out = get_vars(&doc, &Point::new(8, 54));
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Void,
                    name: "hello".to_string(),
                    is_fun: true,
                    range: tree_sitter::Range {
                        start_byte: 60,
                        end_byte: 65,
                        start_point: Point { row: 3, column: 16 },
                        end_point: Point { row: 3, column: 21 },
                    },
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("List<String>".to_owned(),),
                    name: "names".to_string(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 91,
                        end_byte: 96,
                        start_point: Point { row: 4, column: 21 },
                        end_point: Point { row: 4, column: 26 },
                    },
                },
                LocalVariable {
                    level: 5,
                    jtype: dto::JType::Int,
                    name: "i".to_string(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 135,
                        end_byte: 136,
                        start_point: Point { row: 5, column: 17 },
                        end_point: Point { row: 5, column: 18 },
                    },
                },
                LocalVariable {
                    level: 7,
                    jtype: dto::JType::Class("String".to_owned(),),
                    name: "name".to_string(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 178,
                        end_byte: 182,
                        start_point: Point { row: 6, column: 22 },
                        end_point: Point { row: 6, column: 26 },
                    },
                },
                LocalVariable {
                    level: 12,
                    jtype: dto::JType::Void,
                    name: "n".to_string(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 226,
                        end_byte: 227,
                        start_point: Point { row: 7, column: 32 },
                        end_point: Point { row: 7, column: 33 },
                    },
                },
                LocalVariable {
                    level: 12,
                    jtype: dto::JType::Void,
                    name: "m".to_string(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 229,
                        end_byte: 230,
                        start_point: Point { row: 7, column: 35 },
                        end_point: Point { row: 7, column: 36 },
                    },
                },
                LocalVariable {
                    level: 17,
                    jtype: dto::JType::Void,
                    name: "c".to_string(),
                    is_fun: false,
                    range: tree_sitter::Range {
                        start_byte: 285,
                        end_byte: 286,
                        start_point: Point { row: 8, column: 48 },
                        end_point: Point { row: 8, column: 49 },
                    },
                },
            ]
        );
    }
}
