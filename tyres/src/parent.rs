use std::ops::Deref;

use dashmap::DashMap;
use parser::dto::{Class, SuperClass};

pub fn inclued_parent(class: Class, class_map: &DashMap<String, Class>) -> Class {
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
        return overlay_class(b, &class);
    }

    class
}

fn populate_super_class(class: &Class, class_map: &DashMap<String, Class>, s: &mut Vec<Class>) {
    let parent = load_parent(&class.super_class, class_map);
    if let Some(p) = parent {
        populate_super_class(&p, class_map, s);
        s.push(p);
    }
}

fn populate_super_interfaces(
    class: &Class,
    class_map: &DashMap<String, Class>,
    s: &mut Vec<Class>,
) {
    for super_interface in &class.super_interfaces {
        let parent = load_parent(super_interface, class_map);
        if let Some(p) = parent {
            populate_super_interfaces(&p, class_map, s);
            s.push(p);
        }
    }
}
fn overlay_class(b: Class, c: &Class) -> Class {
    let mut out = b;

    for m in c.methods.clone() {
        let mut m = m.clone();
        if m.source.is_none() {
            m.source = Some(c.source.clone());
        }
        out.methods.push(m);
    }
    for f in c.fields.clone() {
        let mut f = f.clone();
        if f.source.is_none() {
            f.source = Some(c.source.clone());
        }
        out.fields.push(f);
    }

    out
}

fn load_parent(super_class: &SuperClass, class_map: &DashMap<String, Class>) -> Option<Class> {
    match super_class {
        SuperClass::None => None,
        SuperClass::Name(n) => class_map
            .get(&format!("java.util.{}", n))
            .map(|p| p.deref().to_owned()),
        SuperClass::ClassPath(class_path) => {
            class_map.get(class_path).map(|p| p.deref().to_owned())
        }
    }
}
