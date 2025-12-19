use ast::types::{AstFile, AstPoint};
use call_chain::{self, CallItem};
use document::Document;
use lsp_extra::{ToLspRangeError, to_lsp_range};
use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};
use my_string::MyString;
use parser::dto::{self, ImportUnit};
use tyres::TyresError;
use variables::LocalVariable;

#[allow(dead_code)]
#[derive(Debug)]
pub enum HoverError {
    Tyres(TyresError),
    CallChainEmpty,
    ParseError(parser::java::ParseJavaError),
    ValidatedItemDoesNotExists,
    LocalVariableNotFound { name: MyString },
    Unimlemented,
    NoClass(MyString),
    ArgumentNotFound,
    ToLspRange(ToLspRangeError),
}

pub fn base(
    document: &Document,
    point: &AstPoint,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<MyString, parser::dto::Class>,
) -> Result<Hover, HoverError> {
    let ast = &document.ast;
    match class_action(ast, point, lo_va, imports, class_map) {
        Ok((class, range)) => {
            return Ok(class_to_hover(&class, range));
        }
        Err(ClassActionError::NotFound) => {}
        Err(e) => eprintln!("class action hover error: {e:?}"),
    }
    let Some(class) = class_map.get(&document.class_path) else {
        return Err(HoverError::NoClass(document.class_path.clone()));
    };

    let call_chain = call_chain::get_call_chain(ast, point);

    call_chain_hover(
        document,
        &call_chain,
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
    VariableFound,
    ToLspRange(ToLspRangeError),
}

pub fn class_action(
    ast: &AstFile,
    point: &AstPoint,
    _lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<MyString, parser::dto::Class>,
) -> Result<(dto::Class, Range), ClassActionError> {
    if let Some(class) = get_class::get_class(ast, point) {
        let range = to_lsp_range(&class.range).map_err(ClassActionError::ToLspRange)?;
        return match tyres::resolve(&class.name, imports, class_map) {
            Ok(resolve_state) => Ok((resolve_state.class, range)),
            Err(tyres_error) => Err(ClassActionError::Tyres { tyres_error }),
        };
    }
    Err(ClassActionError::NotFound)
}

pub fn call_chain_hover(
    document: &Document,
    call_chain: &[CallItem],
    point: &AstPoint,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &dto::Class,
    class_map: &dashmap::DashMap<MyString, parser::dto::Class>,
) -> Result<Hover, HoverError> {
    let (item, relevant) = call_chain::validate(call_chain, point);
    let Some(el) = call_chain.get(item) else {
        return Err(HoverError::ValidatedItemDoesNotExists);
    };
    let resolve_state = match tyres::resolve_call_chain_to_point(
        &relevant, lo_va, imports, class, class_map, point,
    ) {
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
            let range = to_lsp_range(range).map_err(HoverError::ToLspRange)?;
            Ok(methods_to_hover(&methods, range))
        }
        CallItem::FieldAccess { name, range } => {
            let Some(method) = resolve_state.class.fields.iter().find(|m| m.name == *name) else {
                return Err(HoverError::LocalVariableNotFound { name: name.clone() });
            };
            let range = to_lsp_range(range).map_err(HoverError::ToLspRange)?;
            Ok(field_to_hover(method, range))
        }
        CallItem::Variable { name, range } => {
            let Some(var) = lo_va.iter().find(|v| v.name == *name) else {
                return Err(HoverError::LocalVariableNotFound { name: name.clone() });
            };
            let range = to_lsp_range(range).map_err(HoverError::ToLspRange)?;
            Ok(variables_to_hover(&[var], range))
        }
        CallItem::Class { name: _, range } => {
            let range = to_lsp_range(range).map_err(HoverError::ToLspRange)?;
            Ok(class_to_hover(&resolve_state.class, range))
        }
        CallItem::ClassOrVariable { name, range } => {
            let vars: Vec<_> = lo_va.iter().filter(|v| v.name == *name).collect();
            let range = to_lsp_range(range).map_err(HoverError::ToLspRange)?;
            if vars.is_empty() {
                return Ok(class_to_hover(&resolve_state.class, range));
            }
            match parser::load_java(document.as_bytes(), parser::SourceDestination::None) {
                Err(e) => Err(HoverError::ParseError(e)),
                Ok(local_class) => {
                    let vars = vars
                        .iter()
                        .filter(|v| !v.is_fun)
                        .map(|v| format!("{} {}", v.jtype, v.name))
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    let fields = local_class
                        .fields
                        .iter()
                        .filter(|m| m.name == *name)
                        .map(format_field)
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    let methods = local_class
                        .methods
                        .iter()
                        .filter(|m| m.name == *name)
                        .map(format_method)
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    Ok(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("{vars}\n\n{fields}\n\n{methods}"),
                        }),
                        range: Some(range),
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
            if let Some(active_param) = active_param
                && let Some(current_param) = filled_params.get(*active_param)
            {
                return call_chain_hover(
                    document,
                    current_param,
                    point,
                    lo_va,
                    imports,
                    &resolve_state.class,
                    class_map,
                );
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
        .map(|p| {
            p.name.as_ref().map_or_else(
                || format!("{}", p.jtype),
                |name| format!("{} {}", p.jtype, name),
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    if m.throws.is_empty() {
        return format!("{} {}({})", m.ret, m.name, parameters);
    }
    let throws = m
        .throws
        .iter()
        .map(|p| format!("{p}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{} {}({}) throws {}", m.ret, m.name, parameters, throws)
}

fn variables_to_hover(vars: &[&LocalVariable], range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: vars
                .iter()
                .map(|i| format_variable_hoveer(i))
                .collect::<Vec<_>>()
                .join("\n"),
        }),
        range: Some(range),
    }
}

fn format_variable_hoveer(var: &LocalVariable) -> String {
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

fn class_to_hover(class: &dto::Class, range: Range) -> Hover {
    let methods: Vec<_> = class.methods.iter().map(format_method).collect();
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("# {}\n```java\n{}\n```", class.name, methods.join("\n")),
        }),
        range: Some(range),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ast::types::AstPoint;
    use dashmap::DashMap;
    use document::Document;
    use my_string::MyString;
    use parser::dto;

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
        let doc = Document::setup(content, PathBuf::new()).unwrap().0;
        let ast = &doc.ast;

        let out = class_action(ast, &AstPoint::new(3, 14), &[], &[], &string_class_map());
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
        let doc = Document::setup(content, PathBuf::new()).unwrap().0;
        let ast = &doc.ast;

        let out = class_action(ast, &AstPoint::new(3, 9), &[], &[], &string_class_map());
        assert!(out.is_ok());
    }

    #[test]
    fn method_hover() {
        let class = dto::Class {
            access: dto::Access::Public,
            name: "Test".into(),
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
        let doc = Document::setup(content, PathBuf::new()).unwrap().0;
        let point = AstPoint::new(5, 29);
        let vars = variables::get_vars(&doc.ast, &point).unwrap();

        let chain = call_chain::get_call_chain(&doc.ast, &point);
        let out = call_chain_hover(
            &doc,
            &chain,
            &point,
            &vars,
            &[],
            &class,
            &string_class_map(),
        );
        assert!(out.is_ok());
    }

    fn string_class_map() -> DashMap<MyString, dto::Class> {
        let class_map: DashMap<MyString, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".into(),
            dto::Class {
                access: dto::Access::Public,
                name: "String".into(),
                methods: vec![dto::Method {
                    access: dto::Access::Public,
                    name: "length".into(),
                    ret: dto::JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map
    }
}
