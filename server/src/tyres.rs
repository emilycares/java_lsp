use std::ops::Deref;

use dashmap::DashMap;
use parser::{
    call_chain::CallItem,
    dto::{self, Class, JType},
};

use crate::{imports::ImportUnit, variable::LocalVariable};

pub fn is_imported_class_name(
    jtype: &str,
    imports: &[ImportUnit],
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> bool {
    is_imported(jtype, imports, class_map).is_some()
}

#[derive(Debug)]
pub enum ImportResult {
    Class(String),
    StaticClass(String),
}

pub fn is_imported<'a>(
    jtype: &'a str,
    imports: &'a [ImportUnit],
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Option<ImportResult> {
    imports.iter().find_map(|i| match i {
        ImportUnit::Class(c) => {
            if c.ends_with(jtype) {
                return Some(ImportResult::Class(c.to_string()));
            }
            None
        }
        ImportUnit::StaticClass(c) => {
            if c.ends_with(jtype) {
                return Some(ImportResult::StaticClass(c.to_string()));
            }
            None
        }
        ImportUnit::Prefix(p) => {
            let possible_class_path = format!("{}.{}", p, jtype);
            if class_map.get(&possible_class_path).is_some() {
                return Some(ImportResult::Class(possible_class_path));
            }
            None
        }
        ImportUnit::StaticPrefix(p) => {
            let possible_class_path = format!("{}.{}", p, jtype);
            if class_map.get(&possible_class_path).is_some() {
                return Some(ImportResult::StaticClass(possible_class_path));
            }
            None
        }
        ImportUnit::StaticClassMethod(_, _) => None,
    })
}

pub fn resolve(
    jtype: &str,
    imports: &[ImportUnit],
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Option<Class> {
    let lang_class_key = format!("java.lang.{}", jtype);
    if let Some(lang_class) = class_map.get(lang_class_key.as_str()) {
        return Some(lang_class.deref().to_owned());
    }

    let import_result = is_imported(jtype, imports, class_map);
    match import_result {
        Some(ImportResult::Class(c)) => {
            if let Some(imported_class) = class_map.get(&c) {
                return Some(imported_class.deref().to_owned());
            }
            None
        }
        Some(ImportResult::StaticClass(c)) => {
            if let Some(imported_class) = class_map.get(&c) {
                let class = imported_class.deref().to_owned();
                // TODO: Return static version of class
                return Some(class);
            }
            None
        }
        None => None,
    }
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

pub fn resolve_var(
    extend: &LocalVariable,
    imports: &[ImportUnit],
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Option<Class> {
    resolve_jtype(&extend.jtype, imports, class_map)
}

#[allow(dead_code)]
pub fn resolve_params(
    params: Vec<Vec<CallItem>>,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &DashMap<String, Class>,
) -> Vec<Option<Class>> {
    params
        .iter()
        .map(|c| resolve_call_chain(c, lo_va, imports, class_map))
        .collect()
}

pub fn resolve_call_chain(
    call_chain: &[CallItem],
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &DashMap<String, Class>,
) -> Option<Class> {
    let mut ops: Vec<Class> = vec![];
    for item in call_chain {
        let op = match item {
            CallItem::MethodCall { name, range: _ } => {
                let Some(class) = ops.last() else {
                    eprintln!("There is no class in ops");
                    break;
                };
                if let Some(method) = class.methods.iter().find(|m| m.name == *name) {
                    if let Some(c) = resolve_jtype(&method.ret, imports, class_map) {
                        return Some(c);
                    }
                }
                None
            }
            CallItem::FieldAccess { name, range: _ } => {
                let Some(class) = ops.last() else {
                    eprintln!("There is no class in ops");
                    break;
                };
                if let Some(method) = class.fields.iter().find(|m| m.name == *name) {
                    if let Some(c) = resolve_jtype(&method.jtype, imports, class_map) {
                        return Some(c);
                    }
                }
                None
            }
            CallItem::Variable { name, range: _ } => {
                if let Some(lo) = lo_va.iter().find(|va| va.name == *name) {
                    return resolve_var(lo, imports, class_map);
                }
                None
            }
            CallItem::Class { name, range: _ } => {
                if let Some(c) = resolve(name, imports, class_map) {
                    return Some(c);
                }
                None
            }
            CallItem::ClassOrVariable { name, range: _ } => {
                if let Some(lo) = lo_va.iter().find(|va| va.name == *name) {
                    return resolve_var(lo, imports, class_map);
                }
                if let Some(c) = resolve(name, imports, class_map) {
                    return Some(c);
                }
                None
            }
            CallItem::ArgumentList {
                prev: _,
                range: _,
                active_param: _,
                filled_params: _,
            } => None,
        };
        if let Some(op) = op {
            ops.push(op);
        }
    }
    ops.last().cloned()
}

pub fn resolve_jtype(
    jtype: &JType,
    imports: &[ImportUnit],
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
        | JType::Boolean => Some(Class {
            class_path: "".to_owned(),
            source: "".to_owned(),
            access: vec![],
            name: "".to_string(),
            methods: vec![],
            fields: vec![],
        }),
        JType::Array(gen) => Some(Class {
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
        }),
        JType::Class(c) => {
            if let Some(class) = resolve(c, imports, class_map) {
                return Some(class);
            }
            None
        }
    }
}
