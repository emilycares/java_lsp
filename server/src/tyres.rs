use std::ops::Deref;

use dashmap::DashMap;
use parser::dto::{Class, JType};

use crate::variable::{CallItem, LocalVariable};

pub fn is_imported(jtype: &str, imports: &[&str]) -> bool {
    imports.iter().any(|i| i.ends_with(jtype))
}

pub fn resolve<'a>(
    jtype: &str,
    imports: &[&'a str],
    class_map: &'a DashMap<std::string::String, parser::dto::Class>,
) -> Option<Class> {
    let lang_class_key = format!("java.lang.{}", jtype);
    if let Some(lang_class) = class_map.get(lang_class_key.as_str()) {
        return Some(lang_class.deref().to_owned());
    }
    if let Some(imported_class_path) = imports.iter().find(|i| i.ends_with(jtype)) {
        if let Some(imported_class) = class_map.get(*imported_class_path) {
            return Some(imported_class.deref().to_owned());
        }
    }
    None
}
pub fn resolve_import(
    jtype: &str,
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Vec<String> {
    class_map
        .clone()
        .into_read_only()
        .keys()
        .filter(|p| p.ends_with(jtype))
        .map(|a| a.to_string())
        .collect::<Vec<String>>()
}

pub fn resolve_var<'a>(
    extend: &LocalVariable,
    imports: &[&'a str],
    class_map: &'a DashMap<std::string::String, parser::dto::Class>,
) -> Option<Class> {
    resolve(&extend.jtype, imports, class_map)
}

pub fn resolve_call_chain(
    call_chain: &[CallItem<'_>],
    imports: &[&str],
    class_map: &DashMap<String, Class>,
) -> Option<Class> {
    let mut ops: Vec<Class> = vec![];
    for item in call_chain {
        let op = match item {
            CallItem::MethodCall(called) => {
                let Some(class) = ops.last() else {
                    eprintln!("There is no class in ops");
                    break;
                };
                if let Some(method) = class.methods.iter().find(|m| m.name == *called) {
                    if let Some(c) = resolve_jtype(&method.ret, imports, class_map) {
                        return Some(c);
                    }
                }
                None
            }
            CallItem::Variable(var) => resolve_var(var, imports, class_map),
        };
        if let Some(op) = op {
            ops.push(op);
        }
    }
    ops.last().cloned()
}

fn resolve_jtype(
    jtype: &JType,
    imports: &[&str],
    class_map: &DashMap<String, Class>,
) -> Option<Class> {
    match jtype {
        JType::Void
        | JType::Byte
        | JType::Char
        | JType::Double
        | JType::Float
        | JType::Int
        | JType::Long
        | JType::Short
        | JType::Boolean
        | JType::Array(_) => {
            eprintln!("Handle jvm resolve for internal types");
            return None;
        }
        JType::Class(c) => {
            if let Some(class) = resolve(&c, imports, class_map) {
                return Some(class);
            }
            return None;
        }
    }
}
