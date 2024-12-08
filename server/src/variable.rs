use tree_sitter::{Point, TreeCursor};
use tree_sitter_util::{get_string, CommentSkiper};

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

fn tdbc(cursor: &TreeCursor, bytes: &[u8]) {
    eprintln!(
        "{} - kind:{} - text:\"{}\"",
        cursor.node().to_sexp(),
        cursor.node().kind(),
        get_string(&cursor, bytes)
    );
}

#[derive(Debug, PartialEq)]
pub enum CallItem<'a> {
    MethodCall(String),
    Variable(&'a LocalVariable),
}

/// Provides data abuilt the current variable before the cursor
/// ``` java
/// Long other = 1l;
/// other.
///       ^
/// ```
/// Then it would return info about the variable other
pub fn current_symbol<'a>(
    document: &Document,
    point: &Point,
    lo_va: &'a [LocalVariable],
) -> Option<Vec<CallItem<'a>>> {
    let tree = &document.tree;
    let bytes = document
        .text
        .slice(..)
        .as_str()
        .unwrap_or_default()
        .as_bytes();

    let mut cursor = tree.walk();
    let mut level = 0;
    let mut prev = String::new();
    loop {
        // This scoped_type_identifier thing does not work. Real world this works.
        if cursor.node().kind() == "." {
            let l = prev.trim_end_matches('.');
            let l = l.trim();
            if let Some(lo) = lo_va.iter().find(|va| va.name == l) {
                return Some(vec![CallItem::Variable(lo)]);
            }
        }
        prev = get_string(&cursor, bytes);

        if cursor.node().kind() == "scoped_type_identifier" {
            let l = get_string(&cursor, bytes);
            let l = l.split_once("\n").unwrap_or_default().0;
            let l = l.trim();
            let l = l.trim_end_matches('.');

            if let Some(lo) = lo_va.iter().find(|va| va.name == l) {
                return Some(vec![CallItem::Variable(lo)]);
            }
        }

        if cursor.node().kind() == "field_access" || cursor.node().kind() == "template_expression" {
            cursor.first_child();
            match cursor.node().kind() {
                "method_invocation" => {
                    let (variable, called) = parse_method_invocation(cursor.node(), bytes);
                    if let Some(lo) = lo_va.iter().find(|va| va.name == variable) {
                        return Some(vec![CallItem::Variable(lo), CallItem::MethodCall(called)]);
                    }
                }
                _ => {
                    tdbc(&cursor, bytes);
                }
            }
            // method_invocation
            // end method_invocation
            cursor.parent();
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
    None
}

fn parse_method_invocation(node: tree_sitter::Node<'_>, bytes: &[u8]) -> (String, String) {
    let mut cursor = node.walk();
    cursor.first_child();
    let var_name = get_string(&cursor, bytes);
    cursor.sibling();
    cursor.sibling();
    let method_name = get_string(&cursor, bytes);
    cursor.parent();
    (var_name, method_name)
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

import jakarta.inject.Inject;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;

import io.quarkus.qute.TemplateInstance;
import io.quarkus.qute.Template;

@Path(\"/user/interact\")
public class GreetingResource {

    @Inject
    Template hello;
    @Inject
    Template se;

    private String other = \"\";

    @GET
    @Produces(MediaType.TEXT_HTML)
    public TemplateInstance hello(String a) {
	    String local = \"\";

        var lo = 
	    return hello.data(\"name\", \"emilycares\");
    }
}
        ";
        let doc = Document::setup(content).unwrap();

        let out = get_vars(&doc, &Point::new(26, 17));
        assert_eq!(
            out,
            vec![
                LocalVariable {
                    level: 2,
                    jtype: "Template".to_owned(),
                    name: "hello".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 2,
                    jtype: "Template".to_owned(),
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
                    jtype: "TemplateInstance".to_owned(),
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

import jakarta.inject.Inject;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;

import io.quarkus.qute.TemplateInstance;
import io.quarkus.qute.Template;

@Path(\"/user/interact\")
public class GreetingResource {

    @Inject
    Template hello;
    @Inject
    Template se;

    private String other = \"\";

    @GET
    @Produces(MediaType.TEXT_HTML)
    public TemplateInstance hello() {
	    String local = \"\";

        var lo = local. 
	    return hello.data(\"name\", \"emilycares\");
    }
}
        ";
        let doc = Document::setup(content).unwrap();
        let lo_va = vec![LocalVariable {
            level: 3,
            jtype: "String".to_owned(),
            name: "local".to_owned(),
            is_fun: false,
        }];

        let out = current_symbol(&doc, &Point::new(27, 24), &lo_va);
        assert_eq!(
            out,
            Some(vec![CallItem::Variable(&LocalVariable {
                level: 3,
                jtype: "String".to_owned(),
                name: "local".to_owned(),
                is_fun: false,
            })])
        );
    }

    pub const SYMBOL_METHOD: &str = "
package ch.emilycares;

import jakarta.inject.Inject;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;

import io.quarkus.qute.TemplateInstance;
import io.quarkus.qute.Template;

@Path(\"/user/interact\")
public class GreetingResource {

    @Inject
    Template hello;
    @Inject
    Template se;

    private String other = \"\";

    @GET
    @Produces(MediaType.TEXT_HTML)
    public TemplateInstance hello() {
	    String local = \"\";

        var lo = local.concat(\"hehe\"). 
	    return hello.data(\"name\", \"emilycares\");
    }
        ";

    #[test]
    fn symbol_method() {
        let doc = Document::setup(SYMBOL_METHOD).unwrap();
        let lo_va = vec![LocalVariable {
            level: 3,
            jtype: "String".to_owned(),
            name: "local".to_owned(),
            is_fun: false,
        }];

        let out = current_symbol(&doc, &Point::new(27, 40), &lo_va);
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable(&LocalVariable {
                    level: 3,
                    jtype: "String".to_owned(),
                    name: "local".to_owned(),
                    is_fun: false,
                }),
                CallItem::MethodCall("concat".to_string())
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
        let lo_va = vec![LocalVariable {
            level: 3,
            jtype: "String".to_owned(),
            name: "a".to_owned(),
            is_fun: false,
        }];

        let out = current_symbol(&doc, &Point::new(5, 24), &lo_va);
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable(&LocalVariable {
                    level: 3,
                    jtype: "String".to_owned(),
                    name: "a".to_owned(),
                    is_fun: false,
                }),
                CallItem::MethodCall("concat".to_string())
            ])
        );
    }
}
