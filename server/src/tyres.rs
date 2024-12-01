use std::ops::Deref;

use dashmap::DashMap;
use parser::dto::Class;

use crate::variable::LocalVariable;

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
    resolve(&extend.ty, imports, class_map)
}
