use std::ops::Deref;

use dashmap::DashMap;
use parser::dto::{self, Class, JType};

use crate::{call_chain::CallItem, variable::LocalVariable};

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
    resolve_class_key(class_map, |p| p.ends_with(jtype))
}

pub fn resolve_class_key(
    class_map: &DashMap<std::string::String, parser::dto::Class>,
    infl: impl Fn(&&std::string::String) -> bool,
) -> Vec<String> {
    class_map
        .clone()
        .into_read_only()
        .keys()
        .filter(infl)
        .map(|a| a.to_string())
        .collect::<Vec<String>>()
}

pub fn resolve_var<'a>(
    extend: &LocalVariable,
    imports: &[&'a str],
    class_map: &'a DashMap<std::string::String, parser::dto::Class>,
) -> Option<Class> {
    resolve_jtype(&extend.jtype, imports, class_map)
}

pub fn resolve_call_chain(
    call_chain: &[CallItem],
    lo_va: &[LocalVariable],
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
            CallItem::FieldAccess(called) => {
                let Some(class) = ops.last() else {
                    eprintln!("There is no class in ops");
                    break;
                };
                if let Some(method) = class.fields.iter().find(|m| m.name == *called) {
                    if let Some(c) = resolve_jtype(&method.jtype, imports, class_map) {
                        return Some(c);
                    }
                }
                None
            }
            CallItem::Variable(var) => {
                if let Some(lo) = lo_va.iter().find(|va| va.name == *var) {
                    return resolve_var(lo, imports, class_map);
                }
                None
            }
            CallItem::Class(class) => {
                if let Some(c) = resolve(class, imports, class_map) {
                    return Some(c);
                }
                None
            }
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
        | JType::Boolean => {
            Some(Class {
                class_path: "".to_owned(),
                source: "".to_owned(),
                access: vec![],
                name: "".to_string(),
                methods: vec![],
                fields: vec![],
            })
        }
        JType::Array(gen) => {
            Some(Class {
                class_path: "".to_owned(),
                source: "".to_owned(),
                access: vec![],
                name: "array".to_string(),
                methods: vec![dto::Method {
                    access: vec![],
                    name: "clone".to_string(),
                    ret: JType::Array(gen.clone()),
                    parameters: vec![],
                }],
                fields: vec![dto::Field {
                    access: vec![],
                    name: "length".to_string(),
                    jtype: JType::Int,
                }],
            })
        }
        JType::Class(c) => {
            if let Some(class) = resolve(c, imports, class_map) {
                return Some(class);
            }
            None
        }
    }
}
