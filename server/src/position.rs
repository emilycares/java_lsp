use lsp_types::{Location, SymbolInformation, SymbolKind, Uri};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use tree_sitter_util::get_string_node;

use crate::utils::to_lsp_range;

pub fn get_class_position(source: &str, name: &str) -> Vec<PositionSymbol> {
    get_item_ranges(
        source,
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

pub fn get_method_positions(source: &str, name: &str) -> Vec<PositionSymbol> {
    get_item_ranges(
        source,
        "(method_declaration name: (identifier)@capture )",
        Some(name),
    )
}

pub fn get_filed_positions(source: &str, name: &str) -> Vec<PositionSymbol> {
    get_item_ranges(
        source,
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

pub fn get_symbols(source: &str) -> Vec<PositionSymbol> {
    get_item_ranges(
        source,
        "
(field_declaration
  declarator: (variable_declarator
    name: (identifier)@varname))

(local_variable_declaration
  declarator: (variable_declarator
    name: (identifier)@varname))

(enhanced_for_statement
  name: (identifier)@varname)

(formal_parameter
  name: (identifier)@varname)

(method_declaration
  name: (identifier) @method)
  
(class_declaration
  name: (identifier) @class)
",
        None,
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
pub fn get_item_ranges<'a>(
    source: &'a str,
    query: &'a str,
    name: Option<&str>,
) -> Vec<PositionSymbol> {
    let language = tree_sitter_java::LANGUAGE;
    let mut parser = Parser::new();
    if parser.set_language(&language.into()).is_err() {
        eprintln!("----- Not initialized -----");
        return vec![];
    }
    let Some(tree) = parser.parse(source, None) else {
        return vec![];
    };
    let query =
        Query::new(&tree_sitter_java::LANGUAGE.into(), query).expect("Query should be okey");
    let bytes = source.as_bytes();
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
            } else {
                if let Some(parent) = node.parent() {
                    out.push(PositionSymbol::Symbol {
                        range: node.range(),
                        name: cname,
                        kind: parent.kind().to_string(),
                    });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use tree_sitter::{Point, Range};

    use crate::position::{
        get_class_position, get_filed_positions, get_method_positions, PositionSymbol,
    };

    #[test]
    fn method_pos_base() {
        let content = "
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
            vec![PositionSymbol::Range(Range {
                start_byte: 60,
                end_byte: 65,
                start_point: Point { row: 3, column: 16 },
                end_point: Point { row: 3, column: 21 },
            }),]
        );
    }

    #[test]
    fn field_pos_base() {
        let content = "
package ch.emilycares;
public class Test {
    public String a;
}
";
        let out = get_filed_positions(content, "a");
        assert_eq!(
            out,
            vec![PositionSymbol::Range(Range {
                start_byte: 62,
                end_byte: 63,
                start_point: Point { row: 3, column: 18 },
                end_point: Point { row: 3, column: 19 },
            }),]
        );
    }

    #[test]
    fn class_pos_base() {
        let content = "
package ch.emilycares;
public class Test {}
";
        let out = get_class_position(content, "Test");
        assert_eq!(
            out,
            vec![PositionSymbol::Range(Range {
                start_byte: 37,
                end_byte: 41,
                start_point: Point { row: 2, column: 13 },
                end_point: Point { row: 2, column: 17 },
            }),]
        );
    }
}
