use parser::dto;
use tree_sitter::Point;
use tree_sitter_util::{get_string, CommentSkiper};

use crate::Document;

/// Information about a variable or function in a Document
#[derive(Debug, PartialEq)]
pub struct LocalVariable {
    pub level: usize,
    pub jtype: dto::JType,
    pub name: String,
    pub is_fun: bool,
}

/// Get Local Variables and Functions of the current Document
pub fn get_vars(document: &Document, point: &Point) -> Vec<LocalVariable> {
    let tree = &document.tree;
    let bytes = document
        .text
        .slice(..)
        .as_str()
        .unwrap_or_default()
        .as_bytes();

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
                let var = parse_variable(level, ty, name);
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

fn parse_variable(level: usize, ty: String, name: String) -> LocalVariable {
    LocalVariable {
        level,
        jtype: parse_jtype(ty),
        name,
        is_fun: false,
    }
}

fn parse_jtype(ty: String) -> dto::JType {
    match ty.as_str() {
        "void" => dto::JType::Void,
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
            out.push(parse_variable(level, ty, name));
            cursor.parent();
        }
        cursor.parent();
    }
    cursor.sibling();
    cursor.first_child();
    'method: loop {
        match cursor.node().kind() {
            "local_variable_declaration" => {
                cursor.first_child();
                let ty = get_string(&cursor, bytes);
                cursor.sibling();
                cursor.first_child();
                let name = get_string(&cursor, bytes);
                cursor.sibling();
                let var = parse_variable(level, ty, name);
                out.push(var);
                cursor.parent();
                cursor.parent();
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

#[cfg(test)]
pub mod tests {
    use parser::dto;
    use pretty_assertions::assert_eq;
    use tree_sitter::Point;

    use crate::{
        call_chain::{get_call_chain, CallItem},
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
        let doc = Document::setup(content).unwrap();

        let out = get_vars(&doc, &Point::new(12, 17));
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "hello".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "se".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "other".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Void,
                    name: "hello".to_owned(),
                    is_fun: true,
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "a".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("String".to_owned()),
                    name: "local".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("var".to_owned()),
                    name: "lo".to_owned(),
                    is_fun: false,
                },
            ]
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
        let doc = Document::setup(content).unwrap();

        let out = get_vars(&doc, &Point::new(12, 17));
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "hello".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "se".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "other".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 2,
                    jtype: dto::JType::Void,
                    name: "hello".to_owned(),
                    is_fun: true,
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "a".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_owned()))),
                    name: "local".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 3,
                    jtype: dto::JType::Class("var".to_owned()),
                    name: "lo".to_owned(),
                    is_fun: false,
                },
            ]
        );
    }

    #[test]
    fn symbol_base() {
        let content = "
package ch.emilycares;

public class Test {

    public void hello(String a) {
        String local = \"\";

        var lo = local. 
        return;
    }
}
        ";
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(8, 24));
        assert_eq!(out, Some(vec![CallItem::Variable("local".to_string()),]));
    }

    pub const SYMBOL_METHOD: &str = "
package ch.emilycares;

public class Test {

    public void hello() {
        String local = \"\";

        var lo = local.concat(\"hehe\"). 
        return;
    }
}
        ";

    #[test]
    fn symbol_method() {
        let doc = Document::setup(SYMBOL_METHOD).unwrap();

        let out = get_call_chain(&doc, &Point::new(8, 40));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::MethodCall("concat".to_string()),
                CallItem::FieldAccess("return".to_string()),
            ])
        );
    }

    #[test]
    fn symbol_field() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var lo = local.a.
        return;
    }
}
";
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 26));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::FieldAccess("a".to_string())
            ])
        );
    }

    #[test]
    fn symbol_method_base() {
        let content = "
package ch.emilycares;
public class GreetingResource {
    String a;
    public String hello() {
        a.concat(\"\"). 
        return \"huh\";
    }
}
";
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 24));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("a".to_string()),
                CallItem::MethodCall("concat".to_string())
            ])
        );
    }

    #[test]
    fn symbol_field_method() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var lo = local.a.b().
        return;
    }
}
";
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 30));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::FieldAccess("a".to_string()),
                CallItem::MethodCall("b".to_string()),
                CallItem::FieldAccess("return".to_string()),
            ])
        );
    }

    #[test]
    fn symbol_menthod_field() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var lo = local.a().b.
        return;
    }
}
";
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 30));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::MethodCall("a".to_string()),
                CallItem::FieldAccess("b".to_string())
            ])
        );
    }

    #[test]
    fn symbol_semicolon_simple() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var lo = local. ;
        return;
    }
}
";
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 23));
        assert_eq!(out, Some(vec![CallItem::Variable("local".to_string())]));
    }

    #[test]
    fn symbol_semicolon_field() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        int c = local.a().c.;
        return;
    }
}
";
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 28));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::MethodCall("a".to_string()),
                CallItem::FieldAccess("c".to_string()),
            ])
        );
    }

    #[test]
    fn symbol_semicolon_method() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        int c = local.a.c().;
        return;
    }
}
";
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 28));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::FieldAccess("a".to_string()),
                CallItem::MethodCall("c".to_string()),
            ])
        );
    }

    #[test]
    fn symbol_statement() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        local.a.c().;
        return;
    }
}
";
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 20));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::FieldAccess("a".to_string()),
                CallItem::MethodCall("c".to_string()),
            ])
        );
    }

    #[test]
    fn symbol_class() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        String. 
        return;
    }
}
";
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 16));
        assert_eq!(out, Some(vec![CallItem::Class("String".to_string()),]));
    }

    #[test]
    fn symbol_varible_class() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var local = String. 
        return;
    }
}
";
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 28));
        assert_eq!(out, Some(vec![CallItem::Class("String".to_string()),]));
    }
}
