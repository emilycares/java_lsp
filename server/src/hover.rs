use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};
use parser::dto;
use tree_sitter::{Point, Tree};

use crate::{
    call_chain::{class_or_variable, CallItem},
    imports::ImportUnit,
    tyres,
    utils::to_lsp_range,
    Document,
};

pub fn class(
    document: &Document,
    point: &Point,
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<Hover> {
    let tree = &document.tree;
    let bytes = document.as_bytes();

    if let Some((class, range)) = class_action(tree, bytes, point, imports, class_map) {
        return class_to_hover(class, range);
    }

    None
}

pub fn class_action(
    tree: &Tree,
    bytes: &[u8],
    point: &Point,
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<(dto::Class, Range)> {
    if let Ok(n) = tree_sitter_util::get_node_at_point(tree, *point) {
        if n.kind() == "type_identifier" {
            if let Ok(jtype) = n.utf8_text(bytes) {
                if let Some(class) = tyres::resolve(jtype, imports, class_map) {
                    return Some((class, to_lsp_range(n.range())));
                }
            }
        }
        if n.kind() == "identifier" {
            if let Ok(text) = n.utf8_text(bytes) {
                if let Some(CallItem::Class(class)) = class_or_variable(text.to_string()) {
                    if let Some(class) = tyres::resolve(&class, imports, class_map) {
                        return Some((class, to_lsp_range(n.range())));
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
            value: format!("# {}\n\nmethods:\n{}", class.name, methods.join("\n")),
        }),
        range: Some(range),
    })
}

#[cfg(test)]
mod tests {
    use dashmap::DashMap;
    use parser::dto;
    use tree_sitter::Point;

    use crate::{hover::class_action, Document};

    #[test]
    fn class_action_base() {
        let content = "
package ch.emilycares;
public class Test {
    public String hello() {
        return;
    }
}
";
        let bytes = content.as_bytes();
        let doc = Document::setup(content).unwrap();

        let out = class_action(
            &doc.tree,
            bytes,
            &Point::new(3, 14),
            &[],
            &string_class_map(),
        );
        assert!(out.is_some());
    }
    #[test]

    fn class_action_marker_annotation() {
        let content = "
package ch.emilycares;
public class Test {
    @String
    public void hello() {
        return;
    }
}
";
        let bytes = content.as_bytes();
        let doc = Document::setup(content).unwrap();

        let out = class_action(
            &doc.tree,
            bytes,
            &Point::new(3, 9),
            &[],
            &string_class_map(),
        );
        assert!(out.is_some());
    }

    fn string_class_map() -> DashMap<String, dto::Class> {
        let class_map: DashMap<String, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".to_string(),
            dto::Class {
                class_path: "".to_string(),
                source: "".to_string(),
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
        class_map
    }
}
