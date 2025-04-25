use lsp_types::{Location, SymbolInformation, SymbolKind, Uri};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Query, QueryCursor, Tree};
use tree_sitter_util::get_string_node;

use crate::utils::to_lsp_range;

#[derive(Debug, PartialEq)]
pub enum PosionError {
    Treesitter(tree_sitter_util::TreesitterError),
}

pub fn get_class_position(bytes: &[u8], name: &str) -> Result<Vec<PositionSymbol>, PosionError> {
    let (_, tree) = tree_sitter_util::parse(bytes).map_err(PosionError::Treesitter)?;
    get_item_ranges(
        &tree,
        bytes,
        "
        (class_declaration name: (identifier)@capture )
        (interface_declaration name: (identifier)@capture )
        (enum_declaration name: (identifier)@capture )
        (annotation_type_declaration name: (identifier)@capture )
        (record_declaration name: (identifier)@capture )
        ",
        Some(name),
    )
}

pub fn get_method_positions(bytes: &[u8], name: &str) -> Result<Vec<PositionSymbol>, PosionError> {
    let (_, tree) = tree_sitter_util::parse(bytes).map_err(PosionError::Treesitter)?;
    get_item_ranges(
        &tree,
        bytes,
        "(method_declaration name: (identifier)@capture )",
        Some(name),
    )
}

pub fn get_field_positions(bytes: &[u8], name: &str) -> Result<Vec<PositionSymbol>, PosionError> {
    let (_, tree) = tree_sitter_util::parse(bytes).map_err(PosionError::Treesitter)?;
    get_item_ranges(
        &tree,
        bytes,
        "(field_declaration declarator: (variable_declarator name: (identifier)@capture ))",
        Some(name),
    )
}

#[derive(Debug, PartialEq)]
pub enum PositionSymbol {
    Range(tree_sitter::Range),
    Symbol {
        range: tree_sitter::Range,
        name: String,
        kind: String,
    },
}

impl PositionSymbol {
    pub fn get_range(&self) -> tree_sitter::Range {
        match self {
            PositionSymbol::Symbol {
                range,
                name: _,
                kind: _,
            } => *range,
            PositionSymbol::Range(range) => *range,
        }
    }
}

pub fn get_symbols(bytes: &[u8]) -> Result<Vec<PositionSymbol>, PosionError> {
    let (_, tree) = tree_sitter_util::parse(bytes).map_err(PosionError::Treesitter)?;
    get_item_ranges(
        &tree,
        bytes,
        "
        (class_declaration name: (identifier)@capture )
        (interface_declaration name: (identifier)@capture )
        (enum_declaration name: (identifier)@capture )
        (annotation_type_declaration name: (identifier)@capture )
        (record_declaration name: (identifier)@capture )
        ",
        None,
    )
}

pub fn get_type_usage(
    bytes: &[u8],
    query_class_name: &str,
    tree: &Tree,
) -> Result<Vec<PositionSymbol>, PosionError> {
    get_item_ranges(
        tree,
        bytes,
        "
        (type_identifier)@capture
        (field_access object: (identifier)@capture )
        (method_invocation object: (identifier)@capture )
        ",
        Some(query_class_name),
    )
}

pub fn symbols_to_document_symbols(
    symbols: Vec<PositionSymbol>,
    uri: Uri,
) -> Vec<SymbolInformation> {
    symbols
        .iter()
        .filter_map(|r| match r {
            PositionSymbol::Range(_) => None,
            PositionSymbol::Symbol { range, name, kind } =>
            {
                #[allow(deprecated)]
                Some(SymbolInformation {
                    name: name.to_string(),
                    kind: match kind.as_str() {
                        "formal_parameter" => SymbolKind::FIELD,
                        "variable_declarator" => SymbolKind::FIELD,
                        "method_declaration" => SymbolKind::METHOD,
                        "class_declaration" => SymbolKind::CLASS,
                        _ => SymbolKind::FIELD,
                    },
                    tags: Some(vec![]),
                    deprecated: None,
                    location: Location {
                        uri: uri.clone(),
                        range: to_lsp_range(*range),
                    },
                    container_name: None,
                })
            }
        })
        .collect()
}
pub fn get_item_ranges(
    tree: &Tree,
    bytes: &[u8],
    query: &str,
    name: Option<&str>,
) -> Result<Vec<PositionSymbol>, PosionError> {
    let query =
        Query::new(&tree_sitter_java::LANGUAGE.into(), query).expect("Query should be okey");
    let mut cursor = QueryCursor::new();
    let mut matchtes = cursor.matches(&query, tree.root_node(), bytes);

    let mut out = vec![];

    while let Some(m) = matchtes.next() {
        for capture in m.captures {
            let node = capture.node;

            let cname = get_string_node(&node, bytes);
            if let Some(name) = name {
                if cname == name {
                    out.push(PositionSymbol::Range(node.range()));
                }
            } else if let Some(parent) = node.parent() {
                out.push(PositionSymbol::Symbol {
                    range: node.range(),
                    name: cname,
                    kind: parent.kind().to_string(),
                });
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use tree_sitter::{Point, Range};

    use crate::position::{
        get_class_position, get_field_positions, get_method_positions, get_type_usage,
        PositionSymbol,
    };

    #[test]
    fn method_pos_base() {
        let content = b"
package ch.emilycares;
public class Test {
    public void hello() {
        if (a == b ) {
        }
        return;
    }
}
";
        let out = get_method_positions(content, "hello");
        assert_eq!(
            out,
            Ok(vec![PositionSymbol::Range(Range {
                start_byte: 60,
                end_byte: 65,
                start_point: Point { row: 3, column: 16 },
                end_point: Point { row: 3, column: 21 },
            }),])
        );
    }

    #[test]
    fn field_pos_base() {
        let content = b"
package ch.emilycares;
public class Test {
    public String a;
}
";
        let out = get_field_positions(content, "a");
        assert_eq!(
            out,
            Ok(vec![PositionSymbol::Range(Range {
                start_byte: 62,
                end_byte: 63,
                start_point: Point { row: 3, column: 18 },
                end_point: Point { row: 3, column: 19 },
            }),])
        );
    }

    #[test]
    fn class_pos_base() {
        let content = b"
package ch.emilycares;
public class Test {}
";
        let out = get_class_position(content, "Test");
        assert_eq!(
            out,
            Ok(vec![PositionSymbol::Range(Range {
                start_byte: 37,
                end_byte: 41,
                start_point: Point { row: 2, column: 13 },
                end_point: Point { row: 2, column: 17 },
            }),])
        );
    }
    #[test]
    fn type_usage_base() {
        let content = br#"
package ch.emilycares;
public class Test {
private StringBuilder sb = new StringBuilder();
}
"#;
        let (_, tree) = tree_sitter_util::parse(content).unwrap();
        let out = get_type_usage(content, "StringBuilder", &tree);

        assert_eq!(out.unwrap().len(), 2);
    }
}
