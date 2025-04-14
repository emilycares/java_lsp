use std::{
    fs::{self},
    hash::Hash,
};

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
    Treesitter(tree_sitter_util::TreesitterError),
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
    reference_map: &dashmap::DashMap<String, Vec<ReferenceUnit>>,
) -> Result<(), ReferencesError> {
    for class in project_classes {
        reference_update_class(class, None, class_map, &reference_map)?;
    }
    Ok(())
}

pub fn reference_update_class(
    class: &Class,
    tree_buff: Option<(&tree_sitter::Tree, &[u8])>,
    class_map: &dashmap::DashMap<String, Class>,
    reference_map: &dashmap::DashMap<String, Vec<ReferenceUnit>>,
) -> Result<(), ReferencesError> {
    let class_path = class.class_path.clone();
    for import in &class.imports {
        match import {
            ImportUnit::Package(p) | ImportUnit::Prefix(p) => {
                let implicit_imports = get_implicit_imports(class_map, class, p);
                for (s, refs) in implicit_imports {
                    if let Some(mut a) = reference_map.get_mut(&s) {
                        for r in refs {
                            a.push(ReferenceUnit::ClassWithPosition(class_path.clone(), r));
                        }
                    }
                }
            }
            ImportUnit::Class(s) => {
                let refpos = get_position_refrences(class, s, tree_buff)?;
                if refpos.is_empty() {
                    let insert = vec![ReferenceUnit::Class(class.class_path.clone())];
                    insert_or_extend(reference_map, s, insert);
                } else {
                    insert_or_extend(reference_map, s, refpos);
                }
            }
            ImportUnit::StaticClass(s) => {
                let insert = vec![ReferenceUnit::StaticClass(class.class_path.clone())];
                insert_or_extend(reference_map, s, insert);
            }
            ImportUnit::StaticClassMethod(_, _) => (),
            ImportUnit::StaticPrefix(_) => (),
        }
    }
    Ok(())
}

fn get_position_refrences(
    class: &Class,
    query_class_path: &str,
    tree_bytes: Option<(&tree_sitter::Tree, &[u8])>,
) -> Result<Vec<ReferenceUnit>, ReferencesError> {
    if let Some(name) = ImportUnit::class_path_get_class_name(query_class_path) {
        if let Some((tree, bytes)) = tree_bytes {
            return pos_refs_helper(&class.class_path, tree, bytes, name);
        } else {
            match fs::read(&class.source) {
                Err(e) => Err(ReferencesError::IoRead(class.source.clone(), e))?,
                Ok(bytes) => {
                    let (_, tree) = tree_sitter_util::parse(&bytes)
                        .map_err(|e| ReferencesError::Treesitter(e))?;
                    pos_refs_helper(&class.class_path, &tree, &bytes, name)?;
                }
            }
        }
    }
    Ok(vec![])
}

fn pos_refs_helper(
    class_path: &String,
    tree: &tree_sitter::Tree,
    bytes: &[u8],
    name: &str,
) -> Result<Vec<ReferenceUnit>, ReferencesError> {
    match position::get_type_usage(&bytes, name, &tree) {
        Err(e) => Err(ReferencesError::Position(e))?,
        Ok(usages) => {
            return Ok(usages
                .into_iter()
                .map(|u| ReferenceUnit::ClassWithPosition(class_path.clone(), u))
                .collect())
        }
    }
}

fn insert_or_extend<K, V, A>(out: &dashmap::DashMap<K, V>, key: &K, insert: V)
where
    K: Eq + Hash + Clone,
    V: Extend<A> + IntoIterator<Item = A>,
{
    match out.contains_key(key) {
        true => {
            if let Some(mut a) = out.get_mut(key) {
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
        .filter_map(|(k, i)| Some((k, fs::read(i).ok()?)))
        .filter_map(|(k, src)| {
            let (_, tree) = tree_sitter_util::parse(&src).ok()?;
            Some((k, position::get_type_usage(&src, &class.name, &tree)))
        })
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
    reference_map: &dashmap::DashMap<String, Vec<ReferenceUnit>>,
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<Vec<Location>> {
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
        return Some(refs);
    }
    None
}
