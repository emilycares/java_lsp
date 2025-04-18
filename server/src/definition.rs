use std::{fs::read_to_string, path::PathBuf, str::FromStr};

use lsp_types::{GotoDefinitionResponse, Location, Uri};
use parser::{
    call_chain::{self, CallItem},
    dto::{self, ImportUnit},
};
use tree_sitter::Point;

use crate::{
    hover::{class_action, ClassActionError},
    position::{self, PositionSymbol},
    tyres::{self, TyresError},
    utils::to_lsp_range,
    variable::{self, LocalVariable},
    Document,
};

#[derive(Debug)]
#[allow(dead_code)]
pub enum DefinitionError {
    Tyres(TyresError),
    ClassActon(ClassActionError),
    NoSourceFile { file: String },
    UriInvalid { uri: String, error: String },
    LocalVariableNotFound { name: String },
    ValidatedItemDoesNotExists,
    NoCallChain,
    Position(position::PosionError),
    ArgumentNotFound,
}

pub fn class(
    document: &Document,
    document_uri: &Uri,
    point: &Point,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Result<GotoDefinitionResponse, DefinitionError> {
    let tree = &document.tree;
    let bytes = document.as_bytes();

    match class_action(tree, bytes, point, lo_va, imports, class_map) {
        Ok((class, _range)) => {
            let source_file = get_source_content(&class)?;
            let ranges = position::get_class_position(source_file.as_bytes(), &class.name)
                .map_err(|e| DefinitionError::Position(e))?;
            let uri = class_to_uri(&class)?;
            return Ok(go_to_definition_range(uri, ranges));
        }
        Err(ClassActionError::VariableFound { var, range: _ }) => {
            Ok(GotoDefinitionResponse::Scalar(Location {
                uri: document_uri.clone(),
                range: to_lsp_range(var.range),
            }))
        }
        Err(e) => Err(DefinitionError::ClassActon(e)),
    }
}

pub fn call_chain_definition(
    document_uri: Uri,
    point: &Point,
    call_chain: &Vec<CallItem>,
    vars: &[variable::LocalVariable],
    imports: &[ImportUnit],
    class: &dto::Class,
    class_map: &dashmap::DashMap<String, dto::Class>,
) -> Result<GotoDefinitionResponse, DefinitionError> {
    let (item, relevat) = call_chain::validate(&call_chain, point);

    let extend_class = tyres::resolve_call_chain(relevat, vars, imports, class, class_map)
        .map_err(|e| DefinitionError::Tyres(e))?;
    return match relevat.get(item) {
        Some(CallItem::MethodCall { name, range: _ }) => {
            let source_file = get_source_content(&extend_class)?;
            let ranges = position::get_method_positions(source_file.as_bytes(), name)
                .map_err(|e| DefinitionError::Position(e))?;
            let uri = class_to_uri(&extend_class)?;
            return Ok(go_to_definition_range(uri, ranges));
        }
        Some(CallItem::FieldAccess { name, range: _ }) => {
            let source_file = get_source_content(&extend_class)?;
            let ranges = position::get_field_positions(source_file.as_bytes(), name)
                .map_err(|e| DefinitionError::Position(e))?;
            let uri = class_to_uri(&extend_class)?;
            return Ok(go_to_definition_range(uri, ranges));
        }
        Some(CallItem::Variable { name, range: _ }) => {
            let Some(range) = vars.iter().find(|n| n.name == *name).map(|v| v.range) else {
                return Err(DefinitionError::LocalVariableNotFound {
                    name: name.to_owned(),
                });
            };
            return Ok(GotoDefinitionResponse::Scalar(Location {
                uri: document_uri,
                range: to_lsp_range(range),
            }));
        }
        Some(CallItem::ClassOrVariable { name, range: _ }) => {
            let ranges: Vec<_> = vars
                .into_iter()
                .filter(|n| n.name == *name)
                .map(|v| PositionSymbol::Range(v.range))
                .collect();

            Ok(go_to_definition_range(document_uri, ranges))
        }
        Some(CallItem::ArgumentList {
            prev: _,
            active_param,
            filled_params,
            range: _,
        }) => {
            if let Some(current_param) = filled_params.get(*active_param) {
                return call_chain_definition(
                    document_uri,
                    point,
                    current_param,
                    vars,
                    imports,
                    class,
                    class_map,
                );
            }
            Err(DefinitionError::ArgumentNotFound)
        }
        Some(a) => unimplemented!("call_chain_definition {:?}", a),
        None => Err(DefinitionError::ValidatedItemDoesNotExists),
    };
}

fn get_source_content(extend_class: &dto::Class) -> Result<String, DefinitionError> {
    let path = PathBuf::from(&extend_class.source);
    eprintln!("Loading source -> {}", &extend_class.source);
    if path.exists() {
        if let Ok(sourc_file) = read_to_string(path) {
            return Ok(sourc_file);
        }
    }
    Err(DefinitionError::NoSourceFile {
        file: extend_class.source.clone(),
    })
}

pub fn class_to_uri(class: &dto::Class) -> Result<Uri, DefinitionError> {
    let str_uri = format!("file://{}", class.source.replace("\\", "/"));
    let uri = Uri::from_str(&str_uri);
    match uri {
        Ok(uri) => Ok(uri),
        Err(e) => Err(DefinitionError::UriInvalid {
            uri: str_uri,
            error: format!("{e:?}"),
        }),
    }
}

fn go_to_definition_range(uri: Uri, ranges: Vec<PositionSymbol>) -> GotoDefinitionResponse {
    match ranges.len() {
        0 => GotoDefinitionResponse::Scalar(Location {
            uri,
            range: lsp_types::Range::default(),
        }),
        1 => GotoDefinitionResponse::Scalar(Location {
            uri,
            range: to_lsp_range(ranges.first().expect("Length is 1").get_range()),
        }),
        2.. => {
            let locations = ranges
                .iter()
                .map(|r| to_lsp_range(r.get_range()))
                .map(|r| Location {
                    uri: uri.clone(),
                    range: r,
                })
                .collect();
            GotoDefinitionResponse::Array(locations)
        }
    }
}
