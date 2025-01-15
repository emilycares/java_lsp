use std::{fs::read_to_string, path::PathBuf, str::FromStr};

use lsp_types::{GotoDefinitionResponse, Location, Uri};
use parser::dto;
use tree_sitter::Point;

use crate::{
    call_chain::{self, CallItem},
    hover,
    imports::ImportUnit,
    position, tyres,
    utils::to_lsp_range,
    variable, Document,
};

pub fn class(
    document: &Document,
    document_uri: Uri,
    point: &Point,
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<GotoDefinitionResponse> {
    let vars = variable::get_vars(document, point);

    if let Some((class, _range)) = hover::class_action(document, point, imports, class_map) {
        if let Some(sourc_file) = get_source_content(&class) {
            let ranges = position::get_class_position(sourc_file.as_str(), &class.name);
            if let Some(value) = go_to_definition_range(class, ranges) {
                return value;
            }
        }
    }

    if let Some(value) =
        call_chain_definition(document, document_uri, point, &vars, imports, class_map)
    {
        return value;
    }

    None
}

fn call_chain_definition(
    document: &Document,
    document_uri: Uri,
    point: &Point,
    vars: &[variable::LocalVariable],
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<String, dto::Class>,
) -> Option<Option<GotoDefinitionResponse>> {
    if let Some(call_chain) = call_chain::get_call_chain(document, point) {
        let Some((item, relevat)) = call_chain::validate(&call_chain, point) else {
            return None;
        };
        if let Some(extend_class) = tyres::resolve_call_chain(relevat, vars, imports, class_map) {
            match call_chain.get(item) {
                Some(CallItem::MethodCall { name, range: _ }) => {
                    if let Some(sourc_file) = get_source_content(&extend_class) {
                        let ranges = position::get_method_position(sourc_file.as_str(), name);
                        if let Some(value) = go_to_definition_range(extend_class, ranges) {
                            return Some(value);
                        }
                    }
                }
                Some(CallItem::FieldAccess { name, range: _ }) => {
                    if let Some(sourc_file) = get_source_content(&extend_class) {
                        let ranges = position::get_filed_position(sourc_file.as_str(), name);
                        if let Some(value) = go_to_definition_range(extend_class, ranges) {
                            return Some(value);
                        }
                    }
                }
                Some(CallItem::Variable { name, range: _ }) => {
                    let Some(range) = vars.iter().find(|n| n.name == *name).map(|v| v.range) else {
                        return None;
                    };
                    eprint!("var def found {:?}", &document_uri);
                    return Some(Some(GotoDefinitionResponse::Scalar(Location {
                        uri: document_uri,
                        range: to_lsp_range(range),
                    })));
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
    ranges: Vec<tree_sitter::Range>,
) -> Option<Option<GotoDefinitionResponse>> {
    let uri = Uri::from_str(&format!("file:/{}", extend_class.source)).ok()?;
    match ranges.len() {
        0 => Some(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: lsp_types::Range::default(),
        }))),
        1 => Some(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: to_lsp_range(*ranges.first()?),
        }))),
        2.. => {
            let locations = ranges
                .iter()
                .map(|r| to_lsp_range(*r))
                .map(|r| Location {
                    uri: uri.clone(),
                    range: r,
                })
                .collect();
            Some(Some(GotoDefinitionResponse::Array(locations)))
        }
    }
}
