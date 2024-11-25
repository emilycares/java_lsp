use tree_sitter::Point;
use tree_sitter_util::{get_string, CommentSkiper};

use crate::Document;

/// Information about a variable or function in a Document
#[derive(Debug, PartialEq)]
pub struct LocalVariable {
    pub level: usize,
    pub ty: String,
    pub name: String,
    pub is_fun: bool,
}

/// Get Local Variables and Functions of the current Document
pub fn get_vars<'a>(document: &Document, point: &Point) -> Vec<LocalVariable> {
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
            "class_body" => {
                let mut class_cursor = tree.walk();
                class_cursor.reset(cursor.node());
                class_cursor.first_child();
                class_cursor.first_child();
                'class: loop {
                    match class_cursor.node().kind() {
                        "field_declaration" => {
                            class_cursor.first_child();
                            if class_cursor.node().kind() == "modifiers" {
                                class_cursor.sibling();
                            }
                            let ty = get_string(&class_cursor, &bytes);
                            class_cursor.sibling();
                            class_cursor.first_child();

                            let name = get_string(&class_cursor, &bytes);
                            out.push(LocalVariable {
                                level,
                                ty,
                                name,
                                is_fun: false,
                            });

                            class_cursor.parent();
                            class_cursor.parent();
                        }
                        "method_declaration" => {
                            class_cursor.first_child();
                            if class_cursor.node().kind() == "modifiers" {
                                class_cursor.sibling();
                            }
                            let ty = get_string(&class_cursor, &bytes);
                            class_cursor.sibling();
                            let name = get_string(&class_cursor, &bytes);
                            out.push(LocalVariable {
                                level,
                                ty,
                                name,
                                is_fun: true,
                            });
                            class_cursor.parent();
                        }
                        "{" | "}" => {}
                        _ => {
                            //dbg!(class_cursor.node().kind());
                            //dbg!(get_string(&class_cursor, &bytes));
                        }
                    }
                    if !class_cursor.sibling() {
                        break 'class;
                    }
                }
            }
            "method_declaration" => {
                let mut method_cursor = tree.walk();
                method_cursor.reset(cursor.node());
                method_cursor.first_child();
                method_cursor.sibling();
                method_cursor.sibling();
                method_cursor.sibling();
                method_cursor.sibling();
                method_cursor.first_child();
                'method: loop {
                    match method_cursor.node().kind() {
                        "local_variable_declaration" => {
                            method_cursor.first_child();
                            let ty = get_string(&method_cursor, &bytes);
                            method_cursor.sibling();
                            method_cursor.first_child();
                            let name = get_string(&method_cursor, &bytes);
                            method_cursor.sibling();
                            out.push(LocalVariable {
                                level,
                                ty,
                                name,
                                is_fun: false,
                            });
                            method_cursor.parent();
                            method_cursor.parent();
                        }
                        "{" | "}" => {}
                        _ => {
                            //dbg!(method_cursor.node().kind());
                            //dbg!(get_string(&method_cursor, &bytes));
                        }
                    }
                    if !method_cursor.sibling() {
                        break 'method;
                    }
                }
                method_cursor.parent();
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
    lo_va: &'a Vec<LocalVariable>,
) -> Option<&'a LocalVariable> {
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
        // dbg!(cursor.node().kind());
        // dbg!(get_string(&cursor, bytes));
        // This scoped_type_identifier thing does not work. Real world this works.
        if cursor.node().kind() == "." {
            let l = prev.trim_end_matches('.');
            let l = l.trim();
            if let Some(lo) = lo_va.iter().find(|va| va.name == l) {
                return Some(lo);
            }
        }
        prev = get_string(&cursor, bytes);

        if cursor.node().kind() == "scoped_type_identifier" {
            let l = get_string(&cursor, &bytes);
            let l = l.split_once("\n").unwrap_or_default().0;
            let l = l.trim();
            let l = l.trim_end_matches('.');

            let lo = lo_va.iter().find(|va| va.name == l);

            return lo;
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

#[cfg(test)]
mod tests {
    use tree_sitter::Point;

    use crate::{
        variable::{current_symbol, get_vars, LocalVariable},
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
    public TemplateInstance hello() {
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
                    ty: "Template".to_owned(),
                    name: "hello".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 2,
                    ty: "Template".to_owned(),
                    name: "se".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 2,
                    ty: "String".to_owned(),
                    name: "other".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 2,
                    ty: "TemplateInstance".to_owned(),
                    name: "hello".to_owned(),
                    is_fun: true,
                },
                LocalVariable {
                    level: 3,
                    ty: "String".to_owned(),
                    name: "local".to_owned(),
                    is_fun: false,
                },
                LocalVariable {
                    level: 3,
                    ty: "var".to_owned(),
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
            ty: "String".to_owned(),
            name: "local".to_owned(),
            is_fun: false,
        }];

        let out = current_symbol(&doc, &Point::new(27, 24), &lo_va);
        assert_eq!(
            out,
            Some(&LocalVariable {
                level: 3,
                ty: "String".to_owned(),
                name: "local".to_owned(),
                is_fun: false,
            })
        );
    }
}
