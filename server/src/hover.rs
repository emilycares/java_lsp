use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};
use parser::{
    call_chain::{self, class_or_variable, CallItem},
    dto::{self},
};
use tree_sitter::Point;

use crate::{
    imports::ImportUnit,
    tyres::{self, TyresError},
    utils::to_lsp_range,
    variable::LocalVariable,
    Document,
};

#[allow(dead_code)]
#[derive(Debug)]
pub enum HoverError {
    ClassActon(ClassActionError),
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
            eprintln!(".... class  hover");
            return Ok(class_to_hover(class, range));
        }
        Err(ClassActionError::NotFound) => {}
        Err(ClassActionError::VariableFound { var, range }) => {
            return Ok(variables_to_hover(vec![&var], range));
        }
        Err(e) => eprintln!("class action hover error: {:?}", e),
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
#[derive(Debug)]
pub enum ClassActionError {
    /// No class for actions found
    NotFound,
    /// Under the cursor there was no text
    CouldNotGetNode,
    /// In the type resolution error
    Tyres {
        tyres_error: tyres::TyresError,
    },
    VariableFound {
        var: LocalVariable,
        range: Range,
    },
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
                    Ok(class) => Ok((class, to_lsp_range(n.range()))),
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
                if let Some(var) = lo_va.iter().find(|v| v.name == class) {
                    return Err(ClassActionError::VariableFound {
                        var: var.clone(),
                        range: to_lsp_range(n.range()),
                    });
                }
                return match tyres::resolve(&class, imports, class_map) {
                    Ok(class) => Ok((class, to_lsp_range(n.range()))),
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
    let class = match tyres::resolve_call_chain(relevat, lo_va, imports, class, class_map) {
        Ok(c) => Ok(c),
        Err(e) => Err(HoverError::Tyres(e)),
    }?;
    return match el {
        CallItem::MethodCall { name, range } => {
            let methods: Vec<dto::Method> = class
                .methods
                .into_iter()
                .filter(|m| m.name == *name)
                .collect();
            Ok(methods_to_hover(&methods, to_lsp_range(*range)))
        }
        CallItem::FieldAccess { name, range } => {
            let Some(method) = class.fields.iter().find(|m| m.name == *name) else {
                return Err(HoverError::LocalVariableNotFound {
                    name: name.to_owned(),
                });
            };
            Ok(field_to_hover(&method, to_lsp_range(*range)))
        }
        CallItem::Variable { name, range } => {
            let Some(var) = lo_va.iter().find(|v| v.name == *name) else {
                return Err(HoverError::LocalVariableNotFound {
                    name: name.to_owned(),
                });
            };
            Ok(variables_to_hover(vec![var], to_lsp_range(*range)))
        }
        CallItem::Class { name: _, range } => Ok(class_to_hover(class, to_lsp_range(*range))),
        CallItem::ClassOrVariable { name, range } => {
            let vars: Vec<_> = lo_va.iter().filter(|v| v.name == *name).collect();
            if vars.is_empty() {
                return Ok(class_to_hover(class, to_lsp_range(*range)));
            }
            return match parser::load_java(
                document.as_bytes(),
                parser::loader::SourceDestination::None,
            ) {
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
                    return Ok(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("{}\n{}\n{}", vars, fields, methods),
                        }),
                        range: Some(to_lsp_range(*range)),
                    });
                }
            };
        }
        CallItem::ArgumentList {
            prev: _,
            active_param,
            filled_params,
            range: _,
        } => {
            if let Some(current_param) = filled_params.get(*active_param) {
                return call_chain_hover(
                    document,
                    current_param.clone(),
                    point,
                    lo_va,
                    imports,
                    &class,
                    class_map,
                );
            }
            Err(HoverError::ArgumentNotFound)
        }
        CallItem::This { range: _ } => Err(HoverError::Unimlemented),
    };
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
    use parser::{call_chain, dto};
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
            class_path: "".to_string(),
            source: "".to_string(),
            access: vec![dto::Access::Public],
            name: "Test".to_string(),
            methods: vec![],
            fields: vec![],
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
        let vars = variable::get_vars(&doc, &point);

        let chain = call_chain::get_call_chain(&doc.tree, doc.as_bytes(), &point).unwrap();
        let out = call_chain_hover(&doc, chain, &point, &vars, &[], &class, &string_class_map());
        assert!(out.is_ok());
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
