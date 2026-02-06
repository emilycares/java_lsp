use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use my_string::{MyString, smol_str::format_smolstr};
use parser::dto::{Access, Class, ImportUnit, SuperClass};

use crate::{ImportResult, is_imported};

pub fn include_parent(class: Class, class_map: &Arc<Mutex<HashMap<MyString, Class>>>) -> Class {
    let mut s: Vec<Class> = vec![];

    populate_super_class(&class, class_map, &mut s);
    populate_super_interfaces(&class, class_map, &mut s);

    if s.is_empty() {
        return class;
    }

    let mut base = None;

    for c in s.iter().rev() {
        match base {
            Some(b) => base = Some(overlay_class(b, c)),
            None => base = Some(c.clone()),
        }
    }

    if let Some(b) = base {
        return overlay_class(class, &b);
    }

    class
}

fn populate_super_class(
    class: &Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    s: &mut Vec<Class>,
) {
    let parent = load_parent(&class.super_class, &class.imports, class_map);
    if let Some(p) = parent {
        populate_super_class(&p, class_map, s);
        s.push(p);
    }
}

fn populate_super_interfaces(
    class: &Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    s: &mut Vec<Class>,
) {
    for super_interface in &class.super_interfaces {
        let parent = load_parent(super_interface, &class.imports, class_map);
        if let Some(p) = parent {
            populate_super_interfaces(&p, class_map, s);
            s.push(p);
        }
    }
}
fn overlay_class(b: Class, c: &Class) -> Class {
    let mut out = b;

    for m in &c.methods {
        if m.access.intersects(Access::Private) {
            continue;
        }
        let mut method = m.clone();
        if method.source.is_none() {
            method.source = Some(c.source.clone());
        }
        out.methods.push(method);
    }
    for f in &c.fields {
        if f.access.intersects(Access::Private) {
            continue;
        }
        let mut field = f.clone();
        if field.source.is_none() {
            field.source = Some(c.source.clone());
        }
        out.fields.push(field);
    }

    out
}

fn load_parent(
    super_class: &SuperClass,
    imports: &[ImportUnit],
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
) -> Option<Class> {
    match super_class {
        SuperClass::None => None,
        SuperClass::Name(n) => {
            let key = format_smolstr!("java.util.{n}");
            if let Ok(class_map) = class_map.lock()
                && let Some(o) = class_map.get(&key).map(ToOwned::to_owned)
            {
                return Some(o);
            }
            let import_result = is_imported(n, imports, class_map);
            match import_result {
                Some(ImportResult::Class(imp) | ImportResult::StaticClass(imp)) => {
                    if let Ok(class_map) = class_map.lock()
                        && let Some(o) = class_map.get(&imp).map(ToOwned::to_owned)
                    {
                        return Some(o);
                    }
                    None
                }
                None => None,
            }
        }
        SuperClass::ClassPath(class_path) => class_map
            .lock()
            .map_or(None, |cm| cm.get(class_path).map(ToOwned::to_owned)),
    }
}
