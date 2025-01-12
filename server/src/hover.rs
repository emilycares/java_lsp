use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};
use parser::dto;
use tree_sitter::Point;

use crate::{
    call_chain::{self, class_or_variable, CallItem},
    imports::ImportUnit,
    tyres,
    utils::to_lsp_range,
    variable::LocalVariable,
    Document,
};

pub fn base(
    document: &Document,
    point: &Point,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<Hover> {
    if let Some((class, range)) = class_action(document, point, imports, class_map) {
        return Some(class_to_hover(class, range));
    }

    if let Some(hover) = call_chain_hover(document, point, lo_va, imports, class_map) {
        return Some(hover);
    }

    None
}

pub fn class_action(
    document: &Document,
    point: &Point,
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<(dto::Class, Range)> {
    let tree = &document.tree;
    let bytes = document.as_bytes();
    if let Ok(n) = tree_sitter_util::get_node_at_point(tree, *point) {
        if n.kind() == "type_identifier" {
            if let Ok(jtype) = n.utf8_text(bytes) {
                if let Some(class) = tyres::resolve(jtype, imports, class_map) {
                    return Some((class, to_lsp_range(n.range())));
                }
            }
        }
        if n.kind() == "identifier" {
            if let Some(CallItem::Class { name: class, range }) = class_or_variable(n, bytes) {
                if let Some(class) = tyres::resolve(&class, imports, class_map) {
                    return Some((class, to_lsp_range(range)));
                }
            }
        }
    }
    None
}

pub fn call_chain_hover(
    document: &Document,
    point: &Point,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<Hover> {
    let Some(call_chain) = call_chain::get_call_chain(document, point) else {
        return None;
    };

    let Some((item, relevat)) = call_chain::validate(&call_chain, point) else {
        return None;
    };
    let Some(el) = call_chain.get(item) else {
        return None;
    };
    let class = tyres::resolve_call_chain(relevat, lo_va, imports, class_map)?;
    match el {
        CallItem::MethodCall { name, range } => {
            let Some(method) = class.methods.iter().find(|m| m.name == *name) else {
                return None;
            };
            Some(method_to_hover(&method, to_lsp_range(*range)))
        }
        CallItem::FieldAccess { name, range } => {
            let Some(method) = class.fields.iter().find(|m| m.name == *name) else {
                return None;
            };
            Some(field_to_hover(&method, to_lsp_range(*range)))
        }
        CallItem::Variable { name, range } => {
            let Some(var) = lo_va.iter().find(|v| v.name == *name) else {
                return None;
            };
            Some(variable_to_hover(var, to_lsp_range(*range)))
        }
        CallItem::Class { name: _, range } => Some(class_to_hover(class, to_lsp_range(*range))),
    }
}

fn format_method(m: &dto::Method) -> String {
    format!(
        "{} {}({:?})",
        m.ret,
        m.name,
        m.parameters
            .iter()
            .map(|p| format!("{}", p.jtype))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn variable_to_hover(var: &LocalVariable, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("{} {}", var.jtype, var.name),
        }),
        range: Some(range),
    }
}

fn field_to_hover(f: &dto::Field, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("{} {}", f.jtype, f.name),
        }),
        range: Some(range),
    }
}

fn method_to_hover(m: &dto::Method, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format_method(&m),
        }),
        range: Some(range),
    }
}

fn class_to_hover(class: dto::Class, range: Range) -> Hover {
    let methods: Vec<_> = class.methods.iter().map(format_method).collect();
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("# {}\n\nmethods:\n{}", class.name, methods.join("\n")),
        }),
        range: Some(range),
    }
}

#[cfg(test)]
mod tests {
    use dashmap::DashMap;
    use parser::dto;
    use tree_sitter::Point;

    use crate::{
        hover::{call_chain_hover, class_action},
        variable, Document,
    };

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
        let doc = Document::setup(content).unwrap();

        let out = class_action(&doc, &Point::new(3, 14), &[], &string_class_map());
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
        let doc = Document::setup(content).unwrap();

        let out = class_action(&doc, &Point::new(3, 9), &[], &string_class_map());
        assert!(out.is_some());
    }

    #[test]
    fn method_hover() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
    String other = \"asd\";
    String local = other.length().toString();
    }
}
";
        let doc = Document::setup(content).unwrap();
        let point = Point::new(5, 29);
        let vars = variable::get_vars(&doc, &point);

        let out = call_chain_hover(&doc, &point, &vars, &[], &string_class_map());
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
