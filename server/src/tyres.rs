use std::ops::Deref;

use crate::completion::LocaleVariableFunction;
use dashmap::DashMap;
use parser::dto::Class;

pub fn resolve_var<'a>(
    extend: &LocaleVariableFunction,
    imports: &Vec<&'a str>,
    class_map: &'a DashMap<std::string::String, parser::dto::Class>,
) -> Option<Class> {
    let lang_class_key = format!("java.lang.{}", &extend.ty);
    if let Some(lang_class) = class_map.get(lang_class_key.as_str()) {
        return Some(lang_class.deref().to_owned());
    }
    if let Some(imported_class_path) = imports.iter().find(|i| i.ends_with(&extend.ty)) {
        if let Some(imported_class) = class_map.get(*imported_class_path) {
            return Some(imported_class.deref().to_owned());
        }
    }
    None
}
