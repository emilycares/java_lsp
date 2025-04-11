use std::{collections::HashMap, fs::read_to_string};

use parser::{
    dto::{Class, ImportUnit},
    java::parse_import_declarations,
};
use tree_sitter::Tree;
use tree_sitter_util::CommentSkiper;

use crate::{position, Document};

pub fn is_imported(imports: &[ImportUnit], class_path: &str) -> bool {
    for inp in imports {
        match inp {
            ImportUnit::Class(c) => {
                if *c == class_path {
                    return true;
                }
            }
            ImportUnit::StaticClass(c) => {
                if *c == class_path {
                    return true;
                }
            }
            ImportUnit::StaticClassMethod(c, _) => {
                if *c == class_path {
                    return true;
                }
            }
            ImportUnit::Prefix(p) => {
                if class_path.starts_with(p) {
                    return true;
                }
            }
            ImportUnit::StaticPrefix(p) => {
                if class_path.starts_with(p) {
                    return true;
                }
            }
            ImportUnit::Package(p) => {
                if class_path.starts_with(p) {
                    return true;
                }
            }
        }
    }
    false
}

pub fn imports(document: &Document) -> Vec<ImportUnit> {
    let tree = &document.tree;
    let bytes = document.as_bytes();
    return get_imported_classpaths(bytes, tree);
}

#[allow(dead_code)]
fn get_imported_classpaths<'a>(bytes: &'a [u8], tree: &Tree) -> Vec<ImportUnit> {
    let mut out = vec![];
    let mut cursor = tree.walk();
    cursor.first_child();

    cursor.first_child();
    cursor.sibling();
    let package = cursor
        .node()
        .utf8_text(bytes)
        .unwrap_or_default()
        .to_string();
    out.push(ImportUnit::Package(package));
    cursor.parent();

    cursor.sibling();
    out.extend(parse_import_declarations(bytes, &mut cursor));

    out
}

#[cfg(test)]
mod tests {
    use crate::imports::ImportUnit;
    use pretty_assertions::assert_eq;

    use super::get_imported_classpaths;

    #[test]
    fn classes() {
        let demo = "package heh.haha;

import java.util.List;
import java.util.stream.Collectors;
import static org.junit.jupiter.api.Assertions;
import static org.junit.jupiter.api.Assertions.assertEquals;
 
public class Controller {}";
        let (_, tree) = tree_sitter_util::parse(demo).unwrap();
        assert_eq!(
            get_imported_classpaths(demo.as_bytes(), &tree),
            vec![
                ImportUnit::Package("heh.haha".to_string()),
                ImportUnit::Class("java.util.List".to_string()),
                ImportUnit::Class("java.util.stream.Collectors".to_string()),
                ImportUnit::StaticClass("org.junit.jupiter.api.Assertions".to_string()),
                ImportUnit::StaticClassMethod(
                    "org.junit.jupiter.api.Assertions".to_string(),
                    "assertEquals".to_string()
                )
            ]
        );
    }

    #[test]
    fn stat() {
        let demo = "package heh.haha;

import java.util.*;
import static java.util.*;

public class Controller {}";
        let (_, tree) = tree_sitter_util::parse(demo).unwrap();
        assert_eq!(
            get_imported_classpaths(demo.as_bytes(), &tree),
            vec![
                ImportUnit::Package("heh.haha".to_string()),
                ImportUnit::Prefix("java.util".to_string()),
                ImportUnit::StaticPrefix("java.util".to_string())
            ]
        );
    }
}

pub fn init_import_map(
    project_classes: &[Class],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> HashMap<String, Vec<ImportUnit>> {
    let mut out: HashMap<String, Vec<ImportUnit>> = HashMap::new();
    for class in project_classes {
        for import in &class.imports {
            match import {
                ImportUnit::Package(p) | ImportUnit::Prefix(p) => {
                    let implicit_imports: Vec<String> = class_map
                        .clone()
                        .into_read_only()
                        .keys()
                        .filter(|c| {
                            if let Some((c_package, _)) = c.rsplit_once(".") {
                                return c_package == p;
                            }
                            false
                        })
                        .inspect(|a| {
                            dbg!(a);
                        })
                        .map(|a| a.to_string())
                        .map(|k| (k.clone(), class_map.get(&k)))
                        .filter(|(_, class)| class.is_some())
                        .map(|(k, class)| (k, class.unwrap()))
                        .map(|(k, c)| (k, c.source.clone()))
                        .filter_map(|(k, i)| Some((k, read_to_string(i).ok()?)))
                        .map(|(k, src)| (k, position::get_type_usage(src.as_str(), &class.name)))
                        .map(|(k, symbols)| {
                            (
                                k,
                                match symbols {
                                    Ok(s) => s,
                                    Err(e) => {
                                        eprintln!("Errors with workspace document symbol: {:?}", e);
                                        vec![]
                                    }
                                },
                            )
                        })
                        .filter_map(|(k, b)| {
                            if b.is_empty() {
                                return None;
                            }
                            Some(k)
                        })
                        .collect();
                    for s in implicit_imports {
                        if let Some(a) = out.get_mut(&s) {
                            a.push(ImportUnit::Class(class.class_path.clone()));
                        }
                    }
                }
                ImportUnit::Class(s) => match out.contains_key(s) {
                    true => {
                        if let Some(a) = out.get_mut(s) {
                            a.push(ImportUnit::Class(class.class_path.clone()));
                        }
                    }
                    false => {
                        out.insert(s.clone(), vec![ImportUnit::Class(class.class_path.clone())]);
                    }
                },
                ImportUnit::StaticClass(s) => match out.contains_key(s) {
                    true => {
                        if let Some(a) = out.get_mut(s) {
                            a.push(ImportUnit::StaticClass(class.class_path.clone()));
                        }
                    }
                    false => {
                        out.insert(
                            s.clone(),
                            vec![ImportUnit::StaticClass(class.class_path.clone())],
                        );
                    }
                },
                ImportUnit::StaticClassMethod(_, _) => (),
                ImportUnit::StaticPrefix(_) => (),
            }
        }
    }
    eprintln!("{:#?}", out);
    out
}
