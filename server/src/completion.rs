use call_chain::get_call_chain;
use document::DocumentError;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionItemLabelDetails, InsertTextFormat};
use parser::dto::{self, ImportUnit};
use tree_sitter::{Point, Tree};
use tree_sitter_util::{get_node_at_point, get_string_node};
use variables::LocalVariable;

use crate::{codeaction, Document};

#[derive(Debug)]
pub enum CompletionError {
    Tyres { tyres_error: tyres::TyresError },
    Treesitter(DocumentError),
}

/// Convert list LocalVariable to CompletionItem
pub fn complete_vars(vars: &[LocalVariable]) -> Vec<CompletionItem> {
    vars.iter()
        .map(|a| CompletionItem {
            label: a.name.to_owned(),
            label_details: Some(CompletionItemLabelDetails {
                detail: Some(a.jtype.to_string()),
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

/// Preview class with the description of methods
pub fn class_describe(val: &dto::Class, add_import_tree: Option<&Tree>) -> CompletionItem {
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

    let mut addi = None;

    if let Some(tree) = add_import_tree {
        addi = Some(codeaction::import_text_edit(&val.class_path, tree));
    }

    CompletionItem {
        label: val.name.to_string(),
        detail: Some(methods.join(", ")),
        kind: Some(CompletionItemKind::CLASS),
        additional_text_edits: addi,
        ..Default::default()
    }
}

/// Unpack class as completion items with methods and fields
pub fn class_unpack(val: &dto::Class, imports: &[ImportUnit], tree: &Tree) -> Vec<CompletionItem> {
    let mut out = vec![];

    out.extend(
        val.methods
            .iter()
            .filter(|i| {
                if i.access.is_empty() {
                    return true;
                }
                i.access.contains(&parser::dto::Access::Public)
            })
            .map(|i| complete_method(i, imports, tree)),
    );

    out.extend(
        val.fields
            .iter()
            // TODO: Create trait to check access boundary
            // #To check:
            //  - access empty
            //  - has public
            //  - not private
            //  - protected
            // .filter(|i| i.hasAccess(current_class_with_package))
            .filter(|i| {
                if i.access.is_empty() {
                    return true;
                }
                i.access.contains(&parser::dto::Access::Public)
            })
            .map(|f| CompletionItem {
                label: f.name.to_owned(),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some(f.jtype.to_string()),
                    ..Default::default()
                }),
                kind: Some(CompletionItemKind::FIELD),
                ..Default::default()
            }),
    );

    out.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));

    out
}

fn complete_method(m: &dto::Method, imports: &[ImportUnit], tree: &Tree) -> CompletionItem {
    let params_detail: Vec<String> = m
        .parameters
        .iter()
        .map(|p| match &p.name {
            Some(name) => format!("{} {}", p.jtype, name),
            None => p.jtype.to_string(),
        })
        .collect();

    match method_snippet(m) {
        Snippet::Simple(snippet) => CompletionItem {
            label: m.name.to_owned(),
            label_details: Some(CompletionItemLabelDetails {
                detail: Some(format!("{} ({})", m.ret, params_detail.join(", "))),
                ..Default::default()
            }),
            kind: Some(CompletionItemKind::FUNCTION),
            insert_text: Some(snippet),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        Snippet::Import { snippet, import } => {
            let mut additional_text_edits = None;
            if !imports.contains(&import) {
                if let ImportUnit::Class(class_path) = import {
                    additional_text_edits = Some(codeaction::import_text_edit(&class_path, tree));
                };
            }

            CompletionItem {
                label: m.name.to_owned(),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some(format!("{} ({})", m.ret, params_detail.join(", "))),
                    ..Default::default()
                }),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some(snippet),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                additional_text_edits,
                ..Default::default()
            }
        }
    }
}

#[derive(PartialEq, Debug)]
enum Snippet {
    Simple(String),
    Import { snippet: String, import: ImportUnit },
}

fn method_snippet(m: &dto::Method) -> Snippet {
    let mut import = None;
    let mut params_snippet = String::new();
    let p_len = m.parameters.len();
    let mut i = 1;
    for p in &m.parameters {
        let type_representation = match &p.jtype {
            dto::JType::Class(c) => match c.as_str() {
                "java.util.stream.Collector" => {
                    import = Some(ImportUnit::Class("java.util.stream.Collectors".to_string()));
                    "Collectors.toList()".to_string()
                }
                _ => {
                    format!("{}", p.jtype)
                }
            },
            _ => format!("{}", p.jtype),
        };
        params_snippet.push_str(format!("${{{}:{}}}", i, type_representation).as_str());
        i += 1;
        if i <= p_len {
            params_snippet.push_str(", ");
        }
    }

    let snippet = format!("{}({})", m.name, params_snippet);
    match import {
        Some(import) => Snippet::Import { snippet, import },
        None => Snippet::Simple(snippet),
    }
}

/// Completion of the previous variable
pub fn complete_call_chain(
    document: &Document,
    point: &Point,
    vars: &[LocalVariable],
    imports: &[ImportUnit],
    class: &dto::Class,
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Result<Vec<CompletionItem>, CompletionError> {
    if let Some(call_chain) = get_call_chain(&document.tree, document.as_bytes(), point).as_deref()
    {
        return match tyres::resolve_call_chain(call_chain, vars, imports, class, class_map) {
            Ok(resolve_state) => Ok(class_unpack(&resolve_state.class, imports, &document.tree)),
            Err(tyres_error) => Err(CompletionError::Tyres { tyres_error }),
        };
    }
    Ok(vec![])
}

pub fn classes(
    document: &Document,
    point: &Point,
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Vec<CompletionItem> {
    let tree = &document.tree;

    if point.column < 3 {
        return vec![];
    }

    let Ok(node) = get_node_at_point(tree, Point::new(point.row, point.column - 2)) else {
        return vec![];
    };

    let bytes = document.as_bytes();

    let mut out = vec![];

    if let Some(text) = is_class_completion(node, bytes) {
        out.extend(
            imports
                .iter()
                .filter_map(|imp| match imp {
                    ImportUnit::Class(c) => Some(c),
                    ImportUnit::StaticClass(c) => Some(c),
                    ImportUnit::StaticClassMethod(_, _) => None,
                    ImportUnit::Prefix(_) => None,
                    ImportUnit::StaticPrefix(_) => None,
                    ImportUnit::Package(_) => None,
                })
                .filter(|c| {
                    let Some((_, cname)) = c.rsplit_once(".") else {
                        return false;
                    };
                    cname.starts_with(&text)
                })
                .filter_map(|class_path| class_map.get(class_path))
                .map(|c| class_describe(&c, None)),
        );
        out.extend(
            class_map
                .iter()
                .filter(|c| c.name.starts_with(&text))
                .filter(|i| !i.name.contains("&"))
                .filter(|v| {
                    let class_path = v.value().class_path.as_str();
                    !imports::is_imported(imports, class_path)
                })
                .map(|v| class_describe(v.value(), Some(&document.tree)))
                .take(20),
        );
    }
    out
}

fn is_class_completion(node: tree_sitter::Node<'_>, bytes: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "type_identifier" => {
            let text = get_string_node(&node, bytes);
            if let Some(c) = text.chars().next() {
                if c.is_uppercase() {
                    return Some(text);
                }
            }
            None
        }
        _ => None,
    }
}

pub fn static_methods(
    imports: &[ImportUnit],
    tree: &Tree,
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Vec<CompletionItem> {
    imports
        .iter()
        .flat_map(|c| match c {
            ImportUnit::Class(_) => vec![],
            ImportUnit::StaticClass(_) => vec![],
            ImportUnit::StaticClassMethod(c, m) => class_map
                .get(c)
                .into_iter()
                .flat_map(|class| class.methods.clone())
                .filter(|f| f.name == *m)
                .collect(),
            ImportUnit::Prefix(_) => vec![],
            ImportUnit::StaticPrefix(c) => class_map
                .get(c)
                .into_iter()
                .flat_map(|class| class.methods.clone())
                .collect(),
            ImportUnit::Package(_) => vec![],
        })
        .map(|m| complete_method(&m, imports, tree))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use dashmap::DashMap;
    use lsp_types::{
        CompletionItem, CompletionItemKind, CompletionItemLabelDetails, InsertTextFormat, Position,
        Range, TextEdit,
    };
    use parser::dto::{self, ImportUnit};
    use pretty_assertions::assert_eq;
    use tree_sitter::Point;
    use variables::LocalVariable;

    use crate::{
        completion::{classes, complete_call_chain, Snippet},
        Document,
    };

    use super::method_snippet;

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
        var lo = other. ;
        return hello.data(\"name\", \"emilycares\");
    }
}
        ";
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();
        let class = dto::Class {
            access: vec![dto::Access::Public],
            name: "Test".to_string(),
            ..Default::default()
        };
        let lo_va = vec![LocalVariable {
            level: 3,
            jtype: dto::JType::Class("String".to_owned()),
            name: "other".to_owned(),
            is_fun: false,
            range: tree_sitter::Range {
                start_byte: 0,
                end_byte: 0,
                start_point: Point { row: 0, column: 0 },
                end_point: Point { row: 0, column: 0 },
            },
        }];
        let imports = vec![
            ImportUnit::Class("jakarta.inject.Inject".to_string()),
            ImportUnit::Class("jakarta.ws.rs.GET".to_string()),
            ImportUnit::Class("jakarta.ws.rs.Path".to_string()),
            ImportUnit::Class("jakarta.ws.rs.Produces".to_string()),
            ImportUnit::Class("jakarta.ws.rs.core.MediaType".to_string()),
            ImportUnit::Class("io.quarkus.qute.TemplateInstance".to_string()),
            ImportUnit::Class("io.quarkus.qute.Template".to_string()),
        ];
        let class_map: DashMap<String, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".to_string(),
            dto::Class {
                access: vec![dto::Access::Public],
                imports: imports.clone(),
                name: "String".to_string(),
                methods: vec![dto::Method {
                    access: vec![dto::Access::Public],
                    name: "length".to_string(),
                    ret: dto::JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );

        let out = complete_call_chain(
            &doc,
            &Point::new(25, 24),
            &lo_va,
            &imports,
            &class,
            &class_map,
        );
        assert_eq!(
            out.unwrap(),
            vec![CompletionItem {
                label: "length".to_string(),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some("int ()".to_string()),
                    description: None,
                },),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some("length()".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            }]
        );
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
    fn extend_completion_method() {
        let doc = Document::setup(SYMBOL_METHOD, PathBuf::new(), "".to_string()).unwrap();
        let lo_va = vec![LocalVariable {
            level: 3,
            jtype: dto::JType::Class("String".to_owned()),
            name: "local".to_owned(),
            is_fun: false,
            range: tree_sitter::Range {
                start_byte: 0,
                end_byte: 0,
                start_point: Point { row: 0, column: 0 },
                end_point: Point { row: 0, column: 0 },
            },
        }];
        let imports = vec![];
        let class = dto::Class {
            access: vec![dto::Access::Public],
            name: "Test".to_string(),
            ..Default::default()
        };
        let class_map: DashMap<String, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".to_string(),
            dto::Class {
                access: vec![dto::Access::Public],
                name: "String".to_string(),
                methods: vec![dto::Method {
                    access: vec![dto::Access::Public],
                    name: "concat".to_string(),
                    ret: dto::JType::Class("java.lang.String".to_string()),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );

        let out = complete_call_chain(
            &doc,
            &Point::new(8, 40),
            &lo_va,
            &imports,
            &class,
            &class_map,
        );
        assert_eq!(
            out.unwrap(),
            vec![CompletionItem {
                label: "concat".to_string(),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some("String ()".to_string()),
                    description: None,
                },),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some("concat()".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            }]
        );
    }

    #[test]
    fn method_snippet_no_param() {
        let method = dto::Method {
            access: vec![dto::Access::Public],
            name: "length".to_string(),
            parameters: vec![],
            ret: dto::JType::Int,
            throws: vec![],
            source: None,
        };
        let out = method_snippet(&method);
        assert_eq!(out, Snippet::Simple("length()".to_string()));
    }

    #[test]
    fn method_snippet_base() {
        let method = dto::Method {
            access: vec![dto::Access::Public],
            name: "compute".to_string(),
            parameters: vec![dto::Parameter {
                name: None,
                jtype: dto::JType::Int,
            }],
            ret: dto::JType::Int,
            throws: vec![],
            source: None,
        };
        let out = method_snippet(&method);
        assert_eq!(out, Snippet::Simple("compute(${1:int})".to_string()));
    }

    #[test]
    fn method_snippet_args() {
        let method = dto::Method {
            access: vec![dto::Access::Public],
            name: "split".to_string(),
            parameters: vec![
                dto::Parameter {
                    name: None,
                    jtype: dto::JType::Class("java.lang.String".to_string()),
                },
                dto::Parameter {
                    name: None,
                    jtype: dto::JType::Int,
                },
            ],
            ret: dto::JType::Int,
            throws: vec![],
            source: None,
        };
        let out = method_snippet(&method);
        assert_eq!(
            Snippet::Simple("split(${1:String}, ${2:int})".to_string()),
            out,
        );
    }

    #[test]
    fn class_completion_base() {
        let class_map: DashMap<String, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.StringBuilder".to_string(),
            dto::Class {
                class_path: "java.lang.StringBuilder".to_string(),
                access: vec![dto::Access::Public],
                name: "StringBuilder".to_string(),
                ..Default::default()
            },
        );
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        String local = other.toString();
        StringB 

        return;
    }
}
";
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();

        let out = classes(&doc, &Point::new(5, 16), &[], &class_map);
        assert_eq!(
            out,
            vec![CompletionItem {
                label: "StringBuilder".to_string(),
                detail: Some("".to_string()),
                kind: Some(CompletionItemKind::CLASS),
                additional_text_edits: Some(vec![TextEdit {
                    range: Range {
                        start: Position {
                            line: 2,
                            character: 0,
                        },
                        end: Position {
                            line: 2,
                            character: 0,
                        },
                    },
                    new_text: "\nimport java.lang.StringBuilder;".to_string(),
                },],),
                ..Default::default()
            }]
        );
    }

    #[test]
    fn class_completion_imported() {
        let class_map: DashMap<String, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.StringBuilder".to_string(),
            dto::Class {
                class_path: "java.lang.StringBuilder".to_string(),
                access: vec![dto::Access::Public],
                name: "StringBuilder".to_string(),
                ..Default::default()
            },
        );
        let content = "
package ch.emilycares;
import java.lang.StringBuilder;
public class Test {
    public void hello() {
        String local = other.toString();
        StringB 

        return;
    }
}
";
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();

        let out = classes(
            &doc,
            &Point::new(6, 16),
            &[ImportUnit::Class("java.lang.StringBuilder".to_string())],
            &class_map,
        );
        assert_eq!(
            out,
            vec![CompletionItem {
                label: "StringBuilder".to_string(),
                detail: Some("".to_string()),
                kind: Some(CompletionItemKind::CLASS),
                ..Default::default()
            }]
        );
    }
}
