use parser::dto;
use tower_lsp::lsp_types::CompletionItem;
use tree_sitter::Point;
use tree_sitter_util::CommentSkiper;

use crate::Document;

#[derive(Debug, PartialEq)]
pub struct LevelThing {
    pub level: usize,
    pub ty: String,
    pub name: String,
    pub is_fun: bool,
}

pub fn find<'a>(document: &Document, point: &Point) -> Vec<LevelThing> {
    let tree = &document.tree;
    let bytes = document
        .text
        .slice(..)
        .as_str()
        .unwrap_or_default()
        .as_bytes();

    let mut cursor = tree.walk();
    let mut level = 0;
    let mut out: Vec<LevelThing> = vec![];
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
                            out.push(LevelThing {
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
                            out.push(LevelThing {
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
                            if method_cursor.node().kind() == "type_identifier" {
                                method_cursor.sibling();
                            }
                            method_cursor.first_child();
                            let name = get_string(&method_cursor, &bytes);
                            method_cursor.sibling();
                            out.push(LevelThing {
                                level,
                                ty: String::new(),
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

fn get_string(cursor: &tree_sitter::TreeCursor<'_>, bytes: &[u8]) -> String {
    cursor.node().utf8_text(bytes).unwrap().to_owned()
}

pub fn class(val: &dto::Class) -> CompletionItem {
    let methods: Vec<_> = val
        .methods
        .iter()
        .map(|m| {
            format!(
                "{}({:?})",
                m.name,
                m.methods
                    .iter()
                    .map(|p| format!("{}", p.name))
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    CompletionItem::new_simple(val.name.to_string(), methods.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::find;
    use crate::{completion::LevelThing, Document};
    use pretty_assertions::assert_eq;
    use tree_sitter::Point;

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
	    var local = \"\";

        var lo = 
	    return hello.data(\"name\", \"emilycares\");
    }
}
        ";
        let doc = Document::setup(content).unwrap();

        let out = find(&doc, &Point::new(24, 17));
        assert_eq!(
            out,
            vec![
                LevelThing {
                    level: 2,
                    ty: "Template".to_owned(),
                    name: "hello".to_owned(),
                    is_fun: false,
                },
                LevelThing {
                    level: 2,
                    ty: "Template".to_owned(),
                    name: "se".to_owned(),
                    is_fun: false,
                },
                LevelThing {
                    level: 2,
                    ty: "String".to_owned(),
                    name: "other".to_owned(),
                    is_fun: false,
                },
                LevelThing {
                    level: 2,
                    ty: "TemplateInstance".to_owned(),
                    name: "hello".to_owned(),
                    is_fun: true,
                },
                LevelThing {
                    level: 3,
                    ty: "".to_owned(),
                    name: "local".to_owned(),
                    is_fun: false,
                },
                LevelThing {
                    level: 3,
                    ty: "".to_owned(),
                    name: "lo".to_owned(),
                    is_fun: false,
                },
            ]
        );
    }
}
