use parser::dto;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, CompletionItemLabelDetails};
use tree_sitter::Point;
use tree_sitter_util::CommentSkiper;

use crate::{tyres, Document};

#[derive(Debug, PartialEq)]
pub struct LocaleVariableFunction {
    pub level: usize,
    pub ty: String,
    pub name: String,
    pub is_fun: bool,
}

/// Get Locale variables and Functions
pub fn get_vars<'a>(document: &Document, point: &Point) -> Vec<LocaleVariableFunction> {
    let tree = &document.tree;
    let bytes = document
        .text
        .slice(..)
        .as_str()
        .unwrap_or_default()
        .as_bytes();

    let mut cursor = tree.walk();
    let mut level = 0;
    let mut out: Vec<LocaleVariableFunction> = vec![];
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
                            out.push(LocaleVariableFunction {
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
                            out.push(LocaleVariableFunction {
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
                            out.push(LocaleVariableFunction {
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

pub fn complete_vars(vars: &Vec<LocaleVariableFunction>) -> Vec<CompletionItem> {
    vars.iter()
        .map(|a| CompletionItem {
            label: a.name.to_owned(),
            label_details: Some(CompletionItemLabelDetails {
                detail: Some(a.ty.to_string()),
                ..Default::default()
            }),
            kind: match a.is_fun {
                true => Some(CompletionItemKind::FUNCTION),
                false => Some(CompletionItemKind::VARIABLE),
            },
            ..Default::default()
        })
        .collect()
}

fn get_string(cursor: &tree_sitter::TreeCursor<'_>, bytes: &[u8]) -> String {
    cursor.node().utf8_text(bytes).unwrap().to_owned()
}

pub fn class_describe(val: &dto::Class) -> CompletionItem {
    let methods: Vec<_> = val
        .methods
        .iter()
        .map(|m| {
            format!(
                "{}({:?})",
                m.name,
                m.parameters
                    .iter()
                    .map(|p| format!("{}", p.jtype))
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    CompletionItem::new_simple(val.name.to_string(), methods.join("\n"))
}
pub fn class_unpack(val: &dto::Class) -> Vec<CompletionItem> {
    let mut out = vec![];

    out.extend(
        val.methods
            .iter()
            .filter(|i| i.access.contains(&parser::dto::Access::Public))
            .map(|m| {
                let params: Vec<String> = m
                    .parameters
                    .iter()
                    .map(|p| match &p.name {
                        Some(name) => format!("{} {}", p.jtype, name),
                        None => p.jtype.to_string(),
                    })
                    .collect();

                CompletionItem {
                    label: m.name.to_owned(),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: Some(format!("{} ({})", m.ret, params.join(", "))),
                        ..Default::default()
                    }),
                    kind: Some(CompletionItemKind::FUNCTION),
                    ..Default::default()
                }
            }),
    );

    out.extend(
        val.fields
            .iter()
            .filter(|i| i.access.contains(&parser::dto::Access::Public))
            .map(|f| CompletionItem {
                label: f.name.to_owned(),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some(f.jtype.to_string()),
                    ..Default::default()
                }),
                kind: Some(CompletionItemKind::FUNCTION),
                ..Default::default()
            }),
    );

    out.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));

    return out;
}

pub fn current_symbol<'a>(
    document: &Document,
    point: &Point,
    lo_va_fu: &'a Vec<LocaleVariableFunction>,
) -> Option<&'a LocaleVariableFunction> {
    let tree = &document.tree;
    let bytes = document
        .text
        .slice(..)
        .as_str()
        .unwrap_or_default()
        .as_bytes();

    let mut cursor = tree.walk();
    let mut level = 0;
    loop {
        if cursor.node().kind() == "scoped_type_identifier" {
            let l = get_string(&cursor, &bytes);
            let l = l.split_once("\n").unwrap_or_default().0;
            let l = l.trim_end();
            let l = l.trim_end_matches('.');

            let lo = lo_va_fu.iter().find(|va| va.name == l);

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
pub fn extend_completion<'a>(
    document: &Document,
    point: &Point,
    vars: &'a Vec<LocaleVariableFunction>,
    imports: &'a Vec<&str>,
    class_map: &'a dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Vec<CompletionItem> {
    if let Some(extend) = current_symbol(document, point, &vars) {
        if let Some(extend_class) = tyres::resolve_var(extend, imports, class_map) {
            return class_unpack(&extend_class);
        }
    }
    vec![]
}

#[cfg(test)]
mod tests {
    use super::get_vars;
    use crate::{
        completion::{current_symbol, extend_completion, LocaleVariableFunction},
        Document,
    };
    use dashmap::DashMap;
    use parser::dto;
    use pretty_assertions::assert_eq;
    use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, CompletionItemLabelDetails};
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
                LocaleVariableFunction {
                    level: 2,
                    ty: "Template".to_owned(),
                    name: "hello".to_owned(),
                    is_fun: false,
                },
                LocaleVariableFunction {
                    level: 2,
                    ty: "Template".to_owned(),
                    name: "se".to_owned(),
                    is_fun: false,
                },
                LocaleVariableFunction {
                    level: 2,
                    ty: "String".to_owned(),
                    name: "other".to_owned(),
                    is_fun: false,
                },
                LocaleVariableFunction {
                    level: 2,
                    ty: "TemplateInstance".to_owned(),
                    name: "hello".to_owned(),
                    is_fun: true,
                },
                LocaleVariableFunction {
                    level: 3,
                    ty: "String".to_owned(),
                    name: "local".to_owned(),
                    is_fun: false,
                },
                LocaleVariableFunction {
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
        let lo_va_fu = vec![LocaleVariableFunction {
            level: 3,
            ty: "String".to_owned(),
            name: "local".to_owned(),
            is_fun: false,
        }];

        let out = current_symbol(&doc, &Point::new(27, 24), &lo_va_fu);
        assert_eq!(
            out,
            Some(&LocaleVariableFunction {
                level: 3,
                ty: "String".to_owned(),
                name: "local".to_owned(),
                is_fun: false,
            })
        );
    }

    #[test]
    fn extend_completion_base() {
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
        var lo = other. 
        return hello.data(\"name\", \"emilycares\");
    }
}
        ";
        let doc = Document::setup(content).unwrap();
        let lo_va_fu = vec![LocaleVariableFunction {
            level: 3,
            ty: "String".to_owned(),
            name: "other".to_owned(),
            is_fun: false,
        }];
        let imports = vec![
            "jakarta.inject.Inject",
            "jakarta.ws.rs.GET",
            "jakarta.ws.rs.Path",
            "jakarta.ws.rs.Produces",
            "jakarta.ws.rs.core.MediaType",
            "io.quarkus.qute.TemplateInstance",
            "io.quarkus.qute.Template",
        ];
        let class_map: DashMap<String, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".to_string(),
            dto::Class {
                class_path: "".to_string(),
                access: vec![dto::Access::Public],
                name: "String".to_string(),
                methods: vec![dto::Method {
                    access: vec![dto::Access::Public],
                    name: "length".to_string(),
                    parameters: vec![],
                    ret: dto::JType::Int,
                }],
                fields: vec![],
            },
        );

        let out = extend_completion(&doc, &Point::new(25, 24), &lo_va_fu, &imports, &class_map);
        assert_eq!(
            out,
            vec![CompletionItem {
                label: "length".to_string(),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some("int ()".to_string()),
                    description: None,
                },),
                kind: Some(CompletionItemKind::FUNCTION),
                ..Default::default()
            }]
        );
    }
}
