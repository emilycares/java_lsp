use ast::types::{AstFile, AstImportUnit, AstPoint};
use call_chain::get_call_chain;
use document::{Document, DocumentError};
use lsp_types::{CompletionItem, CompletionItemKind, CompletionItemLabelDetails, InsertTextFormat};
use parser::dto::{self, ImportUnit};
use smol_str::SmolStr;
use variables::LocalVariable;

use crate::codeaction;

#[derive(Debug)]
pub enum CompletionError {
    Tyres { tyres_error: tyres::TyresError },
    Treesitter(DocumentError),
}

/// Convert list LocalVariable to CompletionItem
pub fn complete_vars(vars: &[LocalVariable]) -> Vec<CompletionItem> {
    vars.iter()
        .map(|a| CompletionItem {
            label: a.name.to_string(),
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
pub fn class_describe(val: &dto::Class, ast: Option<&AstFile>) -> CompletionItem {
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

    if let Some(ast) = ast {
        addi = Some(codeaction::import_text_edit(&val.class_path, ast));
    }

    let detail = format!("package {};\n{}", val.class_path, methods.join(", "));
    CompletionItem {
        label: val.name.to_string(),
        detail: Some(detail),
        kind: Some(CompletionItemKind::CLASS),
        additional_text_edits: addi,
        ..Default::default()
    }
}

/// Unpack class as completion items with methods and fields
pub fn class_unpack(
    val: &dto::Class,
    imports: &[ImportUnit],
    ast: &AstFile,
) -> Vec<CompletionItem> {
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
            .map(|i| complete_method(i, imports, ast)),
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
                label: f.name.to_string(),
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

fn complete_method(m: &dto::Method, imports: &[ImportUnit], ast: &AstFile) -> CompletionItem {
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
            label: m.name.to_string(),
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
            if !imports.contains(&import)
                && let ImportUnit::Class(class_path) = import
            {
                additional_text_edits = Some(codeaction::import_text_edit(&class_path, ast));
            };

            CompletionItem {
                label: m.name.to_string(),
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
        let type_representation = type_to_snippet(&mut import, p);
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

fn type_to_snippet(import: &mut Option<ImportUnit>, p: &dto::Parameter) -> String {
    match &p.jtype {
        dto::JType::Class(c) => match c.as_str() {
            "java.util.stream.Collector" => {
                *import = Some(ImportUnit::Class("java.util.stream.Collectors".into()));
                "Collectors.toList()".to_string()
            }
            "java.util.function.Function" => "i -> i".to_string(),
            "java.util.function.Consumer" => "i -> i".to_string(),
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
    class: &dto::Class,
    class_map: &dashmap::DashMap<SmolStr, parser::dto::Class>,
) -> Result<Vec<CompletionItem>, CompletionError> {
    let call_chain = get_call_chain(&document.ast, point);
    return match tyres::resolve_call_chain(&call_chain, vars, imports, class, class_map) {
        Ok(resolve_state) => Ok(class_unpack(&resolve_state.class, imports, &document.ast)),
        Err(tyres_error) => Err(CompletionError::Tyres { tyres_error }),
    };
}

pub fn classes(
    document: &Document,
    point: &AstPoint,
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<SmolStr, parser::dto::Class>,
) -> Vec<CompletionItem> {
    if point.col < 3 {
        return vec![];
    }
    let mut out = vec![];
    let text = get_class(&document.ast, point);

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
            .map(|v| class_describe(v.value(), Some(&document.ast)))
            .take(20),
    );
    out
}

fn get_class(ast: &AstFile, point: &AstPoint) -> String {
    let out = String::new();

    match &ast.thing {
        ast::types::AstThing::Class(ast_class) => {
            for v in &ast_class.block.variables {
                if !v.range.is_in_range(point) {
                    continue;
                }
            }
        }
        ast::types::AstThing::Interface(_ast_interface) => (),
        ast::types::AstThing::Enumeration(_ast_enumeration) => (),
        ast::types::AstThing::Annotation(_ast_annotation) => (),
    }
    out
}

pub fn static_methods(
    ast: &AstFile,
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<SmolStr, parser::dto::Class>,
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
        .map(|m| complete_method(&m, imports, ast))
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
    use parser::dto::{self, ImportUnit};
    use pretty_assertions::assert_eq;
    use smol_str::SmolStr;
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
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();
        let class = dto::Class {
            access: vec![dto::Access::Public],
            name: "Test".into(),
            ..Default::default()
        };
        let lo_va = vec![LocalVariable {
            level: 3,
            jtype: dto::JType::Class("String".into()),
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
        let class_map: DashMap<SmolStr, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".into(),
            dto::Class {
                access: vec![dto::Access::Public],
                imports: imports.clone(),
                name: "String".into(),
                methods: vec![dto::Method {
                    access: vec![dto::Access::Public],
                    name: "length".into(),
                    ret: dto::JType::Int,
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
        let doc = Document::setup(SYMBOL_METHOD, PathBuf::new(), "".into()).unwrap();
        let lo_va = vec![LocalVariable {
            level: 3,
            jtype: dto::JType::Class("String".into()),
            name: "local".into(),
            is_fun: false,
            range: AstRange::default(),
        }];
        let imports = vec![];
        let class = dto::Class {
            access: vec![dto::Access::Public],
            name: "Test".into(),
            ..Default::default()
        };
        let class_map: DashMap<SmolStr, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".into(),
            dto::Class {
                access: vec![dto::Access::Public],
                name: "String".into(),
                methods: vec![dto::Method {
                    access: vec![dto::Access::Public],
                    name: "concat".into(),
                    ret: dto::JType::Class("java.lang.String".into()),
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
        let method = dto::Method {
            access: vec![dto::Access::Public],
            name: "length".into(),
            parameters: vec![],
            ret: dto::JType::Int,
            throws: vec![],
            source: None,
        };
        let out = method_snippet(&method);
        assert_eq!(out, Snippet::Simple("length()".into()));
    }

    #[test]
    fn method_snippet_base() {
        let method = dto::Method {
            access: vec![dto::Access::Public],
            name: "compute".into(),
            parameters: vec![dto::Parameter {
                name: None,
                jtype: dto::JType::Int,
            }],
            ret: dto::JType::Int,
            throws: vec![],
            source: None,
        };
        let out = method_snippet(&method);
        assert_eq!(out, Snippet::Simple("compute(${1:int})".into()));
    }

    #[test]
    fn method_snippet_args() {
        let method = dto::Method {
            access: vec![dto::Access::Public],
            name: "split".into(),
            parameters: vec![
                dto::Parameter {
                    name: None,
                    jtype: dto::JType::Class("java.lang.String".into()),
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
        assert_eq!(Snippet::Simple("split(${1:String}, ${2:int})".into()), out,);
    }

    #[test]
    fn class_completion_base() {
        let class_map: DashMap<SmolStr, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.StringBuilder".into(),
            dto::Class {
                class_path: "java.lang.StringBuilder".into(),
                access: vec![dto::Access::Public],
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
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();

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

    #[test]
    fn class_completion_imported() {
        let class_map: DashMap<SmolStr, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.StringBuilder".into(),
            dto::Class {
                class_path: "java.lang.StringBuilder".into(),
                access: vec![dto::Access::Public],
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
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();

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
