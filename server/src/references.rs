use std::{
    fs::{self},
    hash::Hash,
};

use call_chain::CallItem;
use document::{ClassSource, Document, DocumentError};
use lsp_types::Location;
use parser::dto::{self, Class, ImportUnit};
use position::PositionSymbol;
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use tree_sitter::Point;
use tree_sitter_util::lsp::to_lsp_range;
use variables::LocalVariable;

use crate::definition::{self};

#[derive(Debug)]
pub enum ReferencesError {
    IoRead(String, std::io::Error),
    Position(position::PosionError),
    Treesitter(tree_sitter_util::TreesitterError),
    FindClassnameInClasspath(String),
    Tyres(tyres::TyresError),
    ValidatedItemDoesNotExists,
    ArgumentNotFound,
    Definition,
    Document(DocumentError),
}

#[derive(Debug)]
pub enum ReferenceUnit {
    Class(String),
    StaticClass(String),
}
#[derive(Debug)]
pub struct ReferencePosition(PositionSymbol);

pub struct ReferencesContext<'a> {
    pub point: &'a Point,
    pub imports: &'a [ImportUnit],
    pub class_map: &'a dashmap::DashMap<String, parser::dto::Class>,
    pub class: &'a dto::Class,
    pub vars: &'a [LocalVariable],
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
                        eprintln!("Referneces Uri error {e:?}");
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
    call_chain: &[CallItem],
    context: &ReferencesContext,
    reference_map: &dashmap::DashMap<String, Vec<ReferenceUnit>>,
    document_map: &dashmap::DashMap<String, Document>,
) -> Result<Vec<Location>, ReferencesError> {
    let (item, relevat) = call_chain::validate(call_chain, context.point);

    let reference_state = tyres::resolve_call_chain(
        &relevat,
        context.vars,
        context.imports,
        context.class,
        context.class_map,
    )
    .map_err(ReferencesError::Tyres)?;

    match relevat.get(item) {
        Some(CallItem::MethodCall { name, range: _ }) => {
            let mut locations = vec![];
            if let Some(used_in) = reference_map.get(&reference_state.class.class_path) {
                let used_in = used_in.value();
                for ref_unit in used_in {
                    let Some(class) = (match ref_unit {
                        ReferenceUnit::Class(c) => context.class_map.get(c),
                        ReferenceUnit::StaticClass(c) => context.class_map.get(c),
                    }) else {
                        continue;
                    };
                    let method_refs = method_references(&class, name, document_map)?;
                    let uri = definition::source_to_uri(&class.source).map_err(|e| {
                        eprintln!("Got into defintion error: {e:?}");
                        ReferencesError::Definition
                    })?;
                    locations.extend(
                        method_refs
                            .iter()
                            .map(|i| Location::new(uri.clone(), to_lsp_range(i.0.get_range()))),
                    );
                }
            }
            Ok(locations)
        }
        Some(CallItem::FieldAccess { name: _, range: _ }) => todo!(),
        Some(CallItem::Variable { name: _, range: _ }) => todo!(),
        Some(CallItem::ClassOrVariable { name: _, range: _ }) => todo!(),
        Some(CallItem::ArgumentList {
            prev: _,
            active_param,
            filled_params,
            range: _,
        }) => {
            if let Some(active_param) = active_param {
                if let Some(current_param) = filled_params.get(*active_param) {
                    return call_chain_references(
                        current_param,
                        context,
                        reference_map,
                        document_map,
                    );
                }
            }
            Err(ReferencesError::ArgumentNotFound)
        }
        Some(a) => unimplemented!("call_chain_references {a:?}"),
        None => Err(ReferencesError::ValidatedItemDoesNotExists),
    }
}

fn method_references(
    class: &Class,
    query_method_name: &str,
    document_map: &dashmap::DashMap<String, Document>,
) -> Result<Vec<ReferencePosition>, ReferencesError> {
    let uri = definition::source_to_uri(&class.source).map_err(|e| {
        eprintln!("Got into defintion error: {e:?}");
        ReferencesError::Definition
    })?;
    let uri = uri.to_string();
    let doc = document::read_document_or_open_class(
        &class.source,
        class.class_path.clone(),
        document_map,
        &uri,
    );
    match doc {
        ClassSource::Owned(doc) => {
            let o = match position::get_method_usage(doc.as_bytes(), query_method_name, &doc.tree) {
                Err(e) => Err(ReferencesError::Position(e))?,
                Ok(usages) => Ok(usages.into_iter().map(ReferencePosition).collect()),
            };
            document_map.insert(uri, doc);
            o
        }
        ClassSource::Ref(doc) => {
            match position::get_method_usage(doc.as_bytes(), query_method_name, &doc.tree) {
                Err(e) => Err(ReferencesError::Position(e))?,
                Ok(usages) => Ok(usages.into_iter().map(ReferencePosition).collect()),
            }
        }
        ClassSource::Err(e) => Err(ReferencesError::Document(e)),
    }
}

pub fn init_refernece_map(
    project_classes: &[Class],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
    reference_map: &dashmap::DashMap<String, Vec<ReferenceUnit>>,
) -> Result<(), ReferencesError> {
    project_classes.par_iter().for_each(|class| {
        let _ = reference_update_class(class, class_map, reference_map);
    });
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
        pos_refs_helper(tree, bytes, query_class_name)
    } else {
        match fs::read(&class.source) {
            Err(e) => Err(ReferencesError::IoRead(class.source.clone(), e)),
            Ok(bytes) => {
                let (_, tree) =
                    tree_sitter_util::parse(&bytes).map_err(ReferencesError::Treesitter)?;
                pos_refs_helper(&tree, &bytes, query_class_name)
            }
        }
    }
}

fn pos_refs_helper(
    tree: &tree_sitter::Tree,
    bytes: &[u8],
    query_class_name: &str,
) -> Result<Vec<ReferencePosition>, ReferencesError> {
    match position::get_type_usage(bytes, query_class_name, tree) {
        Err(e) => Err(ReferencesError::Position(e))?,
        Ok(usages) => Ok(usages.into_iter().map(ReferencePosition).collect()),
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
        .par_bridge()
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
                    ImportUnit::class_path_match_class_name(lclasspath, &class.name)
                }
                _ => false,
            })
        })
        .map(|i| i.0)
        .collect()
}
