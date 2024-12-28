use std::{fs::read_to_string, path::PathBuf};

use parser::dto;
use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Range, Url};
use tree_sitter::Point;

use crate::{
    call_chain::{get_call_chain, CallItem},
    hover, position, tyres,
    utils::to_lsp_range,
    variable, Document,
};

pub fn class(
    document: &Document,
    point: &Point,
    imports: &[&str],
    class_map: &dashmap::DashMap<std::string::String, parser::dto::Class>,
) -> Option<GotoDefinitionResponse> {
    let tree = &document.tree;
    let bytes = document
        .text
        .slice(..)
        .as_str()
        .unwrap_or_default()
        .as_bytes();
    let vars = variable::get_vars(document, &point);

    if let Some((class, _range)) = hover::class_action(tree, bytes, point, imports, class_map) {
        if let Some(sourc_file) = get_source_content(&class) {
            if let Some(range) = position::get_class_position(sourc_file.as_str(), &class.name) {
                if let Some(value) = go_to_definition_range(class, to_lsp_range(range)) {
                    return value;
                }
            } else {
                // When the range could not be found. Go to top of the file
                if let Some(value) = go_to_definition_range(class, Range::default()) {
                    return value;
                }
            }
        }
    }

    if let Some(value) = call_chain_definition(document, point, &vars, imports, class_map) {
        return value;
    }

    None
}

fn call_chain_definition(
    document: &Document,
    point: &Point,
    vars: &Vec<variable::LocalVariable>,
    imports: &[&str],
    class_map: &dashmap::DashMap<String, dto::Class>,
) -> Option<Option<GotoDefinitionResponse>> {
    if let Some(call_chain) = get_call_chain(document, point).as_deref() {
        if let Some(extend_class) = tyres::resolve_call_chain(call_chain, &vars, imports, class_map)
        {
            match call_chain.last() {
                Some(CallItem::MethodCall(name)) => {
                    if let Some(sourc_file) = get_source_content(&extend_class) {
                        if let Some(range) =
                            position::get_method_position(sourc_file.as_str(), name)
                        {
                            if let Some(value) =
                                go_to_definition_range(extend_class, to_lsp_range(range))
                            {
                                return Some(value);
                            }
                        }
                    }
                }
                Some(CallItem::FieldAccess(name)) => {
                    if let Some(sourc_file) = get_source_content(&extend_class) {
                        if let Some(range) = position::get_filed_position(sourc_file.as_str(), name)
                        {
                            if let Some(value) =
                                go_to_definition_range(extend_class, to_lsp_range(range))
                            {
                                return Some(value);
                            }
                        }
                    }
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
    range: Range,
) -> Option<Option<GotoDefinitionResponse>> {
    let uri = Url::parse(&format!("file:/{}", extend_class.source)).unwrap();
    return Some(Some(GotoDefinitionResponse::Scalar(Location {
        uri,
        range,
    })));
}
