use tree_sitter::Point;
use tree_sitter_util::{get_string, tdbc, CommentSkiper};

use crate::Document;

/// Information about a variable or function in a Document
#[derive(Debug, PartialEq)]
pub struct LocalVariable {
    pub level: usize,
    pub jtype: String,
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
                out.push(LocalVariable {
                    level,
                    jtype: ty,
                    name,
                    is_fun: false,
                });

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
                    jtype: ty,
                    name,
                    is_fun: true,
                });
                cursor.parent();
            }
            "{" | "}" => {}
            _ => {
                //dbg!(class_cursor.node().kind());
                //dbg!(get_string(&class_cursor, &bytes));
            }
        }
        if !cursor.sibling() {
            break 'class;
        }
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
            out.push(LocalVariable {
                level,
                jtype: ty,
                name,
                is_fun: false,
            });
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
                out.push(LocalVariable {
                    level,
                    jtype: ty,
                    name,
                    is_fun: false,
                });
                cursor.parent();
                cursor.parent();
            }
            "{" | "}" => {}
            _ => {
                //dbg!(method_cursor.node().kind());
                //dbg!(get_string(&method_cursor, &bytes));
            }
        }
        if !cursor.sibling() {
            break 'method;
        }
    }
    cursor.parent();
}

#[derive(Debug, PartialEq)]
pub enum CallItem {
    MethodCall(String),
    FieldAccess(String),
    Variable(String),
    Class(String),
}

/// Provides data abuilt the current variable before the cursor
/// ``` java
/// Long other = 1l;
/// other.
///       ^
/// ```
/// Then it would return info about the variable other
pub fn current_symbol<'a>(document: &Document, point: &Point) -> Option<Vec<CallItem>> {
    let tree = &document.tree;
    let bytes = document
        .text
        .slice(..)
        .as_str()
        .unwrap_or_default()
        .as_bytes();

    let mut cursor = tree.walk();
    let mut level = 0;
    let mut out = vec![];
    loop {
        match cursor.node().kind() {
            "scoped_type_identifier" => {
                let l = get_string(&cursor, bytes);
                let l = l.split_once("\n").unwrap_or_default().0;
                let l = l.trim();
                let l = l.trim_end_matches('.');

                if let Some(c) = l.chars().next() {
                    let val = match c.is_uppercase() {
                        true => CallItem::Class(l.to_string()),
                        false => CallItem::Variable(l.to_string()),
                    };
                    out.push(val);
                }
            }
            "template_expression" => {
                cursor.first_child();
                match cursor.node().kind() {
                    "method_invocation" => {
                        out.extend(parse_method_invocation(cursor.node(), bytes));
                    }
                    _ => {}
                }
                cursor.parent();
            }
            "expression_statement" => {
                cursor.first_child();
                out.extend(parse_value(&cursor, bytes));
                cursor.parent();
            }
            "local_variable_declaration" => {
                cursor.first_child();
                cursor.sibling();
                if cursor.node().kind() == "variable_declarator" {
                    cursor.first_child();
                    cursor.sibling();
                    cursor.sibling();
                    out.extend(parse_value(&cursor, bytes));
                    cursor.parent();
                }
                cursor.parent();
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
    if !out.is_empty() {
        return Some(out);
    }
    None
}

fn parse_value(cursor: &tree_sitter::TreeCursor<'_>, bytes: &[u8]) -> Vec<CallItem> {
    match cursor.node().kind() {
        "identifier" => vec![CallItem::Variable(get_string(cursor, bytes))],
        "field_access" => parse_field_access(cursor.node(), bytes),
        "method_invocation" => parse_method_invocation(cursor.node(), bytes),
        _ => vec![],
    }
}

fn parse_method_invocation(node: tree_sitter::Node<'_>, bytes: &[u8]) -> Vec<CallItem> {
    let mut cursor = node.walk();
    let mut out = vec![];
    cursor.first_child();
    match cursor.node().kind() {
        "field_access" => {
            out.extend(parse_field_access(node, bytes));
            cursor.sibling();
            cursor.sibling();
            let method_name = get_string(&cursor, bytes);
            out.extend(vec![CallItem::MethodCall(method_name)]);
        }
        "identifier" => {
            let var_name = get_string(&cursor, bytes);
            cursor.sibling();
            cursor.sibling();
            let method_name = get_string(&cursor, bytes);
            out.extend(vec![
                CallItem::Variable(var_name),
                CallItem::MethodCall(method_name),
            ]);
        }
        "method_invocation" => {
            out.extend(parse_method_invocation(cursor.node(), bytes));
        }
        _ => {}
    };
    cursor.parent();
    out
}

fn parse_field_access(node: tree_sitter::Node<'_>, bytes: &[u8]) -> Vec<CallItem> {
    let mut cursor = node.walk();
    let mut out = vec![];
    cursor.first_child();
    match cursor.node().kind() {
        "identifier" => {
            let var_name = get_string(&cursor, bytes);
            cursor.sibling();
            cursor.sibling();
            let field_name = get_string(&cursor, bytes);
            if let Some(c) = var_name.chars().next() {
                let item = match c.is_uppercase() {
                    true => CallItem::Class(var_name),
                    false => CallItem::Variable(var_name),
                };
                out.push(item);
                if field_name != "return" {
                    out.push(CallItem::FieldAccess(field_name));
                }
            }
        }
        "method_invocation" => {
            out.extend(parse_method_invocation(cursor.node(), bytes));
            cursor.sibling();
            cursor.sibling();
            let field_name = get_string(&cursor, bytes);
            out.extend(vec![CallItem::FieldAccess(field_name)]);
        }
        "field_access" => {
            out.extend(parse_field_access(cursor.node(), bytes));
        }
        _ => {}
    }
    cursor.parent();
    out
}

#[cfg(test)]
pub mod tests {
    use pretty_assertions::assert_eq;
    use tree_sitter::Point;

    use crate::{
        variable::{current_symbol, get_vars, CallItem, LocalVariable},
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
                    jtype: "String".to_owned(),
                    name: "hello".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 2,
                    jtype: "String".to_owned(),
                    name: "se".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 2,
                    jtype: "String".to_owned(),
                    name: "other".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 2,
                    jtype: "void".to_owned(),
                    name: "hello".to_owned(),
                    is_fun: true,
                },
                LocalVariable {
                    level: 3,
                    jtype: "String".to_owned(),
                    name: "a".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 3,
                    jtype: "String".to_owned(),
                    name: "local".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 3,
                    jtype: "var".to_owned(),
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

        let out = current_symbol(&doc, &Point::new(8, 24));
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

        let out = current_symbol(&doc, &Point::new(8, 40));
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

        let out = current_symbol(&doc, &Point::new(5, 26));
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

        let out = current_symbol(&doc, &Point::new(5, 24));
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

        let out = current_symbol(&doc, &Point::new(5, 30));
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

        let out = current_symbol(&doc, &Point::new(5, 30));
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

        let out = current_symbol(&doc, &Point::new(5, 23));
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

        let out = current_symbol(&doc, &Point::new(5, 28));
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

        let out = current_symbol(&doc, &Point::new(5, 28));
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

        let out = current_symbol(&doc, &Point::new(5, 20));
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

        let out = current_symbol(&doc, &Point::new(5, 16));
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

        let out = current_symbol(&doc, &Point::new(5, 28));
        assert_eq!(out, Some(vec![CallItem::Class("String".to_string()),]));
    }
}
