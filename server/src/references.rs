use std::{
    fs::{self},
    hash::Hash,
};

use lsp_types::{Location, Uri};
use parser::{
    call_chain::{self, CallItem},
    dto::{Class, ImportUnit},
};
use tree_sitter::Point;

use crate::{
    definition::{self},
    position::{self, PositionSymbol},
    tyres,
    utils::to_lsp_range,
    variable::LocalVariable,
};

#[derive(Debug)]
pub enum ReferencesError {
    IoRead(String, std::io::Error),
    Position(position::PosionError),
    Treesitter(tree_sitter_util::TreesitterError),
    FindClassnameInClasspath(String),
    Tyres(tyres::TyresError),
    ValidatedItemDoesNotExists,
    ArgumentNotFound,
    Definition(definition::DefinitionError),
}

#[derive(Debug)]
pub enum ReferenceUnit {
    Class(String),
    StaticClass(String),
}
#[derive(Debug)]
pub struct ReferencePosition(PositionSymbol);

pub fn init_refernece_map(
    project_classes: &[Class],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
    reference_map: &dashmap::DashMap<String, Vec<ReferenceUnit>>,
) -> Result<(), ReferencesError> {
    for class in project_classes {
        reference_update_class(class, class_map, &reference_map)?;
    }
    Ok(())
}

pub fn reference_update_class(
    class: &Class,
    class_map: &dashmap::DashMap<String, Class>,
    reference_map: &dashmap::DashMap<String, Vec<ReferenceUnit>>,
) -> Result<(), ReferencesError> {
    let class_path = class.class_path.clone();
    for import in &class.imports {
        match import {
            ImportUnit::Package(p) | ImportUnit::Prefix(p) => {
                let implicit_imports = get_implicit_imports(class_map, class, p);
                for s in implicit_imports {
                    if let Some(mut a) = reference_map.get_mut(&s) {
                        a.push(ReferenceUnit::Class(class_path.clone()));
                    }
                }
            }
            ImportUnit::Class(s) => {
                let insert = vec![ReferenceUnit::Class(class.class_path.clone())];
                insert_or_extend(reference_map, s, insert);
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
) -> Result<Vec<ReferencePosition>, ReferencesError> {
    let Some(query_class_name) = ImportUnit::class_path_get_class_name(query_class_path) else {
        return Err(ReferencesError::FindClassnameInClasspath(
            query_class_path.to_string(),
        ));
    };
    if let Some((tree, bytes)) = tree_bytes {
        return pos_refs_helper(tree, bytes, query_class_name);
    } else {
        return match fs::read(&class.source) {
            Err(e) => Err(ReferencesError::IoRead(class.source.clone(), e)),
            Ok(bytes) => {
                let (_, tree) =
                    tree_sitter_util::parse(&bytes).map_err(|e| ReferencesError::Treesitter(e))?;
                pos_refs_helper(&tree, &bytes, query_class_name)
            }
        };
    }
}

fn pos_refs_helper(
    tree: &tree_sitter::Tree,
    bytes: &[u8],
    query_class_name: &str,
) -> Result<Vec<ReferencePosition>, ReferencesError> {
    match position::get_type_usage(&bytes, query_class_name, &tree) {
        Err(e) => Err(ReferencesError::Position(e))?,
        Ok(usages) => return Ok(usages.into_iter().map(|u| ReferencePosition(u)).collect()),
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
) -> Vec<String> {
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
        .map(|a| a.to_string())
        .map(|k| (k.clone(), class_map.get(&k)))
        .filter(|(_, class)| class.is_some())
        .map(|(k, class)| (k, class.unwrap()))
        // Prefilter already parsed data before parsing file with treesitter
        .filter(|(_k, lclass)| {
            lclass.imports.iter().any(|i| match i {
                ImportUnit::Class(lclasspath) => {
                    ImportUnit::class_path_match_class_name(&lclasspath, &class.name)
                }
                _ => false,
            })
        })
        .map(|i| i.0)
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
            .filter_map(|i| match i {
                ReferenceUnit::Class(s) => class_map.get(s),
                ReferenceUnit::StaticClass(s) => class_map.get(s),
            })
            .filter_map(|lookup| {
                let refs = get_position_refrences(&lookup, class_path, None).ok()?;
                Some((lookup, refs.first().map(|i| i.0.get_range())?))
            })
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
                range: to_lsp_range(range),
            })
            .collect();
        return Some(refs);
    }
    None
}

pub fn call_chain_references(
    document_uri: Uri,
    point: &Point,
    call_chain: &Vec<CallItem>,
    vars: &[LocalVariable],
    imports: &[ImportUnit],
    class: &dashmap::mapref::one::Ref<'_, String, Class>,
    class_map: &dashmap::DashMap<String, Class>,
    reference_map: &dashmap::DashMap<String, Vec<ReferenceUnit>>,
) -> Result<Vec<Location>, ReferencesError> {
    let (item, relevat) = call_chain::validate(&call_chain, point);

    // let extend_class = tyres::resolve_call_chain(relevat, vars, imports, class, class_map)
    //     .map_err(|e| ReferencesError::Tyres(e))?;
    return match relevat.get(item) {
        // Some(CallItem::MethodCall { name, range: _ }) => {}
        // Some(CallItem::FieldAccess { name, range: _ }) => {}
        // Some(CallItem::Variable { name, range: _ }) => {}
        // Some(CallItem::ClassOrVariable { name, range: _ }) => {}
        Some(CallItem::ArgumentList {
            prev: _,
            active_param,
            filled_params,
            range: _,
        }) => {
            if let Some(current_param) = filled_params.get(*active_param) {
                return call_chain_references(
                    document_uri,
                    point,
                    current_param,
                    vars,
                    imports,
                    class,
                    class_map,
                    reference_map,
                );
            }
            Err(ReferencesError::ArgumentNotFound)
        }
        Some(a) => unimplemented!("call_chain_revernece {:?}", a),
        None => Err(ReferencesError::ValidatedItemDoesNotExists),
    };
}
