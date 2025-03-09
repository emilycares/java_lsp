use std::{fs::read_to_string, path::PathBuf, str::FromStr};

use lsp_types::{GotoDefinitionResponse, Location, Uri};
use parser::{
    call_chain::{self, CallItem},
    dto,
};
use tree_sitter::Point;

use crate::{
    hover,
    imports::ImportUnit,
    position::{self, PositionSymbol},
    tyres,
    utils::to_lsp_range,
    variable, Document,
};

pub fn class(
    document: &Document,
    point: &Point,
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<GotoDefinitionResponse> {
    let tree = &document.tree;
    let bytes = document.as_bytes();

    if let Some((class, _range)) = hover::class_action(tree, bytes, point, imports, class_map) {
        if let Some(sourc_file) = get_source_content(&class) {
            let ranges = position::get_class_position(sourc_file.as_str(), &class.name);
            let o = go_to_definition_range(class, ranges);
            return o;
        }
    }

    None
}

pub fn call_chain_definition(
    document: &Document,
    document_uri: Uri,
    point: &Point,
    vars: &[variable::LocalVariable],
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<String, dto::Class>,
) -> Option<GotoDefinitionResponse> {
    if let Some(call_chain) = call_chain::get_call_chain(&document.tree, document.as_bytes(), point)
    {
        let Some((item, relevat)) = call_chain::validate(&call_chain, point) else {
            return None;
        };
        if let Some(extend_class) = tyres::resolve_call_chain(relevat, vars, imports, class_map) {
            match call_chain.get(item) {
                Some(CallItem::MethodCall { name, range: _ }) => {
                    if let Some(sourc_file) = get_source_content(&extend_class) {
                        let ranges = position::get_method_positions(sourc_file.as_str(), name);
                        if let Some(value) = go_to_definition_range(extend_class, ranges) {
                            return Some(value);
                        }
                    }
                }
                Some(CallItem::FieldAccess { name, range: _ }) => {
                    if let Some(sourc_file) = get_source_content(&extend_class) {
                        let ranges = position::get_filed_positions(sourc_file.as_str(), name);
                        if let Some(value) = go_to_definition_range(extend_class, ranges) {
                            return Some(value);
                        }
                    }
                }
                Some(CallItem::Variable { name, range: _ }) => {
                    let Some(range) = vars.iter().find(|n| n.name == *name).map(|v| v.range) else {
                        return None;
                    };
                    return Some(GotoDefinitionResponse::Scalar(Location {
                        uri: document_uri,
                        range: to_lsp_range(range),
                    }));
                }
                Some(_) => {}
                None => {}
            };
        }
    }
    None
}

fn get_source_content(extend_class: &dto::Class) -> Option<String> {
    let path = PathBuf::from(&extend_class.source);
    if path.exists() {
        if let Ok(sourc_file) = read_to_string(path) {
            return Some(sourc_file);
        }
    }
    None
}

fn go_to_definition_range(
    extend_class: dto::Class,
    ranges: Vec<PositionSymbol>,
) -> Option<GotoDefinitionResponse> {
    let uri = format!("file://{}", extend_class.source);
    let uri = Uri::from_str(&uri);
    let uri = uri.ok()?;
    match ranges.len() {
        0 => Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: lsp_types::Range::default(),
        })),
        1 => Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: to_lsp_range(ranges.first()?.get_range()),
        })),
        2.. => {
            let locations = ranges
                .iter()
                .map(|r| to_lsp_range(r.get_range()))
                .map(|r| Location {
                    uri: uri.clone(),
                    range: r,
                })
                .collect();
            Some(GotoDefinitionResponse::Array(locations))
        }
    }
}
