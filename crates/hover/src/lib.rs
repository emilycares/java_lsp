use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use ast::types::{AstFile, AstPoint};
use call_chain::{self, CallItem};
use document::get_class_path;
use dto::{Access, Class, Field, ImportUnit, Method};
use local_variable::{LocalVariable, VarFlags};
use lsp_extra::{ToLspRangeError, to_lsp_range};
use lsp_types::{Hover, HoverContents, LanguageString, MarkupContent, MarkupKind, Range};
use my_string::{NuVec, NuVecBuilder};
use tyres::TyresError;

#[allow(dead_code)]
#[derive(Debug)]
pub enum HoverError {
    Tyres(TyresError),
    ValidatedItemDoesNotExists,
    LocalVariableNotFound { name: NuVec },
    Unimlemented,
    NoClass(NuVec),
    ArgumentNotFound,
    ToLspRange(ToLspRangeError),
    CouldNotFindClassPath,
}

pub fn base(
    ast: &AstFile,
    point: &AstPoint,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &Arc<RwLock<HashMap<NuVec, Class>>>,
) -> Result<Hover, HoverError> {
    match class_action(ast, point, lo_va, imports, class_map) {
        Ok((class, range)) => {
            return Ok(class_to_hover(&class, range));
        }
        Err(ClassActionError::NotFound | ClassActionError::Tyres(TyresError::NotImported(_))) => {}
        Err(e) => eprintln!("class action hover error: {e:?}"),
    }
    let Some(class_path) = get_class_path(ast) else {
        eprintln!("Could not get class_path");
        return Err(HoverError::CouldNotFindClassPath);
    };
    let class;
    if let Ok(cm) = class_map.read()
        && let Some(c) = cm.get(&class_path)
    {
        class = c.clone();
    } else {
        return Err(HoverError::NoClass(class_path));
    }

    let call_chain = call_chain::get_call_chain(ast, point);

    call_chain_hover(&call_chain, point, lo_va, imports, &class, class_map)
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum ClassActionError {
    /// No class for actions found
    NotFound,
    /// In the type resolution error
    Tyres(TyresError),
    ToLspRange(ToLspRangeError),
}

pub fn class_action(
    ast: &AstFile,
    point: &AstPoint,
    _lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &Arc<RwLock<HashMap<NuVec, Class>>>,
) -> Result<(Class, Range), ClassActionError> {
    if let Some(class) = get_class::get_class(ast, point) {
        let range = to_lsp_range(&class.range).map_err(ClassActionError::ToLspRange)?;
        return match tyres::resolve(&class.name, imports, class_map) {
            Ok(resolve_state) => Ok((resolve_state.class, range)),
            Err(tyres_error) => Err(ClassActionError::Tyres(tyres_error)),
        };
    }
    Err(ClassActionError::NotFound)
}

pub fn call_chain_hover(
    call_chain: &[CallItem],
    point: &AstPoint,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &Arc<RwLock<HashMap<NuVec, Class>>>,
) -> Result<Hover, HoverError> {
    let (item, relevant) = call_chain::validate(call_chain, point);
    let Some(el) = call_chain.get(item) else {
        return Err(HoverError::ValidatedItemDoesNotExists);
    };
    let resolve_state = match tyres::resolve_call_chain_to_point(
        &relevant,
        lo_va,
        imports,
        class,
        &class_map.clone(),
        point,
    ) {
        Ok(c) => Ok(c),
        Err(e) => Err(HoverError::Tyres(e)),
    }?;
    match el {
        CallItem::MethodCall { name, args, range } => {
            let args_len = args.len();
            let methods: Vec<Method> = resolve_state
                .class
                .methods
                .into_iter()
                .filter(|i| i.name.as_ref().is_some_and(|i| i == name))
                .filter(|i| i.parameters.len() == args_len)
                .collect();
            let range = to_lsp_range(range).map_err(HoverError::ToLspRange)?;
            Ok(methods_to_hover(&methods, range, &resolve_state.class.name))
        }
        CallItem::FieldAccess { name, range } => {
            let Some(method) = resolve_state.class.fields.iter().find(|m| m.name == *name) else {
                return Err(HoverError::LocalVariableNotFound { name: name.clone() });
            };
            let range = to_lsp_range(range).map_err(HoverError::ToLspRange)?;
            Ok(field_to_hover(method, range))
        }
        CallItem::Variable { name, range } => {
            let Some(var) = lo_va.iter().find(|v| &v.name == name) else {
                return Err(HoverError::LocalVariableNotFound { name: name.clone() });
            };
            let range = to_lsp_range(range).map_err(HoverError::ToLspRange)?;
            Ok(variables_to_hover(&[var], range))
        }
        CallItem::Class { range, .. } | CallItem::ClassGeneric { range, .. } => {
            let range = to_lsp_range(range).map_err(HoverError::ToLspRange)?;
            Ok(class_to_hover(&resolve_state.class, range))
        }
        CallItem::ClassOrVariable { name, range } => {
            let range = to_lsp_range(range).map_err(HoverError::ToLspRange)?;
            if let Some(var) = lo_va.iter().find(|v| &v.name == name) {
                return Ok(variables_to_hover(&[var], range));
            }
            Ok(class_to_hover(&resolve_state.class, range))
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
        CallItem::ArrayAccess { range: _ } | CallItem::This { range: _ } => {
            Err(HoverError::Unimlemented)
        }
    }
}

fn format_field(f: &Field) -> NuVec {
    let mut o = NuVecBuilder::new();
    o.extend(&f.jtype.to_nuvec());
    o.push(b' ');
    o.extend(&f.name);
    o.push(b';');
    o.finish()
}

fn format_method(m: &Method, class_name: &NuVec) -> NuVec {
    let mut out = NuVecBuilder::new();
    if m.access.intersects(Access::Static) {
        out.pusha(b"static ");
    }

    if let Some(name) = &m.name {
        out.extend(&m.ret.to_nuvec());
        out.push(b' ');
        out.extend(name);
    } else {
        out.extend(class_name);
    }

    out.push(b'(');
    let mut params = m.parameters.iter().peekable();
    while let Some(param) = params.next() {
        out.extend(&param.jtype.to_nuvec());
        if let Some(name) = &param.name {
            out.push(b' ');
            out.extend(name);
        }
        if params.peek().is_some() {
            out.pusha(b", ");
        }
    }
    out.push(b')');

    if !m.throws.is_empty() {
        out.pusha(b" throws ");
        let mut throw = m.throws.iter().peekable();
        while let Some(j) = throw.next() {
            out.extend(&j.to_nuvec());
            if throw.peek().is_some() {
                out.pusha(b", ");
            }
        }
    }

    out.push(b';');
    out.finish()
}

fn variables_to_hover(vars: &[&LocalVariable], range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: vars
                .iter()
                .map(|i| format_variable_hover(i))
                .collect::<Vec<_>>()
                .join("\n"),
        }),
        range: Some(range),
    }
}

fn format_variable_hover(var: &LocalVariable) -> String {
    if var.flags.intersects(VarFlags::Function) {
        return format!("{} {}()", var.jtype.to_nuvec(), var.name);
    }
    format!("{} {}", var.jtype.to_nuvec(), var.name)
}

fn field_to_hover(f: &Field, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: format!("{} {}", f.jtype, f.name),
        }),
        range: Some(range),
    }
}

fn methods_to_hover(methods: &[Method], range: Range, class_name: &NuVec) -> Hover {
    let mut o = NuVecBuilder::new();
    let mut it = methods.iter().peekable();
    while let Some(i) = it.next() {
        if i.access.intersects(Access::Private | Access::Deprecated) {
            continue;
        }
        o.extend(&format_method(i, class_name));
        if it.peek().is_some() {
            o.push(b'\n');
        }
    }
    let value = o.finish();
    Hover {
        contents: HoverContents::Scalar(lsp_types::MarkedString::LanguageString(LanguageString {
            language: String::from("java"),
            value: value.to_string(),
        })),
        range: Some(range),
    }
}

fn class_to_hover(class: &Class, range: Range) -> Hover {
    let value = format!("// {}\n{}", class.class_path, class_to_markdown(class));
    Hover {
        contents: HoverContents::Scalar(lsp_types::MarkedString::LanguageString(LanguageString {
            language: String::from("java"),
            value,
        })),
        range: Some(range),
    }
}

#[must_use]
pub fn class_to_markdown(class: &Class) -> String {
    let mut o = NuVecBuilder::new();
    {
        let mut first = true;
        for i in &class.methods {
            if i.access.intersects(Access::Private | Access::Deprecated) {
                continue;
            }
            if first {
                first = false;
            } else {
                o.push(b'\n');
            }
            o.extend(&format_method(i, &class.name));
        }
    }
    {
        let mut header = false;
        let mut first = true;
        for i in &class.fields {
            if i.access.intersects(Access::Private | Access::Deprecated) {
                continue;
            }
            if !header {
                header = true;
                o.pusha(b"\n// Fields\n");
            }
            if first {
                first = false;
            } else {
                o.push(b'\n');
            }
            o.extend(&format_field(i));
        }
    }
    let value = o.finish().trim_end_matches_byte(b'\n');
    value.to_string()
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{Arc, RwLock},
    };

    use ast::types::AstPoint;
    use document::Document;
    use dto::{Access, Class, JType, Method};
    use expect_test::expect;
    use my_string::NuVec;
    use variables::VariableContext;

    use crate::{call_chain_hover, class_action};

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
        let ast = &doc.ast;

        let out = class_action(ast, &AstPoint::new(3, 14), &[], &[], &string_class_map());
        let expected = expect![[r#"
            (
                Class {
                    class_path: "",
                    source: None,
                    access: Access(
                        Public,
                    ),
                    imports: [],
                    signature: None,
                    name: "String",
                    methods: [
                        Method {
                            access: Access(
                                Public,
                            ),
                            name: Some(
                                "length",
                            ),
                            parameters: [],
                            throws: [],
                            ret: Int,
                            source: None,
                        },
                    ],
                    fields: [],
                    super_class: None,
                    super_interfaces: [],
                },
                Range {
                    start: Position {
                        line: 3,
                        character: 11,
                    },
                    end: Position {
                        line: 3,
                        character: 17,
                    },
                },
            )
        "#]];
        expected.assert_debug_eq(&out.unwrap());
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
        let ast = &doc.ast;

        let out = class_action(ast, &AstPoint::new(3, 9), &[], &[], &string_class_map());
        let expected = expect![[r#"
            (
                Class {
                    class_path: "",
                    source: None,
                    access: Access(
                        Public,
                    ),
                    imports: [],
                    signature: None,
                    name: "String",
                    methods: [
                        Method {
                            access: Access(
                                Public,
                            ),
                            name: Some(
                                "length",
                            ),
                            parameters: [],
                            throws: [],
                            ret: Int,
                            source: None,
                        },
                    ],
                    fields: [],
                    super_class: None,
                    super_interfaces: [],
                },
                Range {
                    start: Position {
                        line: 3,
                        character: 5,
                    },
                    end: Position {
                        line: 3,
                        character: 11,
                    },
                },
            )
        "#]];
        expected.assert_debug_eq(&out.unwrap());
    }

    #[test]
    fn method_hover() {
        let class = Class {
            access: Access::Public,
            name: NuVec::new_static(b"Test"),
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
        let doc = Document::setup(content, PathBuf::new()).unwrap();
        let point = AstPoint::new(5, 29);
        let vars = variables::get_vars(
            &doc.ast,
            &VariableContext {
                point: Some(point),
                imports: &[],
                class: &class,
                class_map: string_class_map(),
            },
        )
        .unwrap();

        let chain = call_chain::get_call_chain(&doc.ast, &point);
        let out =
            call_chain_hover(&chain, &point, &vars, &[], &class, &string_class_map()).unwrap();
        let expected = expect![[r#"
            Hover {
                contents: Scalar(
                    LanguageString(
                        LanguageString {
                            language: "java",
                            value: "int length();",
                        },
                    ),
                ),
                range: Some(
                    Range {
                        start: Position {
                            line: 5,
                            character: 25,
                        },
                        end: Position {
                            line: 5,
                            character: 31,
                        },
                    },
                ),
            }
        "#]];
        expected.assert_debug_eq(&out);
    }

    fn string_class_map() -> Arc<RwLock<HashMap<NuVec, Class>>> {
        let mut class_map: HashMap<NuVec, Class> = HashMap::new();
        class_map.insert(
            NuVec::new_static(b"java.lang.String"),
            Class {
                access: Access::Public,
                name: NuVec::new_static(b"String"),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some(NuVec::new_static(b"length")),
                    ret: JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        Arc::new(RwLock::new(class_map))
    }
}
