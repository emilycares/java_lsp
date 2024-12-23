use parser::dto;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};
use tree_sitter::Point;

use crate::{tyres, utils::to_lsp_range, Document};

pub fn class(
    document: &Document,
    point: &Point,
    imports: &[&str],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<Hover> {
    let tree = &document.tree;
    let bytes = document
        .text
        .slice(..)
        .as_str()
        .unwrap_or_default()
        .as_bytes();

    if let Ok(n) = tree_sitter_util::get_node_at_point(tree, *point) {
        if n.kind() == "type_identifier" {
            if let Ok(jtype) = n.utf8_text(bytes) {
                if let Some(class) = tyres::resolve(jtype, imports, class_map) {
                    return class_to_hover(class, to_lsp_range(n.range()));
                }
            }
        }

        'annotation: {
            if n.kind() == "identifier" {
                let Some(n) = n.parent() else {
                    break 'annotation;
                };
                if n.kind() == "annotation" || n.kind() == "marker_annotation" {
                    if let Ok(jtype) = n.utf8_text(bytes) {
                        if let Some(class) = tyres::resolve(jtype, imports, class_map) {
                            return class_to_hover(class, to_lsp_range(n.range()));
                        }
                    }
                }
            }
        }
    }

    None
}

fn class_to_hover(class: dto::Class, range: Range) -> Option<Hover> {
    let methods: Vec<_> = class
        .methods
        .iter()
        .map(|m| {
            format!(
                " - {}({:?})",
                m.name,
                m.parameters
                    .iter()
                    .map(|p| format!("{}", p.jtype))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect();
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("# {}\n\nmethods: {}", class.name, methods.join("\n")),
        }),
        range: Some(range),
    })
}
