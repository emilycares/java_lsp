use call_chain::{self, CallItem, class_or_variable};
use document::Document;
use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};
use parser::dto::{self, ImportUnit};
use tree_sitter::Point;
use tree_sitter_util::lsp::to_lsp_range;
use tyres::TyresError;
use variables::LocalVariable;

#[allow(dead_code)]
#[derive(Debug)]
pub enum HoverError {
    ClassAction,
    Tyres(TyresError),
    CallChainEmpty,
    ParseError(parser::java::ParseJavaError),
    ValidatedItemDoesNotExists,
    LocalVariableNotFound { name: String },
    Unimlemented,
    NoClass(String),
    ArgumentNotFound,
}

pub fn base(
    document: &Document,
    point: &Point,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Result<Hover, HoverError> {
    let tree = &document.tree;
    let bytes = document.as_bytes();
    match class_action(tree, bytes, point, lo_va, imports, class_map) {
        Ok((class, range)) => {
            return Ok(class_to_hover(class, range));
        }
        Err(ClassActionError::NotFound) => {}
        Err(e) => eprintln!("class action hover error: {e:?}"),
    };
    let Some(class) = class_map.get(&document.class_path) else {
        return Err(HoverError::NoClass(document.class_path.clone()));
    };

    let Some(call_chain) = call_chain::get_call_chain(&document.tree, document.as_bytes(), point)
    else {
        return Err(HoverError::CallChainEmpty);
    };

    call_chain_hover(
        document,
        call_chain,
        point,
        lo_va,
        imports,
        class.value(),
        class_map,
    )
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum ClassActionError {
    /// No class for actions found
    NotFound,
    /// Under the cursor there was no text
    CouldNotGetNode,
    /// In the type resolution error
    Tyres {
        tyres_error: tyres::TyresError,
    },
    VariableFound,
}

pub fn class_action(
    tree: &tree_sitter::Tree,
    bytes: &[u8],
    point: &Point,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Result<(dto::Class, Range), ClassActionError> {
    let Ok(n) = tree_sitter_util::get_node_at_point(tree, *point) else {
        return Err(ClassActionError::CouldNotGetNode);
    };
    match n.kind() {
        "type_identifier" => {
            if let Ok(jtype) = n.utf8_text(bytes) {
                return match tyres::resolve(jtype, imports, class_map) {
                    Ok(resolve_state) => Ok((resolve_state.class, to_lsp_range(n.range()))),
                    Err(tyres_error) => Err(ClassActionError::Tyres { tyres_error }),
                };
            }
        }
        "identifier" => {
            if let Some(CallItem::ClassOrVariable {
                name: class,
                range: _,
            }) = class_or_variable(n, bytes)
            {
                if lo_va.iter().any(|v| v.name == class) {
                    return Err(ClassActionError::VariableFound);
                }
                return match tyres::resolve(&class, imports, class_map) {
                    Ok(resolve_state) => Ok((resolve_state.class, to_lsp_range(n.range()))),
                    Err(tyres_error) => Err(ClassActionError::Tyres { tyres_error }),
                };
            }
        }
        _ => {}
    };
    Err(ClassActionError::NotFound)
}

pub fn call_chain_hover(
    document: &Document,
    call_chain: Vec<CallItem>,
    point: &Point,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &dto::Class,
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Result<Hover, HoverError> {
    let (item, relevat) = call_chain::validate(&call_chain, point);
    let Some(el) = call_chain.get(item) else {
        return Err(HoverError::ValidatedItemDoesNotExists);
    };
    let resolve_state =
        match tyres::resolve_call_chain_to_point(&relevat, lo_va, imports, class, class_map, point)
        {
            Ok(c) => Ok(c),
            Err(e) => Err(HoverError::Tyres(e)),
        }?;
    match el {
        CallItem::MethodCall { name, range } => {
            let methods: Vec<dto::Method> = resolve_state
                .class
                .methods
                .into_iter()
                .filter(|m| m.name == *name)
                .collect();
            Ok(methods_to_hover(&methods, to_lsp_range(*range)))
        }
        CallItem::FieldAccess { name, range } => {
            let Some(method) = resolve_state.class.fields.iter().find(|m| m.name == *name) else {
                return Err(HoverError::LocalVariableNotFound {
                    name: name.to_owned(),
                });
            };
            Ok(field_to_hover(method, to_lsp_range(*range)))
        }
        CallItem::Variable { name, range } => {
            let Some(var) = lo_va.iter().find(|v| v.name == *name) else {
                return Err(HoverError::LocalVariableNotFound {
                    name: name.to_owned(),
                });
            };
            Ok(variables_to_hover(vec![var], to_lsp_range(*range)))
        }
        CallItem::Class { name: _, range } => {
            Ok(class_to_hover(resolve_state.class, to_lsp_range(*range)))
        }
        CallItem::ClassOrVariable { name, range } => {
            let vars: Vec<_> = lo_va.iter().filter(|v| v.name == *name).collect();
            if vars.is_empty() {
                return Ok(class_to_hover(resolve_state.class, to_lsp_range(*range)));
            }
            match parser::load_java(document.as_bytes(), parser::loader::SourceDestination::None) {
                Err(e) => Err(HoverError::ParseError(e)),
                Ok(local_class) => {
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
                        .map(format_field)
                        .collect::<Vec<_>>()
                        .join("\n");
                    let methods = local_class
                        .methods
                        .iter()
                        .filter(|m| m.name == *name)
                        .map(format_method)
                        .collect::<Vec<_>>()
                        .join("\n");
                    Ok(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("{}\n{}\n{}", vars, fields, methods),
                        }),
                        range: Some(to_lsp_range(*range)),
                    })
                }
            }
        }
        CallItem::ArgumentList {
            prev: _,
            active_param,
            filled_params,
            range: _,
        } => {
            if let Some(active_param) = active_param {
                if let Some(current_param) = filled_params.get(*active_param) {
                    return call_chain_hover(
                        document,
                        current_param.clone(),
                        point,
                        lo_va,
                        imports,
                        &resolve_state.class,
                        class_map,
                    );
                }
            }
            Err(HoverError::ArgumentNotFound)
        }
        CallItem::This { range: _ } => Err(HoverError::Unimlemented),
    }
}

fn format_field(f: &dto::Field) -> String {
    format!("{} {}", f.jtype, f.name)
}

fn format_method(m: &dto::Method) -> String {
    let parameters = m
        .parameters
        .iter()
        .map(|p| match p.name.as_deref() {
            Some(name) => format!("{} {}", p.jtype, name),
            None => format!("{}", p.jtype),
        })
        .collect::<Vec<_>>()
        .join(", ");
    if m.throws.is_empty() {
        return format!("{} {}({})", m.ret, m.name, parameters);
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
                .map(format_varible_hover)
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

fn methods_to_hover(methods: &[dto::Method], range: Range) -> Hover {
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
    use document::Document;
    use parser::dto;
    use tree_sitter::Point;

    use crate::hover::{call_chain_hover, class_action};

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
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();
        let tree = &doc.tree;
        let bytes = doc.as_bytes();

        let out = class_action(
            tree,
            bytes,
            &Point::new(3, 14),
            &[],
            &[],
            &string_class_map(),
        );
        assert!(out.is_ok());
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
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();
        let tree = &doc.tree;
        let bytes = doc.as_bytes();

        let out = class_action(
            tree,
            bytes,
            &Point::new(3, 9),
            &[],
            &[],
            &string_class_map(),
        );
        assert!(out.is_ok());
    }

    #[test]
    fn method_hover() {
        let class = dto::Class {
            access: vec![dto::Access::Public],
            name: "Test".to_string(),
            ..Default::default()
        };
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
    String other = \"asd\";
    String local = other.length().toString();
    }
}
";
        let doc = Document::setup(content, PathBuf::new(), "".to_string()).unwrap();
        let point = Point::new(5, 29);
        let vars = variables::get_vars(&doc, &point).unwrap();

        let chain = call_chain::get_call_chain(&doc.tree, doc.as_bytes(), &point).unwrap();
        let out = call_chain_hover(&doc, chain, &point, &vars, &[], &class, &string_class_map());
        assert!(out.is_ok());
    }

    fn string_class_map() -> DashMap<String, dto::Class> {
        let class_map: DashMap<String, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".to_string(),
            dto::Class {
                access: vec![dto::Access::Public],
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
        class_map
    }
}
