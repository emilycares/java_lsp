use std::ops::Deref;

use dashmap::DashMap;
use parser::{
    call_chain::CallItem,
    dto::{self, Class, JType},
};

use crate::{imports::ImportUnit, variable::LocalVariable};

#[derive(Debug, PartialEq, Clone)]
pub enum TyresError {
    NotImported,
    ClassNotFound { class_path: String },
    CallChainInvalid,
}

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
        ImportUnit::Package(p) | ImportUnit::Prefix(p) => {
            let possible_class_path = format!("{}.{}", p, jtype);
            if class_map.contains_key(&possible_class_path) {
                return Some(ImportResult::Class(possible_class_path));
            }
            None
        }
        ImportUnit::StaticPrefix(p) => {
            let possible_class_path = format!("{}.{}", p, jtype);
            if class_map.contains_key(&possible_class_path) {
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
) -> Result<Class, TyresError> {
    let lang_class_key = format!("java.lang.{}", jtype);
    if let Some(lang_class) = class_map.get(lang_class_key.as_str()) {
        return Ok(lang_class.deref().to_owned());
    }

    let import_result = is_imported(jtype, imports, class_map);
    match import_result {
        Some(ImportResult::Class(c)) | Some(ImportResult::StaticClass(c)) => {
            let Some(imported_class) = class_map.get(&c) else {
                return Err(TyresError::ClassNotFound { class_path: c });
            };
            Ok(imported_class.deref().to_owned())
        }
        None => Err(TyresError::NotImported),
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
) -> Result<Class, TyresError> {
    resolve_jtype(&extend.jtype, imports, class_map)
}

#[allow(dead_code)]
pub fn resolve_params(
    params: Vec<Vec<CallItem>>,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &DashMap<String, Class>,
) -> Vec<Result<Class, TyresError>> {
    params
        .iter()
        .map(|c| resolve_call_chain(c, lo_va, imports, class, class_map))
        .collect()
}

pub fn resolve_call_chain(
    call_chain: &[CallItem],
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &Class,
    class_map: &DashMap<String, Class>,
) -> Result<Class, TyresError> {
    let mut ops: Vec<Class> = vec![];
    for item in call_chain {
        let op = match item {
            CallItem::MethodCall { name, range: _ } => {
                let Some(class) = ops.last() else {
                    eprintln!("There is no class in ops");
                    break;
                };
                if let Some(method) = class.methods.iter().find(|m| m.name == *name) {
                    return Ok(resolve_jtype(&method.ret, imports, class_map)?);
                }
                None
            }
            CallItem::FieldAccess { name, range: _ } => {
                let Some(class) = ops.last() else {
                    eprintln!("There is no class in ops");
                    break;
                };
                if let Some(method) = class.fields.iter().find(|m| m.name == *name) {
                    return Ok(resolve_jtype(&method.jtype, imports, class_map)?);
                }
                None
            }
            CallItem::Variable { name, range: _ } => {
                if let Some(lo) = lo_va.iter().find(|va| va.name == *name) {
                    return resolve_var(lo, imports, class_map);
                }
                None
            }
            CallItem::This { range: _ } => Some(class.clone()),
            CallItem::Class { name, range: _ } => Some(resolve(name, imports, class_map)?),
            CallItem::ClassOrVariable { name, range: _ } => {
                if let Some(lo) = lo_va.iter().find(|va| va.name == *name) {
                    return resolve_var(lo, imports, class_map);
                }
                return Ok(resolve(name, imports, class_map)?);
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
    match ops.last() {
        Some(last) => Ok(last.to_owned()),
        None => Err(TyresError::CallChainInvalid),
    }
}

pub fn resolve_jtype(
    jtype: &JType,
    imports: &[ImportUnit],
    class_map: &DashMap<String, Class>,
) -> Result<Class, TyresError> {
    match jtype {
        JType::Void
        | JType::Byte
        | JType::Char
        | JType::Double
        | JType::Float
        | JType::Int
        | JType::Long
        | JType::Short
        | JType::Boolean => Ok(Class {
            class_path: "".to_owned(),
            source: "".to_owned(),
            access: vec![],
            name: "".to_string(),
            methods: vec![],
            fields: vec![],
        }),
        JType::Array(gen) => Ok(Class {
            class_path: "".to_owned(),
            source: "".to_owned(),
            access: vec![],
            name: "array".to_string(),
            methods: vec![dto::Method {
                access: vec![],
                name: "clone".to_string(),
                ret: JType::Array(gen.clone()),
                parameters: vec![],
                throws: vec![],
            }],
            fields: vec![dto::Field {
                access: vec![],
                name: "length".to_string(),
                jtype: JType::Int,
            }],
        }),
        JType::Class(c) => resolve(c, imports, class_map),
        JType::Generic(c, _vec) => resolve(c, imports, class_map),
    }
}
