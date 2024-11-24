use parser::dto;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, CompletionItemLabelDetails};
use tree_sitter::Point;

use crate::{tyres, variable::{current_symbol, LocalVariable}, Document};

/// Convert list LocalVariable to CompletionItem
pub fn complete_vars(vars: &Vec<LocalVariable>) -> Vec<CompletionItem> {
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


/// Preview class with the description of methods
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

/// Unpack class as completion items with methods and fields
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


/// Completion of the previous variable
pub fn extend_completion<'a>(
    document: &Document,
    point: &Point,
    vars: &'a Vec<LocalVariable>,
    imports: &'a Vec<&str>,
    class_map: &'a dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Vec<CompletionItem> {
    if let Some(extend) = current_symbol(document, point, &vars) {
        if let Some(extend_class) = tyres::resolve_var(extend, imports, class_map) {
            return class_unpack(&extend_class);
        } else {
            dbg!("unable to resolve var", extend);
        }
    } else {
        dbg!("did not finnd a current_symbol");
    }
    vec![]
}

#[cfg(test)]
mod tests {
    use crate::{
        completion::{extend_completion, LocalVariable},
        Document,
    };
    use dashmap::DashMap;
    use parser::dto;
    use pretty_assertions::assert_eq;
    use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, CompletionItemLabelDetails};
    use tree_sitter::Point;



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
        let lo_va = vec![LocalVariable {
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

        let out = extend_completion(&doc, &Point::new(25, 24), &lo_va, &imports, &class_map);
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

