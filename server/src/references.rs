use std::{collections::HashMap, fs::read_to_string, hash::Hash};

use lsp_types::{Location, Range};
use parser::dto::{Class, ImportUnit};

use crate::{
    definition,
    position::{self, PositionSymbol},
    utils::to_lsp_range,
};

#[derive(Debug)]
pub enum ReferencesError {
    IoRead(String, std::io::Error),
    Position(position::PosionError),
}

#[derive(Debug)]
pub enum ReferenceUnit {
    Class(String),
    ClassWithPosition(String, PositionSymbol),
    StaticClass(String),
}

pub fn init_refernece_map(
    project_classes: &[Class],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Result<HashMap<String, Vec<ReferenceUnit>>, ReferencesError> {
    let mut out: HashMap<String, Vec<ReferenceUnit>> = HashMap::new();
    for class in project_classes {
        let class_path = class.class_path.clone();
        for import in &class.imports {
            match import {
                ImportUnit::Package(p) | ImportUnit::Prefix(p) => {
                    let implicit_imports = get_implicit_imports(class_map, class, p);
                    for (s, refs) in implicit_imports {
                        if let Some(a) = out.get_mut(&s) {
                            for r in refs {
                                a.push(ReferenceUnit::ClassWithPosition(class_path.clone(), r));
                            }
                        }
                    }
                }
                ImportUnit::Class(s) => {
                    let refpos = get_position_refrences(class, s)?;
                    if refpos.is_empty() {
                        let insert = vec![ReferenceUnit::Class(class.class_path.clone())];
                        insert_or_extend(&mut out, s, insert);
                    } else {
                        insert_or_extend(&mut out, s, refpos);
                    }
                }
                ImportUnit::StaticClass(s) => {
                    let insert = vec![ReferenceUnit::StaticClass(class.class_path.clone())];
                    insert_or_extend(&mut out, s, insert);
                }
                ImportUnit::StaticClassMethod(_, _) => (),
                ImportUnit::StaticPrefix(_) => (),
            }
        }
    }
    Ok(out)
}

fn get_position_refrences(
    class: &Class,
    query_class_path: &str,
) -> Result<Vec<ReferenceUnit>, ReferencesError> {
    if let Some(name) = ImportUnit::class_path_get_class_name(query_class_path) {
        match read_to_string(&class.source) {
            Err(e) => Err(ReferencesError::IoRead(class.source.clone(), e))?,
            Ok(source) => match position::get_type_usage(&source, name) {
                Err(e) => Err(ReferencesError::Position(e))?,
                Ok(usages) => {
                    return Ok(usages
                        .into_iter()
                        .map(|u| ReferenceUnit::ClassWithPosition(class.class_path.clone(), u))
                        .collect())
                }
            },
        }
    }
    Ok(vec![])
}

fn insert_or_extend<K, V, A>(out: &mut HashMap<K, V>, key: &K, insert: V)
where
    K: Eq + Hash + Clone,
    V: Extend<A> + IntoIterator<Item = A>,
{
    match out.contains_key(key) {
        true => {
            if let Some(a) = out.get_mut(key) {
                a.extend(insert);
            }
        }
        false => {
            out.insert(key.clone(), insert);
        }
    }
}

fn get_implicit_imports(
    class_map: &dashmap::DashMap<String, Class>,
    class: &Class,
    package: &String,
) -> Vec<(String, Vec<PositionSymbol>)> {
    class_map
        .clone()
        .into_read_only()
        .keys()
        .filter(|c| {
            if let Some((c_package, _)) = c.rsplit_once(".") {
                return c_package == package;
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
            Some((k, b))
        })
        .collect()
}

pub fn class_path(
    class_path: &str,
    reference_map: &HashMap<String, Vec<ReferenceUnit>>,
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<Option<Vec<Location>>> {
    if let Some(crefs) = reference_map.get(class_path) {
        let refs = crefs
            .iter()
            .map(|i| match i {
                ReferenceUnit::Class(s) => (class_map.get(s), None),
                ReferenceUnit::StaticClass(s) => (class_map.get(s), None),
                ReferenceUnit::ClassWithPosition(s, position_symbol) => {
                    (class_map.get(s), Some(position_symbol))
                }
            })
            .filter(|(lookup, _)| lookup.is_some())
            .map(|(lookup, range)| (lookup.unwrap(), range))
            .filter_map(
                |(lookup, range)| match definition::class_to_uri(lookup.value()) {
                    Ok(u) => Some((u, range)),
                    Err(e) => {
                        eprintln!("Referneces Uri error {:?}", e);
                        None
                    }
                },
            )
            .map(|(i, range)| Location {
                uri: i,
                range: range
                    .map(|p| to_lsp_range(p.get_range()))
                    .unwrap_or(Range::default()),
            })
            .collect();
        return Some(Some(refs));
    }
    None
}
