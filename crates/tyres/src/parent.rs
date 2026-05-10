use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use dto::{Access, Class, ImportUnit, JType, SuperClass};
use my_string::{
    MyString,
    smol_str::{SmolStr, format_smolstr},
};

use crate::{ImportResult, is_imported};

pub fn include_parent(
    class: Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    args: &[JType],
) -> Class {
    let mut s: Vec<Class> = vec![];

    let mut class = class;

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

    if let Some(sig) = &class.signature {
        let class_signature: Vec<_> = sig.args.iter().enumerate().collect();
        for m in &mut class.methods {
            replace_generic_jtype(&mut m.ret, &class_signature, args);
            for arg in &mut m.parameters {
                replace_generic_jtype(&mut arg.jtype, &class_signature, args);
            }
        }

        for f in &mut class.fields {
            replace_generic_jtype(&mut f.jtype, &class_signature, args);
        }
    }

    class
}

fn replace_generic_jtype(jtype: &mut JType, class_signature: &[(usize, &SmolStr)], args: &[JType]) {
    match jtype {
        JType::Array(jtype) => replace_generic_jtype(jtype, class_signature, args),
        JType::Generic(_, jtypes) => {
            for g in jtypes {
                replace_generic_jtype(g, class_signature, args);
            }
        }
        JType::Parameter(p) => {
            if let Some((i, _)) = class_signature.iter().find(|i| p.eq(&i.1))
                && let Some(r) = args.get(*i)
            {
                r.clone_into(jtype);
            } else {
                *jtype = JType::Parameter(p.clone());
            }
        }
        _ => (),
    }
}

pub fn populate_super_class(
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

pub fn populate_super_interfaces(
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
            method.source = Some(c.get_source());
        }
        out.methods.push(method);
    }
    for f in &c.fields {
        if f.access.intersects(Access::Private) {
            continue;
        }
        let mut field = f.clone();
        if field.source.is_none() {
            field.source = Some(c.get_source());
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
            if let Some(package) = imports.iter().find_map(|i| match i {
                ImportUnit::Package(smol_str) => Some(smol_str),
                _ => None,
            }) {
                let key = format_smolstr!("{package}.{n}");
                if let Ok(cm) = class_map.lock()
                    && let Some(o) = cm.get(&key)
                {
                    return Some(o.clone());
                }
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
