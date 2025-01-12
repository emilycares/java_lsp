use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use tree_sitter_util::get_string_node;

pub fn get_class_position(source: &str, method_name: &str) -> Vec<tree_sitter::Range> {
    get_item_position(
        source,
        "
        (class_declaration name: (identifier)@capture )
        (interface_declaration name: (identifier)@capture )
        (enum_declaration name: (identifier)@capture )
        (annotation_type_declaration name: (identifier)@capture )
        (record_declaration name: (identifier)@capture )
        ",
        method_name,
    )
}

pub fn get_method_position(source: &str, method_name: &str) -> Vec<tree_sitter::Range> {
    get_item_position(
        source,
        "(method_declaration name: (identifier)@capture )",
        method_name,
    )
}

pub fn get_filed_position(source: &str, method_name: &str) -> Vec<tree_sitter::Range> {
    get_item_position(
        source,
        "(field_declaration declarator: (variable_declarator name: (identifier)@capture ))",
        method_name,
    )
}

pub fn get_item_position(source: &str, query: &str, method_name: &str) -> Vec<tree_sitter::Range> {
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
            if cname == method_name {
                out.push(node.range());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use tree_sitter::{Point, Range};

    use crate::position::{get_class_position, get_filed_position, get_method_position};

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
        let out = get_method_position(content, "hello");
        assert_eq!(
            out,
            vec![Range {
                start_byte: 60,
                end_byte: 65,
                start_point: Point { row: 3, column: 16 },
                end_point: Point { row: 3, column: 21 },
            },]
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
        let out = get_filed_position(content, "a");
        assert_eq!(
            out,
            vec![Range {
                start_byte: 62,
                end_byte: 63,
                start_point: Point { row: 3, column: 18 },
                end_point: Point { row: 3, column: 19 },
            },]
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
            vec![Range {
                start_byte: 37,
                end_byte: 41,
                start_point: Point { row: 2, column: 13 },
                end_point: Point { row: 2, column: 17 },
            },]
        );
    }
}
