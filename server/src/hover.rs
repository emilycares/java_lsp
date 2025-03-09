use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};
use parser::{
    call_chain::{self, class_or_variable, CallItem},
    dto,
};
use tree_sitter::Point;

use crate::{imports::ImportUnit, tyres, utils::to_lsp_range, variable::LocalVariable, Document};

pub fn base(
    document: &Document,
    point: &Point,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<Hover> {
    let tree = &document.tree;
    let bytes = document.as_bytes();
    if let Some((class, range)) = class_action(tree, bytes, point, imports, class_map) {
        return Some(class_to_hover(class, range));
    }

    if let Some(hover) = call_chain_hover(document, point, lo_va, imports, class_map) {
        return Some(hover);
    }

    None
}

pub fn class_action(
    tree: &tree_sitter::Tree,
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
            if let Some(CallItem::ClassOrVariable { name: class, range }) =
                class_or_variable(n, bytes)
            {
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
    let Some(call_chain) = call_chain::get_call_chain(&document.tree, document.as_bytes(), point)
    else {
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
            let methods: Vec<dto::Method> = class
                .methods
                .into_iter()
                .filter(|m| m.name == *name)
                .collect();
            Some(methods_to_hover(&methods, to_lsp_range(*range)))
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
            Some(variables_to_hover(vec![var], to_lsp_range(*range)))
        }
        CallItem::Class { name: _, range } => Some(class_to_hover(class, to_lsp_range(*range))),
        CallItem::ClassOrVariable { name, range } => {
            let vars: Vec<_> = lo_va.iter().filter(|v| v.name == *name).collect();
            if vars.is_empty() {
                return Some(class_to_hover(class, to_lsp_range(*range)));
            }
            if let Ok(local_class) =
                parser::load_java(document.as_bytes(), parser::loader::SourceDestination::None)
            {
                let vars = vars
                    .iter()
                    .filter(|v| !v.is_fun)
                    .map(|v| format!("{} {}", v.jtype, v.name))
                    .collect::<Vec<_>>()
                    .join("\n");
                let fields = local_class
                    .fields
                    .iter()
                    .filter(|m| m.name == *name)
                    .map(|f| format_field(f))
                    .collect::<Vec<_>>()
                    .join("\n");
                let methods = local_class
                    .methods
                    .iter()
                    .filter(|m| m.name == *name)
                    .map(|m| format_method(m))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("{}\n{}\n{}", vars, fields, methods),
                    }),
                    range: Some(to_lsp_range(*range)),
                });
            }
            None
        }
        CallItem::ArgumentList {
            prev: _,
            range: _,
            active_param: _,
            filled_params: _,
        } => None,
    }
}

fn format_field(f: &dto::Field) -> String {
    format!("{} {}", f.jtype, f.name)
}

fn format_method(m: &dto::Method) -> String {
    let parameters = m
        .parameters
        .iter()
        .map(|p| format!("{}", p.jtype))
        .collect::<Vec<_>>()
        .join(", ");
    if m.throws.is_empty() {
        return format!("{} {}({:?})", m.ret, m.name, parameters);
    }
    let throws = m
        .throws
        .iter()
        .map(|p| format!("{}", p))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{} {}({}) throws {}", m.ret, m.name, parameters, throws)
}

fn variables_to_hover(vars: Vec<&LocalVariable>, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: vars
                .iter()
                .map(|v| format_varible_hover(v))
                .collect::<Vec<_>>()
                .join("\n"),
        }),
        range: Some(range),
    }
}

fn format_varible_hover(var: &&LocalVariable) -> String {
    if var.is_fun {
        return format!("{} {}()", var.jtype, var.name);
    }
    format!("{} {}", var.jtype, var.name)
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

fn methods_to_hover(methods: &Vec<dto::Method>, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: methods
                .iter()
                .map(format_method)
                .collect::<Vec<_>>()
                .join("\n"),
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
    use std::path::PathBuf;

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
        let doc = Document::setup(content, PathBuf::new()).unwrap();
        let tree = &doc.tree;
        let bytes = doc.as_bytes();

        let out = class_action(tree, bytes, &Point::new(3, 14), &[], &string_class_map());
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
        let doc = Document::setup(content, PathBuf::new()).unwrap();
        let tree = &doc.tree;
        let bytes = doc.as_bytes();

        let out = class_action(tree, bytes, &Point::new(3, 9), &[], &string_class_map());
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
        let doc = Document::setup(content, PathBuf::new()).unwrap();
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
                    throws: vec![],
                }],
                fields: vec![],
            },
        );
        class_map
    }
}
