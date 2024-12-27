use parser::dto;
use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Position, Range, Url};
use tree_sitter::Point;

use crate::{hover, Document};

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

    if let Some((class, _range)) = hover::class_action(tree, bytes, point, imports, class_map) {
        return class_to_definition(&class);
    }

    None
}

pub fn class_to_definition(c: &dto::Class) -> Option<GotoDefinitionResponse> {
    let range = Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 0,
        },
    };
    let uri = Url::parse(&format!("file:/{}", c.source)).unwrap();
    return Some(GotoDefinitionResponse::Scalar(Location { uri, range }));
}
