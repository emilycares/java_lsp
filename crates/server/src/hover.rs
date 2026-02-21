use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ast::types::{AstFile, AstPoint};
use call_chain::{self, CallItem};
use document::get_class_path;
use lsp_extra::{ToLspRangeError, to_lsp_range};
use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};
use my_string::MyString;
use parser::{
    dto::{Access, Class, Field, ImportUnit, JType, Method, SourceDestination},
    java::load_java_tree,
};
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
    CouldNotFindClassPath,
}

pub fn base(
    ast: &AstFile,
    point: &AstPoint,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
) -> Result<Hover, HoverError> {
    match class_action(ast, point, lo_va, imports, class_map) {
        Ok((class, range)) => {
            return Ok(class_to_hover(&class, range));
        }
        Err(ClassActionError::NotFound) => {}
        Err(e) => eprintln!("class action hover error: {e:?}"),
    }
    let Some(class_path) = get_class_path(ast) else {
        eprintln!("Could not get class_path");
        return Err(HoverError::CouldNotFindClassPath);
    };
    let class;
    if let Ok(cm) = class_map.lock()
        && let Some(c) = cm.get(&class_path)
    {
        class = c.clone();
    } else {
        return Err(HoverError::NoClass(class_path));
    }

    let call_chain = call_chain::get_call_chain(ast, point);

    call_chain_hover(ast, &call_chain, point, lo_va, imports, &class, class_map)
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
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
) -> Result<(Class, Range), ClassActionError> {
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
    ast: &AstFile,
    call_chain: &[CallItem],
    point: &AstPoint,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
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
        CallItem::MethodCall { name, range } => {
            let methods: Vec<Method> = resolve_state
                .class
                .methods
                .into_iter()
                .filter(|i| i.name.as_ref().filter(|i| *i == name).is_some())
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
            let vars = lo_va.iter().any(|v| v.name == *name);
            let range = to_lsp_range(range).map_err(HoverError::ToLspRange)?;
            if !vars {
                return Ok(class_to_hover(&resolve_state.class, range));
            }

            let local_class = load_java_tree(ast, SourceDestination::None);
            Ok(class_to_hover(&local_class, range))
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
                    ast,
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

fn format_field(f: &Field) -> String {
    format!("{} {}", jtype_hover_display(&f.jtype), f.name)
}

fn format_method(m: &Method, class_name: &str) -> String {
    let mut out = String::new();
    out.push_str(jtype_hover_display(&m.ret).as_str());
    out.push(' ');

    if let Some(name) = &m.name {
        out.push_str(name.as_str());
    } else {
        out.push_str(class_name);
    }
    out.push('(');
    let mut params = m.parameters.iter().peekable();
    while let Some(param) = params.next() {
        out.push_str(jtype_hover_display(&param.jtype).as_str());
        if let Some(name) = &param.name {
            out.push(' ');
            out.push_str(name.as_str());
        }
        if params.peek().is_some() {
            out.push_str(", ");
        }
    }
    out.push(')');

    if !m.throws.is_empty() {
        out.push_str(" throws ");
        let mut throw = m.throws.iter().peekable();
        while let Some(j) = throw.next() {
            out.push_str(j.to_string().as_str());
            if throw.peek().is_some() {
                out.push_str(", ");
            }
        }
    }
    out
}

fn jtype_hover_display(jtype: &JType) -> String {
    match jtype {
        JType::Void => "void".to_owned(),
        JType::Byte => "byte".to_owned(),
        JType::Char => "char".to_owned(),
        JType::Double => "double".to_owned(),
        JType::Float => "float".to_owned(),
        JType::Int => "int".to_owned(),
        JType::Long => "long".to_owned(),
        JType::Short => "short".to_owned(),
        JType::Boolean => "boolean".to_owned(),
        JType::Wildcard => "?".to_owned(),
        JType::Var => "var".to_owned(),
        JType::Class(s) => class_name_hover(s),
        JType::Array(jtype) => format!("{}[]", jtype_hover_display(jtype)),
        JType::Generic(jtype, jtypes) => format!(
            "{}<{}>",
            class_name_hover(jtype),
            jtypes
                .iter()
                .map(jtype_hover_display)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        JType::Parameter(p) => format!("<{p}>"),
        JType::Access { base, inner } => format!(
            "{}.{}",
            jtype_hover_display(base),
            jtype_hover_display(inner)
        ),
    }
}

fn class_name_hover(s: &str) -> String {
    if let Some((_, s)) = s.rsplit_once('.') {
        return s.replace('$', ".");
    }
    s.to_owned()
}

fn variables_to_hover(vars: &[&LocalVariable], range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
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
    if var.is_fun {
        return format!("{} {}()", jtype_hover_display(&var.jtype), var.name);
    }
    format!("{} {}", jtype_hover_display(&var.jtype), var.name)
}

fn field_to_hover(f: &Field, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("{} {}", f.jtype, f.name),
        }),
        range: Some(range),
    }
}

fn methods_to_hover(methods: &[Method], range: Range, class_name: &str) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: methods
                .iter()
                .filter(|i| {
                    i.access.contains(Access::Private) || i.access.contains(Access::Deprecated)
                })
                .map(|i| format_method(i, class_name))
                .collect::<Vec<_>>()
                .join("\n"),
        }),
        range: Some(range),
    }
}

fn class_to_hover(class: &Class, range: Range) -> Hover {
    let methods: Vec<_> = class
        .methods
        .iter()
        .filter(|i| !i.access.intersects(Access::Private | Access::Deprecated))
        .map(|i| format_method(i, &class.name))
        .collect();
    let fields: Vec<_> = class
        .fields
        .iter()
        .filter(|i| !i.access.contains(Access::Deprecated))
        .map(format_field)
        .collect();
    let mut value = format!("# {}\n```java\n", class.name);
    let has_fields = !fields.is_empty();
    if !methods.is_empty() {
        value.push_str(methods.join("\n").as_str());
        if has_fields {
            value.push('\n');
        }
    }
    if has_fields {
        value.push_str(fields.join("\n").as_str());
    }
    value.push_str("\n```");
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: Some(range),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use ast::types::AstPoint;
    use document::Document;
    use my_string::MyString;
    use parser::dto::{Access, Class, JType, Method};

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
        let doc = Document::setup(content, PathBuf::new()).unwrap();
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
        let doc = Document::setup(content, PathBuf::new()).unwrap();
        let ast = &doc.ast;

        let out = class_action(ast, &AstPoint::new(3, 9), &[], &[], &string_class_map());
        assert!(out.is_ok());
    }

    #[test]
    fn method_hover() {
        let class = Class {
            access: Access::Public,
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
        let doc = Document::setup(content, PathBuf::new()).unwrap();
        let point = AstPoint::new(5, 29);
        let vars = variables::get_vars(&doc.ast, &point).unwrap();

        let chain = call_chain::get_call_chain(&doc.ast, &point);
        let out = call_chain_hover(
            &doc.ast,
            &chain,
            &point,
            &vars,
            &[],
            &class,
            &string_class_map(),
        );
        assert!(out.is_ok());
    }

    fn string_class_map() -> Arc<Mutex<HashMap<MyString, Class>>> {
        let mut class_map: HashMap<MyString, Class> = HashMap::new();
        class_map.insert(
            "java.lang.String".into(),
            Class {
                access: Access::Public,
                name: "String".into(),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some("length".into()),
                    ret: JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        Arc::new(Mutex::new(class_map))
    }
}
