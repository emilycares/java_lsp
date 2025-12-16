use std::ops::Deref;

use dashmap::DashMap;
use my_string::MyString;
use parser::dto::{Class, SuperClass};

pub fn include_parent(class: Class, class_map: &DashMap<MyString, Class>) -> Class {
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

fn populate_super_class(class: &Class, class_map: &DashMap<MyString, Class>, s: &mut Vec<Class>) {
    let parent = load_parent(&class.super_class, class_map);
    if let Some(p) = parent {
        populate_super_class(&p, class_map, s);
        s.push(p);
    }
}

fn populate_super_interfaces(
    class: &Class,
    class_map: &DashMap<MyString, Class>,
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
        let mut method = m.clone();
        if method.source.is_none() {
            method.source = Some(c.source.clone());
        }
        out.methods.push(method);
    }
    for f in c.fields.clone() {
        let mut field = f.clone();
        if field.source.is_none() {
            field.source = Some(c.source.clone());
        }
        out.fields.push(field);
    }

    out
}

fn load_parent(super_class: &SuperClass, class_map: &DashMap<MyString, Class>) -> Option<Class> {
    match super_class {
        SuperClass::None => None,
        SuperClass::Name(n) => {
            let mut key = MyString::new();
            key.push_str("java.util.");
            key.push_str(n);
            class_map.get(&key).map(|p| p.deref().to_owned())
        }
        SuperClass::ClassPath(class_path) => {
            class_map.get(class_path).map(|p| p.deref().to_owned())
        }
    }
}
