use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ast::types::{AstFile, AstPoint};
use call_chain::CallItem;
use document::{Document, DocumentError};
use lsp_extra::{SourceToUriError, ToLspRangeError, source_to_uri, to_lsp_range};
use lsp_types::Location;
use my_string::MyString;
use parser::dto::{Class, ImportUnit};
use position::PositionSymbol;
use variables::LocalVariable;

use crate::definition::DefinitionError;

#[derive(Debug)]
pub enum ReferencesError {
    FindClassnameInClasspath(String),
    Tyres(tyres::TyresError),
    ValidatedItemDoesNotExists,
    ArgumentNotFound,
    Definition(DefinitionError),
    Document(DocumentError),
    ToLspRange(ToLspRangeError),
    SourceToUri(SourceToUriError),
    Locked,
}

#[derive(Debug)]
pub enum ReferenceUnit {
    Class(MyString),
    StaticClass(MyString),
}
#[derive(Debug)]
pub struct ReferencePosition(PositionSymbol);

pub struct ReferencesContext<'a> {
    pub point: &'a AstPoint,
    pub imports: &'a [ImportUnit],
    pub class_map: Arc<Mutex<HashMap<MyString, Class>>>,
    pub class: &'a Class,
    pub vars: &'a [LocalVariable],
}

#[must_use]
pub fn class_path(
    class_path: &str,
    reference_map: &Arc<Mutex<HashMap<MyString, Vec<ReferenceUnit>>>>,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    document_map: &Arc<Mutex<HashMap<MyString, Document>>>,
) -> Option<Vec<Location>> {
    if let Ok(class_map) = class_map.lock()
        && let Ok(reference_map) = reference_map.lock()
        && let Some(crefs) = reference_map.get(class_path)
        && let Ok(document_map) = document_map.lock()
    {
        let refs = crefs
            .iter()
            .filter_map(|i| match i {
                ReferenceUnit::Class(s) | ReferenceUnit::StaticClass(s) => class_map.get(s),
            })
            .filter_map(|i| document_map.get(&i.get_source()))
            .filter_map(|lookup| {
                let refs = pos_refs_helper(&lookup.ast, class_path);
                let a = refs.first().map(|i| i.0.range);
                a.map(|a| (lookup, a))
            })
            .filter_map(
                |(lookup, range)| match source_to_uri(lookup.path.to_str()?) {
                    Ok(u) => Some((u, range)),
                    Err(e) => {
                        eprintln!("References Uri error {e:?}");
                        None
                    }
                },
            )
            .filter_map(|(i, range)| {
                let range = to_lsp_range(&range).ok()?;
                Some(Location { uri: i, range })
            })
            .collect();
        return Some(refs);
    }
    None
}

pub fn call_chain_references(
    call_chain: &[CallItem],
    context: &ReferencesContext,
    reference_map: &Arc<Mutex<HashMap<MyString, Vec<ReferenceUnit>>>>,
    document_map: &Arc<Mutex<HashMap<MyString, Document>>>,
) -> Result<Vec<Location>, ReferencesError> {
    let (item, relevant) = call_chain::validate(call_chain, context.point);

    let reference_state = tyres::resolve_call_chain(
        &relevant,
        context.vars,
        context.imports,
        context.class,
        &context.class_map,
    )
    .map_err(ReferencesError::Tyres)?;

    match relevant.get(item) {
        Some(CallItem::MethodCall { name, range: _ }) => {
            let mut locations = vec![];
            if let Ok(reference_map) = reference_map.lock()
                && let Some(used_in) = reference_map.get(&reference_state.class.class_path)
                && let Ok(class_map) = context.class_map.lock()
            {
                for ref_unit in used_in {
                    let Some(class) = (match ref_unit {
                        ReferenceUnit::Class(c) | ReferenceUnit::StaticClass(c) => class_map.get(c),
                    }) else {
                        continue;
                    };
                    let method_refs = method_references(class, name, document_map)?;
                    let uri = source_to_uri(&class.get_source()).map_err(|e| {
                        eprintln!("Got into definition error: {e:?}");
                        ReferencesError::SourceToUri(e)
                    })?;
                    for i in method_refs {
                        let r = to_lsp_range(&i.0.range).map_err(ReferencesError::ToLspRange)?;
                        let loc = Location::new(uri.clone(), r);
                        locations.push(loc);
                    }
                }
            }

            Ok(locations)
        }
        Some(CallItem::ArgumentList {
            prev: _,
            active_param,
            filled_params,
            range: _,
        }) => {
            if let Some(active_param) = active_param
                && let Some(current_param) = filled_params.get(*active_param)
            {
                return call_chain_references(current_param, context, reference_map, document_map);
            }
            Err(ReferencesError::ArgumentNotFound)
        }
        Some(a) => unimplemented!("call_chain_references {a:?}"),
        None => Err(ReferencesError::ValidatedItemDoesNotExists),
    }
}

/// remove clippy when done
#[allow(clippy::nursery, clippy::pedantic)]
fn method_references(
    _class: &Class,
    _query_method_name: &str,
    _document_map: &Arc<Mutex<HashMap<MyString, Document>>>,
) -> Result<Vec<ReferencePosition>, ReferencesError> {
    //TODO
    Ok(Vec::new())
}

pub fn init_reference_map(
    project_classes: &[Class],
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    reference_map: &Arc<Mutex<HashMap<MyString, Vec<ReferenceUnit>>>>,
) -> Result<(), ReferencesError> {
    for class in project_classes {
        reference_update_class(class, class_map, reference_map)?;
    }
    Ok(())
}

pub fn reference_update_class(
    class: &Class,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    reference_map: &Arc<Mutex<HashMap<MyString, Vec<ReferenceUnit>>>>,
) -> Result<(), ReferencesError> {
    let class_path = class.class_path.clone();
    let Ok(mut reference_map) = reference_map.lock() else {
        return Err(ReferencesError::Locked);
    };
    for import in &class.imports {
        match import {
            ImportUnit::Package(p) | ImportUnit::Prefix(p) => {
                let implicit_imports = get_implicit_imports(class_map, class, p);
                for s in implicit_imports {
                    if let Some(a) = reference_map.get_mut(&s) {
                        a.push(ReferenceUnit::Class(class_path.clone()));
                    }
                }
            }
            ImportUnit::Class(s) => {
                if reference_map.contains_key(s) {
                    if let Some(a) = reference_map.get_mut(s) {
                        a.push(ReferenceUnit::Class(class.class_path.clone()));
                    }
                } else {
                    reference_map.insert(
                        s.clone(),
                        vec![ReferenceUnit::Class(class.class_path.clone())],
                    );
                }
            }
            ImportUnit::StaticClass(s) => {
                if reference_map.contains_key(s) {
                    if let Some(a) = reference_map.get_mut(s) {
                        a.push(ReferenceUnit::StaticClass(class.class_path.clone()));
                    }
                } else {
                    reference_map.insert(
                        s.clone(),
                        vec![ReferenceUnit::StaticClass(class.class_path.clone())],
                    );
                }
            }
            ImportUnit::StaticClassMethod(_, _) | ImportUnit::StaticPrefix(_) => (),
        }
    }
    Ok(())
}

fn pos_refs_helper(ast: &AstFile, query_class_name: &str) -> Vec<ReferencePosition> {
    let mut usages = vec![];
    position::get_class_position_ast(ast, Some(query_class_name), &mut usages);
    usages
        .into_iter()
        .map(ReferencePosition)
        .collect::<Vec<_>>()
}

fn get_implicit_imports(
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    class: &Class,
    package: &MyString,
) -> Vec<MyString> {
    let Ok(class_map) = class_map.lock() else {
        return Vec::new();
    };
    class_map
        .keys()
        .filter(|c| {
            if let Some((c_package, _)) = c.rsplit_once('.') {
                return c_package == package;
            }
            false
        })
        .map(|k| (k.clone(), class_map.get(k)))
        .filter(|(_, class)| class.is_some())
        .map(|(k, class)| (k, class.expect("Is some is checked in line before")))
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
