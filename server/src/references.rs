use std::{
    fs::{self},
    hash::Hash,
    str::Utf8Error,
};

use ast::types::{AstFile, AstPoint};
use call_chain::CallItem;
use document::{ClassSource, Document, DocumentError};
use lsp_types::Location;
use my_string::MyString;
use parser::dto::{self, Class, ImportUnit};
use position::PositionSymbol;
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use variables::LocalVariable;

use crate::{
    codeaction::ToLspRangeError,
    codeaction::to_lsp_range,
    definition::{self},
};

#[derive(Debug)]
pub enum ReferencesError {
    IoRead(MyString, std::io::Error),
    Utf8(Utf8Error),
    Lexer(ast::lexer::LexerError),
    Ast(ast::error::AstError),
    Position(position::PosionError),
    FindClassnameInClasspath(String),
    Tyres(tyres::TyresError),
    ValidatedItemDoesNotExists,
    ArgumentNotFound,
    Definition,
    Document(DocumentError),
    ToLspRange(ToLspRangeError),
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
    pub class_map: &'a dashmap::DashMap<MyString, parser::dto::Class>,
    pub class: &'a dto::Class,
    pub vars: &'a [LocalVariable],
}

#[must_use]
pub fn class_path(
    class_path: &str,
    reference_map: &dashmap::DashMap<MyString, Vec<ReferenceUnit>>,
    class_map: &dashmap::DashMap<MyString, parser::dto::Class>,
) -> Option<Vec<Location>> {
    if let Some(crefs) = reference_map.get(class_path) {
        let refs = crefs
            .iter()
            .filter_map(|i| match i {
                ReferenceUnit::Class(s) | ReferenceUnit::StaticClass(s) => class_map.get(s),
            })
            .filter_map(|lookup| {
                let refs = get_position_refrences(&lookup, class_path, None).ok()?;
                let a = refs.first().map(|i| i.0.get_range());
                a.map(|a| (lookup, *a))
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
    reference_map: &dashmap::DashMap<MyString, Vec<ReferenceUnit>>,
    document_map: &dashmap::DashMap<MyString, Document>,
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
                        ReferenceUnit::Class(c) | ReferenceUnit::StaticClass(c) => {
                            context.class_map.get(c)
                        }
                    }) else {
                        continue;
                    };
                    let method_refs = method_references(&class, name, document_map)?;
                    let uri = definition::source_to_uri(&class.source).map_err(|e| {
                        eprintln!("Got into defintion error: {e:?}");
                        ReferencesError::Definition
                    })?;
                    for i in method_refs {
                        let r =
                            to_lsp_range(i.0.get_range()).map_err(ReferencesError::ToLspRange)?;
                        let loc = Location::new(uri.clone(), r);
                        locations.push(loc);
                    }
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

fn method_references(
    class: &Class,
    query_method_name: &str,
    document_map: &dashmap::DashMap<MyString, Document>,
) -> Result<Vec<ReferencePosition>, ReferencesError> {
    let uri = definition::source_to_uri(&class.source).map_err(|e| {
        eprintln!("Got into defintion error: {e:?}");
        ReferencesError::Definition
    })?;
    let uri = uri.as_str();
    let doc = document::read_document_or_open_class(
        &class.source,
        class.class_path.clone(),
        document_map,
        uri,
    );
    match doc {
        ClassSource::Owned(doc) => {
            let o = match position::get_method_usage(doc.as_bytes(), query_method_name, &doc.ast) {
                Err(e) => Err(ReferencesError::Position(e))?,
                Ok(usages) => Ok(usages.into_iter().map(ReferencePosition).collect()),
            };
            document_map.insert(uri.into(), doc);
            o
        }
        ClassSource::Ref(doc) => {
            match position::get_method_usage(doc.as_bytes(), query_method_name, &doc.ast) {
                Err(e) => Err(ReferencesError::Position(e))?,
                Ok(usages) => Ok(usages.into_iter().map(ReferencePosition).collect()),
            }
        }
        ClassSource::Err(e) => Err(ReferencesError::Document(e)),
    }
}

pub fn init_refernece_map(
    project_classes: &[Class],
    class_map: &dashmap::DashMap<MyString, parser::dto::Class>,
    reference_map: &dashmap::DashMap<MyString, Vec<ReferenceUnit>>,
) -> Result<(), ReferencesError> {
    project_classes.par_iter().for_each(|class| {
        let _ = reference_update_class(class, class_map, reference_map);
    });
    Ok(())
}

pub fn reference_update_class(
    class: &Class,
    class_map: &dashmap::DashMap<MyString, Class>,
    reference_map: &dashmap::DashMap<MyString, Vec<ReferenceUnit>>,
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
            ImportUnit::StaticClassMethod(_, _) | ImportUnit::StaticPrefix(_) => (),
        }
    }
    Ok(())
}

fn get_position_refrences(
    class: &Class,
    query_class_path: &str,
    ast: Option<&AstFile>,
) -> Result<Vec<ReferencePosition>, ReferencesError> {
    let Some(query_class_name) = ImportUnit::class_path_get_class_name(query_class_path) else {
        return Err(ReferencesError::FindClassnameInClasspath(
            query_class_path.to_string(),
        ));
    };
    if let Some(ast) = ast {
        pos_refs_helper(ast, query_class_name)
    } else {
        match fs::read(&class.source) {
            Err(e) => Err(ReferencesError::IoRead(class.source.clone(), e)),
            Ok(bytes) => {
                let str = str::from_utf8(&bytes).map_err(ReferencesError::Utf8)?;
                let tokens = ast::lexer::lex(str).map_err(ReferencesError::Lexer)?;
                let ast = ast::parse_file(&tokens).map_err(ReferencesError::Ast)?;
                pos_refs_helper(&ast, query_class_name)
            }
        }
    }
}

fn pos_refs_helper(
    ast: &AstFile,
    query_class_name: &str,
) -> Result<Vec<ReferencePosition>, ReferencesError> {
    match position::get_class_position_ast(ast, Some(query_class_name)) {
        Err(e) => Err(ReferencesError::Position(e))?,
        Ok(usages) => Ok(usages.into_iter().map(ReferencePosition).collect()),
    }
}

fn insert_or_extend<K, V, A>(out: &dashmap::DashMap<K, V>, key: &K, insert: V)
where
    K: Eq + Hash + Clone,
    V: Extend<A> + IntoIterator<Item = A>,
{
    if out.contains_key(key) {
        if let Some(mut a) = out.get_mut(key) {
            a.extend(insert);
        }
    } else {
        out.insert(key.clone(), insert);
    }
}

fn get_implicit_imports(
    class_map: &dashmap::DashMap<MyString, Class>,
    class: &Class,
    package: &MyString,
) -> Vec<MyString> {
    class_map
        .clone()
        .into_read_only()
        .keys()
        .par_bridge()
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
