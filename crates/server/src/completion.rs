use ast::types::{AstFile, AstPoint};
use call_chain::get_call_chain;
use document::{Document, DocumentError};
use get_class::FoundClass;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionItemLabelDetails, InsertTextFormat};
use my_string::MyString;
use parser::dto::{Access, Class, ImportUnit, JType, Method, Parameter};
use variables::LocalVariable;

use crate::codeaction;

#[derive(Debug)]
pub enum CompletionError {
    Tyres { tyres_error: tyres::TyresError },
    Document(DocumentError),
}

/// Convert list `LocalVariable` to `CompletionItem`
#[must_use]
pub fn complete_vars(vars: &[LocalVariable]) -> Vec<CompletionItem> {
    vars.iter()
        .map(|a| CompletionItem {
            label: a.name.clone(),
            label_details: Some(CompletionItemLabelDetails {
                detail: Some(a.jtype.to_string()),
                ..Default::default()
            }),
            kind: if a.is_fun {
                Some(CompletionItemKind::FUNCTION)
            } else {
                Some(CompletionItemKind::VARIABLE)
            },
            ..Default::default()
        })
        .collect()
}

/// Preview class with the description of methods
#[must_use]
pub fn class_describe(val: &Class, ast: Option<&AstFile>) -> CompletionItem {
    let methods: Vec<_> = val
        .methods
        .iter()
        .map(|m| {
            format!(
                "{}({:?})",
                m.name.as_ref().unwrap_or(&val.name),
                m.parameters
                    .iter()
                    .map(|p| format!("{}", p.jtype))
                    .collect::<Vec<_>>()
            )
        })
        .collect();

    let addi = ast
        .as_ref()
        .map(|ast| codeaction::import_text_edit(&val.class_path, ast));
    let detail = format!("package {};\n{}", val.class_path, methods.join(", "));
    CompletionItem {
        label: val.name.clone(),
        detail: Some(detail),
        kind: Some(CompletionItemKind::CLASS),
        additional_text_edits: addi,
        ..Default::default()
    }
}

/// Unpack class as completion items with methods and fields
#[must_use]
pub fn class_unpack(val: &Class, imports: &[ImportUnit], ast: &AstFile) -> Vec<CompletionItem> {
    let mut out = vec![];

    out.extend(
        val.methods
            .iter()
            .filter(|i| {
                if i.access.is_empty() {
                    return true;
                }
                i.access.contains(Access::Public)
            })
            .filter_map(|i| complete_method(i, imports, ast)),
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
                i.access.contains(Access::Public)
            })
            .map(|f| CompletionItem {
                label: f.name.clone(),
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

fn complete_method(m: &Method, imports: &[ImportUnit], ast: &AstFile) -> Option<CompletionItem> {
    let Some(method_name) = &m.name else {
        return None;
    };
    let params_detail: Vec<String> = m
        .parameters
        .iter()
        .map(|p| {
            p.name.as_ref().map_or_else(
                || p.jtype.to_string(),
                |name| format!("{} {}", p.jtype, name),
            )
        })
        .collect();

    match method_snippet(m) {
        Some(Snippet::Simple(snippet)) => Some(CompletionItem {
            label: method_name.clone(),
            label_details: Some(CompletionItemLabelDetails {
                detail: Some(format!("{} ({})", m.ret, params_detail.join(", "))),
                ..Default::default()
            }),
            kind: Some(CompletionItemKind::FUNCTION),
            insert_text: Some(snippet),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        }),
        Some(Snippet::Import { snippet, import }) => {
            let additional_text_edits = if !imports.contains(&import)
                && let ImportUnit::Class(class_path) = import
            {
                Some(codeaction::import_text_edit(&class_path, ast))
            } else {
                None
            };

            Some(CompletionItem {
                label: method_name.clone(),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some(format!("{} ({})", m.ret, params_detail.join(", "))),
                    ..Default::default()
                }),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some(snippet),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                additional_text_edits,
                ..Default::default()
            })
        }
        None => None,
    }
}

#[derive(PartialEq, Debug)]
enum Snippet {
    Simple(String),
    Import { snippet: String, import: ImportUnit },
}

fn method_snippet(m: &Method) -> Option<Snippet> {
    let Some(method_name) = &m.name else {
        return None;
    };
    let mut import = None;
    let mut params_snippet = String::new();
    let p_len = m.parameters.len();
    let mut i = 1;
    for p in &m.parameters {
        let type_representation = type_to_snippet(&mut import, p);
        params_snippet.push_str(format!("${{{i}:{type_representation}}}").as_str());
        i += 1;
        if i <= p_len {
            params_snippet.push_str(", ");
        }
    }

    let snippet = format!("{method_name}({params_snippet})");
    match import {
        Some(import) => Some(Snippet::Import { snippet, import }),
        None => Some(Snippet::Simple(snippet)),
    }
}

fn type_to_snippet(import: &mut Option<ImportUnit>, p: &Parameter) -> String {
    match &p.jtype {
        JType::Class(c) => match c.as_str() {
            "java.util.stream.Collector" => {
                *import = Some(ImportUnit::Class("java.util.stream.Collectors".into()));
                "Collectors.toList()".to_string()
            }
            "java.util.function.Function" | "java.util.function.Consumer" => "i -> i".to_string(),
            "java.util.function.Predicate" => "i -> true".to_string(),
            "java.util.function.BiFunction" => "(a, b) -> i".to_string(),
            "java.util.function.BiComsumer" => "(i, consumer) -> i".to_string(),
            _ => {
                format!("{}", p.jtype)
            }
        },
        _ => format!("{}", p.jtype),
    }
}

/// Completion of the previous variable
pub fn complete_call_chain(
    document: &Document,
    point: &AstPoint,
    vars: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &dashmap::DashMap<MyString, Class>,
) -> Result<Vec<CompletionItem>, CompletionError> {
    let call_chain = get_call_chain(&document.ast, point);
    match tyres::resolve_call_chain(&call_chain, vars, imports, class, class_map) {
        Ok(resolve_state) => Ok(class_unpack(&resolve_state.class, imports, &document.ast)),
        Err(tyres_error) => Err(CompletionError::Tyres { tyres_error }),
    }
}

#[must_use]
pub fn classes(
    document: &Document,
    point: &AstPoint,
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<MyString, Class>,
) -> Vec<CompletionItem> {
    if point.col < 3 {
        return vec![];
    }
    let mut out = vec![];
    if let Some(FoundClass {
        name: text,
        range: _,
    }) = get_class::get_class(&document.ast, point)
    {
        out.extend(
            imports
                .iter()
                .filter_map(|imp| match imp {
                    ImportUnit::Class(c) | ImportUnit::StaticClass(c) => Some(c),
                    ImportUnit::StaticClassMethod(_, _)
                    | ImportUnit::Prefix(_)
                    | ImportUnit::StaticPrefix(_)
                    | ImportUnit::Package(_) => None,
                })
                .filter(|c| {
                    let Some((_, cname)) = c.rsplit_once('.') else {
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
                .filter(|i| !i.name.contains('&'))
                .filter(|v| {
                    let class_path = v.value().class_path.as_str();
                    !imports::is_imported(imports, class_path)
                })
                .map(|v| class_describe(v.value(), Some(&document.ast)))
                .take(20),
        );
    }
    out
}

#[must_use]
pub fn static_methods(
    ast: &AstFile,
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<MyString, Class>,
) -> Vec<CompletionItem> {
    imports
        .iter()
        .flat_map(|c| match c {
            ImportUnit::Prefix(_)
            | ImportUnit::Package(_)
            | ImportUnit::Class(_)
            | ImportUnit::StaticClass(_) => vec![],
            ImportUnit::StaticClassMethod(c, m) => class_map
                .get(c)
                .into_iter()
                .flat_map(|class| class.methods.clone())
                .filter(|f| f.name.as_ref().filter(|i| *i == m).is_some())
                .collect(),
            ImportUnit::StaticPrefix(c) => class_map
                .get(c)
                .into_iter()
                .flat_map(|class| class.methods.clone())
                .collect(),
        })
        .filter_map(|m| complete_method(&m, imports, ast))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ast::types::{AstPoint, AstRange};
    use dashmap::DashMap;
    use document::Document;
    use lsp_types::{
        CompletionItem, CompletionItemKind, CompletionItemLabelDetails, InsertTextFormat, Position,
        Range, TextEdit,
    };
    use my_string::MyString;
    use parser::dto::{Access, Class, ImportUnit, JType, Method, Parameter};
    use pretty_assertions::assert_eq;
    use variables::LocalVariable;

    use crate::completion::{Snippet, classes, complete_call_chain};

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
        let doc = Document::setup(content, PathBuf::new()).unwrap().0;
        let class = Class {
            access: Access::Public,
            name: "Test".into(),
            ..Default::default()
        };
        let lo_va = vec![LocalVariable {
            level: 3,
            jtype: JType::Class("String".into()),
            name: "other".into(),
            is_fun: false,
            range: AstRange::default(),
        }];
        let imports = vec![
            ImportUnit::Class("jakarta.inject.Inject".into()),
            ImportUnit::Class("jakarta.ws.rs.GET".into()),
            ImportUnit::Class("jakarta.ws.rs.Path".into()),
            ImportUnit::Class("jakarta.ws.rs.Produces".into()),
            ImportUnit::Class("jakarta.ws.rs.core.MediaType".into()),
            ImportUnit::Class("io.quarkus.qute.TemplateInstance".into()),
            ImportUnit::Class("io.quarkus.qute.Template".into()),
        ];
        let class_map: DashMap<MyString, Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".into(),
            Class {
                access: Access::Public,
                imports: imports.clone(),
                name: "String".into(),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some("length".into()),
                    ret: JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );

        let out = complete_call_chain(
            &doc,
            &AstPoint::new(25, 24),
            &lo_va,
            &imports,
            &class,
            &class_map,
        );
        assert_eq!(
            out.unwrap(),
            vec![CompletionItem {
                label: "length".into(),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some("int ()".into()),
                    description: None,
                },),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some("length()".into()),
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
        let doc = Document::setup(SYMBOL_METHOD, PathBuf::new()).unwrap().0;
        let lo_va = vec![LocalVariable {
            level: 3,
            jtype: JType::Class("String".into()),
            name: "local".into(),
            is_fun: false,
            range: AstRange::default(),
        }];
        let imports = vec![];
        let class = Class {
            access: Access::Public,
            name: "Test".into(),
            ..Default::default()
        };
        let class_map: DashMap<MyString, Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".into(),
            Class {
                access: Access::Public,
                name: "String".into(),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some("concat".into()),
                    ret: JType::Class("java.lang.String".into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );

        let out = complete_call_chain(
            &doc,
            &AstPoint::new(8, 40),
            &lo_va,
            &imports,
            &class,
            &class_map,
        );
        assert_eq!(
            out.unwrap(),
            vec![CompletionItem {
                label: "concat".into(),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some("String ()".into()),
                    description: None,
                },),
                kind: Some(CompletionItemKind::FUNCTION),
                insert_text: Some("concat()".into()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            }]
        );
    }

    #[test]
    fn method_snippet_no_param() {
        let method = Method {
            access: Access::Public,
            name: Some("length".into()),
            parameters: vec![],
            ret: JType::Int,
            throws: vec![],
            source: None,
        };
        let out = method_snippet(&method);
        assert_eq!(out, Some(Snippet::Simple("length()".into())));
    }

    #[test]
    fn method_snippet_base() {
        let method = Method {
            access: Access::Public,
            name: Some("compute".into()),
            parameters: vec![Parameter {
                name: None,
                jtype: JType::Int,
            }],
            ret: JType::Int,
            throws: vec![],
            source: None,
        };
        let out = method_snippet(&method);
        assert_eq!(out, Some(Snippet::Simple("compute(${1:int})".into())));
    }

    #[test]
    fn method_snippet_args() {
        let method = Method {
            access: Access::Public,
            name: Some("split".into()),
            parameters: vec![
                Parameter {
                    name: None,
                    jtype: JType::Class("java.lang.String".into()),
                },
                Parameter {
                    name: None,
                    jtype: JType::Int,
                },
            ],
            ret: JType::Int,
            throws: vec![],
            source: None,
        };
        let out = method_snippet(&method);
        assert_eq!(
            Some(Snippet::Simple("split(${1:String}, ${2:int})".into())),
            out
        );
    }

    #[ignore = "todo"]
    #[test]
    fn class_completion_base() {
        let class_map: DashMap<MyString, Class> = DashMap::new();
        class_map.insert(
            "java.lang.StringBuilder".into(),
            Class {
                class_path: "java.lang.StringBuilder".into(),
                access: Access::Public,
                name: "StringBuilder".into(),
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
        let doc = Document::setup(content, PathBuf::new()).unwrap().0;

        let out = classes(&doc, &AstPoint::new(5, 16), &[], &class_map);
        assert_eq!(
            out,
            vec![CompletionItem {
                label: "StringBuilder".into(),
                detail: Some("package java.lang.StringBuilder;\n".into()),
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
                    new_text: "\nimport java.lang.StringBuilder;".into(),
                },],),
                ..Default::default()
            }]
        );
    }

    #[ignore = "todo"]
    #[test]
    fn class_completion_imported() {
        let class_map: DashMap<MyString, Class> = DashMap::new();
        class_map.insert(
            "java.lang.StringBuilder".into(),
            Class {
                class_path: "java.lang.StringBuilder".into(),
                access: Access::Public,
                name: "StringBuilder".into(),
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
        let doc = Document::setup(content, PathBuf::new()).unwrap().0;

        let out = classes(
            &doc,
            &AstPoint::new(6, 16),
            &[ImportUnit::Class("java.lang.StringBuilder".into())],
            &class_map,
        );
        assert_eq!(
            out,
            vec![CompletionItem {
                label: "StringBuilder".into(),
                detail: Some("package java.lang.StringBuilder;\n".into()),
                kind: Some(CompletionItemKind::CLASS),
                ..Default::default()
            }]
        );
    }
}
