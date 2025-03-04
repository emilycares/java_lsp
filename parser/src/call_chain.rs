use tree_sitter::{Point, Range};
use tree_sitter_util::{get_string, get_string_node, CommentSkiper};

#[derive(Debug, PartialEq, Clone)]
pub enum CallItem {
    MethodCall {
        name: String,
        range: Range,
    },
    FieldAccess {
        name: String,
        range: Range,
    },
    Variable {
        name: String,
        range: Range,
    },
    Class {
        name: String,
        range: Range,
    },
    ClassOrVariable {
        name: String,
        range: Range,
    },
    ArgumentList {
        prev: Vec<CallItem>,
        active_param: usize,
        filled_params: Vec<Vec<CallItem>>,
        range: Range,
    },
}

#[derive(Debug, PartialEq, Clone)]
struct Argument {
    range: Option<Range>,
    value: Vec<CallItem>,
}

/// Provides data abuilt the current variable before the cursor
/// ``` java
/// Long other = 1l;
/// other.
///       ^
/// ```
/// Then it would return info about the variable other
pub fn get_call_chain(
    tree: &tree_sitter::Tree,
    bytes: &[u8],
    point: &Point,
) -> Option<Vec<CallItem>> {
    let mut cursor = tree.walk();
    let mut level = 0;
    let mut out = vec![];
    loop {
        match cursor.node().kind() {
            "argument_list" => {
                let (after, value) = parse_argument_list(&out, &mut cursor, bytes, point);
                out.clear();
                out.push(value);
                if let Some(after) = after {
                    out.extend(after.clone());
                }
                cursor.parent();
            }
            "scoped_type_identifier" => {
                let node = cursor.node();

                if let Some(c) = class_or_variable(node, bytes) {
                    out.push(c);
                }
            }
            "template_expression" => {
                cursor.first_child();
                if cursor.node().kind() == "method_invocation" {
                    out.extend(parse_method_invocation(cursor.node(), bytes));
                }
                cursor.parent();
            }
            "expression_statement" => {
                cursor.first_child();
                out.extend(parse_value(&cursor, bytes));
                cursor.parent();
            }
            "return_statement" => {
                cursor.first_child();
                cursor.sibling();
                out.extend(parse_value(&cursor, bytes));
                cursor.parent();
            }
            "local_variable_declaration" => {
                cursor.first_child();
                cursor.sibling();
                if cursor.node().kind() == "variable_declarator" {
                    cursor.first_child();
                    cursor.sibling();
                    cursor.sibling();
                    out.extend(parse_value(&cursor, bytes));
                    cursor.parent();
                }
                cursor.parent();
            }
            "parenthesized_expression" => {
                cursor.first_child();
                cursor.sibling();
                cursor.goto_first_child_for_point(Point::new(point.row, point.column - 3));
                out.extend(parse_value(&cursor, bytes));
                cursor.parent();
            }
            _ => {}
        }

        let n = cursor.goto_first_child_for_point(*point);
        level += 1;
        if n.is_none() {
            break;
        }
        if level >= 200 {
            break;
        }
    }
    if !out.is_empty() {
        return Some(out);
    }
    None
}

pub fn parse_argument_list(
    out: &Vec<CallItem>,
    cursor: &mut tree_sitter::TreeCursor<'_>,
    bytes: &[u8],
    point: &Point,
) -> (Option<Vec<CallItem>>, CallItem) {
    let mut active_param = 0;
    let mut current_param = 0;

    let arg_prev = out.clone();
    let arg_range = cursor.node().range();
    let mut filled_params = vec![];
    cursor.first_child();
    let Argument { range, value } = parse_argument_list_argument(cursor, bytes);
    filled_params.push(value.clone());
    if tree_sitter_util::is_point_in_range(point, &range.unwrap()) {
        active_param = current_param;
    }
    while cursor.node().kind() == "," {
        current_param += 1;
        let Argument { range, value } = parse_argument_list_argument(cursor, bytes);
        filled_params.push(value.clone());
        if tree_sitter_util::is_point_in_range(point, &range.unwrap()) {
            active_param = current_param;
        }
    }
    let value = CallItem::ArgumentList {
        prev: arg_prev,
        active_param,
        filled_params: filled_params.clone(),
        range: arg_range,
    };
    let after = filled_params.get(active_param).cloned();
    (after, value)
}

fn parse_argument_list_argument(
    cursor: &mut tree_sitter::TreeCursor<'_>,
    bytes: &[u8],
) -> Argument {
    let mut out = Argument {
        range: None,
        value: vec![],
    };
    cursor.sibling();
    out.range = Some(cursor.node().range());
    out.value.extend(parse_value(cursor, bytes));
    if cursor.sibling() {
        let kind = cursor.node().kind();
        if kind == "," || kind == ")" {
            out.range = Some(tree_sitter_util::add_ranges(
                out.range.unwrap(),
                cursor.node().range(),
            ));
        }
    }

    out
}

fn parse_value(cursor: &tree_sitter::TreeCursor<'_>, bytes: &[u8]) -> Vec<CallItem> {
    match cursor.node().kind() {
        "identifier" => {
            if let Some(c) = class_or_variable(cursor.node(), bytes) {
                return vec![c];
            }
            vec![]
        }
        "field_access" => parse_field_access(cursor.node(), bytes),
        "method_invocation" => parse_method_invocation(cursor.node(), bytes),
        "object_creation_expression" => parse_object_creation(cursor.node(), bytes),
        "string_literal" => vec![CallItem::Class {
            name: "String".to_string(),
            range: cursor.node().range(),
        }],
        _ => vec![],
    }
}

fn parse_method_invocation(node: tree_sitter::Node<'_>, bytes: &[u8]) -> Vec<CallItem> {
    let mut cursor = node.walk();
    let mut out = vec![];
    cursor.first_child();
    match cursor.node().kind() {
        "field_access" => {
            out.extend(parse_field_access(node, bytes));
            cursor.sibling();
            cursor.sibling();
            let method_name = get_string(&cursor, bytes);
            out.extend(vec![CallItem::MethodCall {
                name: method_name,
                range: cursor.node().range(),
            }]);
        }
        "identifier" => {
            let var_name = cursor.node();
            if let Some(c) = class_or_variable(var_name, bytes) {
                out.push(c);
            }
            cursor.sibling();
            cursor.sibling();
            let method_name = get_string(&cursor, bytes);
            out.push(CallItem::MethodCall {
                name: method_name,
                range: cursor.node().range(),
            });
        }
        "method_invocation" => {
            out.extend(parse_method_invocation(cursor.node(), bytes));
        }
        "object_creation_expression" => {
            out.extend(parse_object_creation(node, bytes));
            cursor.sibling();
            cursor.sibling();
            let method_name = get_string(&cursor, bytes);
            out.push(CallItem::MethodCall {
                name: method_name,
                range: cursor.node().range(),
            });
        }
        _ => {}
    };
    cursor.parent();
    out
}

fn parse_field_access(node: tree_sitter::Node<'_>, bytes: &[u8]) -> Vec<CallItem> {
    let mut cursor = node.walk();
    let mut out = vec![];
    cursor.first_child();
    match cursor.node().kind() {
        "identifier" => {
            let var_name = cursor.node();
            if let Some(item) = class_or_variable(var_name, bytes) {
                out.push(item);
            }
            cursor.sibling();
            if cursor.sibling() {
                let field_name = get_string(&cursor, bytes);
                if field_name != "return" {
                    out.push(CallItem::FieldAccess {
                        name: field_name,
                        range: cursor.node().range(),
                    });
                }
            }
        }
        "string_literal" => {
            let val = parse_value(&cursor, bytes);
            out.extend(val);
        }
        "method_invocation" => {
            out.extend(parse_method_invocation(cursor.node(), bytes));
            cursor.sibling();
            if cursor.sibling() {
                let field_name = get_string(&cursor, bytes);
                out.push(CallItem::FieldAccess {
                    name: field_name,
                    range: cursor.node().range(),
                });
            }
        }
        "field_access" => {
            out.extend(parse_field_access(cursor.node(), bytes));
        }
        "object_creation_expression" => {
            out.extend(parse_object_creation(cursor.node(), bytes));
            cursor.sibling();
            cursor.sibling();
            let field_name = get_string(&cursor, bytes);
            out.push(CallItem::FieldAccess {
                name: field_name,
                range: cursor.node().range(),
            });
        }
        _ => {}
    }

    cursor.parent();
    out
}
fn parse_object_creation(node: tree_sitter::Node<'_>, bytes: &[u8]) -> Vec<CallItem> {
    let mut cursor = node.walk();
    let mut out = vec![];
    cursor.first_child();
    cursor.first_child();
    cursor.sibling();
    out.push(CallItem::Class {
        name: get_string(&cursor, bytes),
        range: cursor.node().range(),
    });
    cursor.parent();
    cursor.parent();
    out
}
pub fn class_or_variable(node: tree_sitter::Node<'_>, bytes: &[u8]) -> Option<CallItem> {
    let var_name = get_string_node(&node, bytes);
    return Some(CallItem::ClassOrVariable {
        name: var_name,
        range: node.range(),
    });
}

pub fn validate<'a>(
    call_chain: &'a Vec<CallItem>,
    point: &'a Point,
) -> Option<(usize, &'a [CallItem])> {
    let item = call_chain
        .iter()
        .enumerate()
        .find(|(_n, ci)| match ci {
            CallItem::MethodCall { name: _, range } => {
                tree_sitter_util::is_point_in_range(point, range)
            }
            CallItem::FieldAccess { name: _, range } => {
                tree_sitter_util::is_point_in_range(point, range)
            }
            CallItem::Variable { name: _, range } => {
                tree_sitter_util::is_point_in_range(point, range)
            }
            CallItem::ClassOrVariable { name: _, range } => {
                tree_sitter_util::is_point_in_range(point, range)
            }
            CallItem::Class { name: _, range } => tree_sitter_util::is_point_in_range(point, range),
            CallItem::ArgumentList {
                prev: _,
                range,
                filled_params: _,
                active_param: _,
            } => tree_sitter_util::is_point_in_range(point, range),
        })
        .map(|(a, _)| a)
        .unwrap_or_default();

    let relevat = &call_chain[0..item + 1];
    Some((item, relevat))
}

#[cfg(test)]
pub mod tests {
    use pretty_assertions::assert_eq;
    use tree_sitter::{Point, Range};

    use crate::call_chain::{get_call_chain, CallItem};

    #[test]
    fn call_chain_base() {
        let content = "
package ch.emilycares;

public class Test {

    public void hello(String a) {
        String local = \"\";

        var lo = local. 
        return;
    }
}
        ";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(8, 24));
        assert_eq!(
            out,
            Some(vec![CallItem::ClassOrVariable {
                name: "local".to_string(),
                range: Range {
                    start_byte: 125,
                    end_byte: 130,
                    start_point: Point { row: 8, column: 17 },
                    end_point: Point { row: 8, column: 22 }
                }
            }])
        );
    }

    pub const SYMBOL_METHOD: &str = "
package ch.emilycares;

public class Test {

    public void hello() {
        String local = \"\";

        var lo = local.concat(\"hehe\"). 
        return;
    }
}
        ";

    #[test]
    fn call_chain_method() {
        let (_, tree) = tree_sitter_util::parse(SYMBOL_METHOD).unwrap();

        let out = get_call_chain(&tree, SYMBOL_METHOD.as_bytes(), &Point::new(8, 40));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ClassOrVariable {
                    name: "local".to_string(),
                    range: Range {
                        start_byte: 117,
                        end_byte: 122,
                        start_point: Point { row: 8, column: 17 },
                        end_point: Point { row: 8, column: 22 },
                    }
                },
                CallItem::MethodCall {
                    name: "concat".to_string(),
                    range: Range {
                        start_byte: 123,
                        end_byte: 129,
                        start_point: Point { row: 8, column: 23 },
                        end_point: Point { row: 8, column: 29 }
                    },
                },
                CallItem::FieldAccess {
                    name: "return".to_string(),
                    range: Range {
                        start_byte: 148,
                        end_byte: 154,
                        start_point: Point { row: 9, column: 8 },
                        end_point: Point { row: 9, column: 14 },
                    }
                }
            ])
        );
    }

    #[test]
    fn call_chain_string() {
        let content = r#"
package ch.emilycares;
public class Test {
    public void hello() {
        String a = "";
        return "".  ;
    }
}
"#;
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 19));
        assert_eq!(
            out,
            Some(vec![CallItem::Class {
                name: "String".to_string(),
                range: Range {
                    start_byte: 108,
                    end_byte: 110,
                    start_point: Point { row: 5, column: 15 },
                    end_point: Point { row: 5, column: 17 },
                }
            }])
        );
    }

    #[test]
    fn call_chain_field() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var lo = local.a.
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 26));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ClassOrVariable {
                    name: "local".to_string(),
                    range: Range {
                        start_byte: 113,
                        end_byte: 118,
                        start_point: Point { row: 5, column: 17 },
                        end_point: Point { row: 5, column: 22 },
                    }
                },
                CallItem::FieldAccess {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 119,
                        end_byte: 120,
                        start_point: Point { row: 5, column: 23 },
                        end_point: Point { row: 5, column: 24 },
                    }
                }
            ])
        );
    }

    #[test]
    fn call_chain_method_base() {
        let content = "
package ch.emilycares;
public class GreetingResource {
    String a;
    public String hello() {
        a.concat(\"\"). 
        return \"huh\";
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 24));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ClassOrVariable {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 106,
                        end_byte: 107,
                        start_point: Point { row: 5, column: 8 },
                        end_point: Point { row: 5, column: 9 },
                    }
                },
                CallItem::MethodCall {
                    name: "concat".to_string(),
                    range: Range {
                        start_byte: 108,
                        end_byte: 114,
                        start_point: Point { row: 5, column: 10 },
                        end_point: Point { row: 5, column: 16 }
                    }
                }
            ])
        );
    }

    #[test]
    fn call_chain_method_info() {
        let content = "
package ch.emilycares;
public class GreetingResource {
    public String hello() {
        a.concat(\"\").other();
        return \"huh\";
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        // the cursor is on the concat method_call
        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 14));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ClassOrVariable {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 92,
                        end_byte: 93,
                        start_point: Point { row: 4, column: 8 },
                        end_point: Point { row: 4, column: 9 },
                    }
                },
                CallItem::MethodCall {
                    name: "concat".to_string(),
                    range: Range {
                        start_byte: 94,
                        end_byte: 100,
                        start_point: Point { row: 4, column: 10 },
                        end_point: Point { row: 4, column: 16 }
                    }
                }
            ])
        );
    }

    #[test]
    fn call_chain_field_method() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var lo = local.a.b(). ;
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 30));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ClassOrVariable {
                    name: "local".to_string(),
                    range: Range {
                        start_byte: 113,
                        end_byte: 118,
                        start_point: Point { row: 5, column: 17 },
                        end_point: Point { row: 5, column: 22 }
                    }
                },
                CallItem::FieldAccess {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 119,
                        end_byte: 120,
                        start_point: Point { row: 5, column: 23 },
                        end_point: Point { row: 5, column: 24 }
                    },
                },
                CallItem::MethodCall {
                    name: "b".to_string(),
                    range: Range {
                        start_byte: 121,
                        end_byte: 122,
                        start_point: Point { row: 5, column: 25 },
                        end_point: Point { row: 5, column: 26 }
                    }
                },
            ])
        );
    }

    #[test]
    fn call_chain_menthod_field() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var lo = local.a().b.
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 30));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ClassOrVariable {
                    name: "local".to_string(),
                    range: Range {
                        start_byte: 113,
                        end_byte: 118,
                        start_point: Point { row: 5, column: 17 },
                        end_point: Point { row: 5, column: 22 },
                    }
                },
                CallItem::MethodCall {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 119,
                        end_byte: 120,
                        start_point: Point { row: 5, column: 23 },
                        end_point: Point { row: 5, column: 24 },
                    }
                },
                CallItem::FieldAccess {
                    name: "b".to_string(),
                    range: Range {
                        start_byte: 123,
                        end_byte: 124,
                        start_point: Point { row: 5, column: 27 },
                        end_point: Point { row: 5, column: 28 },
                    }
                }
            ])
        );
    }

    #[test]
    fn call_chain_semicolon_simple() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var lo = local. ;
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 23));
        assert_eq!(
            out,
            Some(vec![CallItem::ClassOrVariable {
                name: "local".to_string(),
                range: Range {
                    start_byte: 113,
                    end_byte: 118,
                    start_point: Point { row: 5, column: 17 },
                    end_point: Point { row: 5, column: 22 },
                }
            }])
        );
    }

    #[test]
    fn call_chain_semicolon_field() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        int c = local.a().c.;
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 28));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ClassOrVariable {
                    name: "local".to_string(),
                    range: Range {
                        start_byte: 112,
                        end_byte: 117,
                        start_point: Point { row: 5, column: 16 },
                        end_point: Point { row: 5, column: 21 },
                    }
                },
                CallItem::MethodCall {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 118,
                        end_byte: 119,
                        start_point: Point { row: 5, column: 22 },
                        end_point: Point { row: 5, column: 23 },
                    }
                },
                CallItem::FieldAccess {
                    name: "c".to_string(),
                    range: Range {
                        start_byte: 122,
                        end_byte: 123,
                        start_point: Point { row: 5, column: 26 },
                        end_point: Point { row: 5, column: 27 },
                    }
                },
            ])
        );
    }

    #[test]
    fn call_chain_semicolon_method() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        int c = local.a.c().;
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 28));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ClassOrVariable {
                    name: "local".to_string(),
                    range: Range {
                        start_byte: 112,
                        end_byte: 117,
                        start_point: Point { row: 5, column: 16 },
                        end_point: Point { row: 5, column: 21 }
                    }
                },
                CallItem::FieldAccess {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 118,
                        end_byte: 119,
                        start_point: Point { row: 5, column: 22 },
                        end_point: Point { row: 5, column: 23 },
                    }
                },
                CallItem::MethodCall {
                    name: "c".to_string(),
                    range: Range {
                        start_byte: 120,
                        end_byte: 121,
                        start_point: Point { row: 5, column: 24 },
                        end_point: Point { row: 5, column: 25 },
                    }
                },
            ])
        );
    }

    #[test]
    fn call_chain_statement() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        local.a.c().;
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 20));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ClassOrVariable {
                    name: "local".to_string(),
                    range: Range {
                        start_byte: 104,
                        end_byte: 109,
                        start_point: Point { row: 5, column: 8 },
                        end_point: Point { row: 5, column: 13 },
                    }
                },
                CallItem::FieldAccess {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 110,
                        end_byte: 111,
                        start_point: Point { row: 5, column: 14 },
                        end_point: Point { row: 5, column: 15 },
                    }
                },
                CallItem::MethodCall {
                    name: "c".to_string(),
                    range: Range {
                        start_byte: 112,
                        end_byte: 113,
                        start_point: Point { row: 5, column: 16 },
                        end_point: Point { row: 5, column: 17 },
                    }
                },
            ])
        );
    }

    #[test]
    fn call_chain_class() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        String. 
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 16));
        assert_eq!(
            out,
            Some(vec![CallItem::ClassOrVariable {
                name: "String".to_string(),
                range: Range {
                    start_byte: 104,
                    end_byte: 110,
                    start_point: Point { row: 5, column: 8 },
                    end_point: Point { row: 5, column: 14 },
                }
            },])
        );
    }

    #[test]
    fn call_chain_varible_class() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        var local = String. 
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 28));
        assert_eq!(
            out,
            Some(vec![CallItem::ClassOrVariable {
                name: "String".to_string(),
                range: Range {
                    start_byte: 116,
                    end_byte: 122,
                    start_point: Point { row: 5, column: 20 },
                    end_point: Point { row: 5, column: 26 },
                }
            },])
        );
    }

    #[test]
    fn call_chain_argument() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        local.concat( )
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 22));
        assert_eq!(
            out,
            Some(vec![CallItem::ArgumentList {
                prev: vec![
                    CallItem::ClassOrVariable {
                        name: "local".to_string(),
                        range: Range {
                            start_byte: 104,
                            end_byte: 109,
                            start_point: Point { row: 5, column: 8 },
                            end_point: Point { row: 5, column: 13 },
                        },
                    },
                    CallItem::MethodCall {
                        name: "concat".to_string(),
                        range: Range {
                            start_byte: 110,
                            end_byte: 116,
                            start_point: Point { row: 5, column: 14 },
                            end_point: Point { row: 5, column: 20 },
                        },
                    },
                ],
                range: Range {
                    start_byte: 116,
                    end_byte: 119,
                    start_point: Point { row: 5, column: 20 },
                    end_point: Point { row: 5, column: 23 },
                },
                filled_params: vec![vec![]],
                active_param: 0
            },],)
        );
    }

    #[test]
    fn call_chain_argument_var() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        local.concat(local. );
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 27));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ArgumentList {
                    prev: vec![
                        CallItem::ClassOrVariable {
                            name: "local".to_string(),
                            range: Range {
                                start_byte: 104,
                                end_byte: 109,
                                start_point: Point { row: 5, column: 8 },
                                end_point: Point { row: 5, column: 13 },
                            },
                        },
                        CallItem::MethodCall {
                            name: "concat".to_string(),
                            range: Range {
                                start_byte: 110,
                                end_byte: 116,
                                start_point: Point { row: 5, column: 14 },
                                end_point: Point { row: 5, column: 20 },
                            },
                        },
                    ],
                    range: Range {
                        start_byte: 116,
                        end_byte: 125,
                        start_point: Point { row: 5, column: 20 },
                        end_point: Point { row: 5, column: 29 },
                    },
                    filled_params: vec![vec![CallItem::ClassOrVariable {
                        name: "local".to_string(),
                        range: Range {
                            start_byte: 117,
                            end_byte: 122,
                            start_point: Point { row: 5, column: 21 },
                            end_point: Point { row: 5, column: 26 }
                        }
                    }]],
                    active_param: 0
                },
                CallItem::ClassOrVariable {
                    name: "local".to_string(),
                    range: Range {
                        start_byte: 117,
                        end_byte: 122,
                        start_point: Point { row: 5, column: 21 },
                        end_point: Point { row: 5, column: 26 }
                    }
                }
            ])
        );
    }

    #[test]
    fn call_chain_argument_var_no_dot() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        local.concat(local );
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 27));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ArgumentList {
                    prev: vec![
                        CallItem::ClassOrVariable {
                            name: "local".to_string(),
                            range: Range {
                                start_byte: 104,
                                end_byte: 109,
                                start_point: Point { row: 5, column: 8 },
                                end_point: Point { row: 5, column: 13 },
                            },
                        },
                        CallItem::MethodCall {
                            name: "concat".to_string(),
                            range: Range {
                                start_byte: 110,
                                end_byte: 116,
                                start_point: Point { row: 5, column: 14 },
                                end_point: Point { row: 5, column: 20 },
                            },
                        },
                    ],
                    range: Range {
                        start_byte: 116,
                        end_byte: 124,
                        start_point: Point { row: 5, column: 20 },
                        end_point: Point { row: 5, column: 28 },
                    },
                    filled_params: vec![vec![CallItem::ClassOrVariable {
                        name: "local".to_string(),
                        range: Range {
                            start_byte: 117,
                            end_byte: 122,
                            start_point: Point { row: 5, column: 21 },
                            end_point: Point { row: 5, column: 26 }
                        }
                    }]],
                    active_param: 0
                },
                CallItem::ClassOrVariable {
                    name: "local".to_string(),
                    range: Range {
                        start_byte: 117,
                        end_byte: 122,
                        start_point: Point { row: 5, column: 21 },
                        end_point: Point { row: 5, column: 26 }
                    }
                }
            ])
        );
    }

    #[test]
    fn call_chain_argument_second_var() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        a.concat(b, c. );
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 23));
        match out.clone().unwrap().first().unwrap() {
            CallItem::ArgumentList {
                prev: _,
                active_param,
                filled_params: _,
                range: _,
            } => assert_eq!(active_param, &1),
            _ => assert!(false),
        };
        assert_eq!(
            out,
            Some(vec![
                CallItem::ArgumentList {
                    prev: vec![
                        CallItem::ClassOrVariable {
                            name: "a".to_string(),
                            range: Range {
                                start_byte: 104,
                                end_byte: 105,
                                start_point: Point { row: 5, column: 8 },
                                end_point: Point { row: 5, column: 9 },
                            },
                        },
                        CallItem::MethodCall {
                            name: "concat".to_string(),
                            range: Range {
                                start_byte: 106,
                                end_byte: 112,
                                start_point: Point { row: 5, column: 10 },
                                end_point: Point { row: 5, column: 16 },
                            },
                        },
                    ],
                    range: Range {
                        start_byte: 112,
                        end_byte: 120,
                        start_point: Point { row: 5, column: 16 },
                        end_point: Point { row: 5, column: 24 },
                    },
                    filled_params: vec![
                        vec![CallItem::ClassOrVariable {
                            name: "b".to_string(),
                            range: Range {
                                start_byte: 113,
                                end_byte: 114,
                                start_point: Point { row: 5, column: 17 },
                                end_point: Point { row: 5, column: 18 },
                            },
                        }],
                        vec![CallItem::ClassOrVariable {
                            name: "c".to_string(),
                            range: Range {
                                start_byte: 116,
                                end_byte: 117,
                                start_point: Point { row: 5, column: 20 },
                                end_point: Point { row: 5, column: 21 },
                            },
                        }]
                    ],
                    active_param: 1
                },
                CallItem::ClassOrVariable {
                    name: "c".to_string(),
                    range: Range {
                        start_byte: 116,
                        end_byte: 117,
                        start_point: Point { row: 5, column: 20 },
                        end_point: Point { row: 5, column: 21 },
                    }
                }
            ])
        );
    }

    #[test]
    fn call_chain_argument_active_param_not_last() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        a.concat(b , c);
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 19));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ArgumentList {
                    prev: vec![
                        CallItem::ClassOrVariable {
                            name: "a".to_string(),
                            range: Range {
                                start_byte: 104,
                                end_byte: 105,
                                start_point: Point { row: 5, column: 8 },
                                end_point: Point { row: 5, column: 9 },
                            },
                        },
                        CallItem::MethodCall {
                            name: "concat".to_string(),
                            range: Range {
                                start_byte: 106,
                                end_byte: 112,
                                start_point: Point { row: 5, column: 10 },
                                end_point: Point { row: 5, column: 16 },
                            },
                        },
                    ],
                    range: Range {
                        start_byte: 112,
                        end_byte: 119,
                        start_point: Point { row: 5, column: 16 },
                        end_point: Point { row: 5, column: 23 },
                    },
                    filled_params: vec![
                        vec![CallItem::ClassOrVariable {
                            name: "b".to_string(),
                            range: Range {
                                start_byte: 113,
                                end_byte: 114,
                                start_point: Point { row: 5, column: 17 },
                                end_point: Point { row: 5, column: 18 },
                            }
                        }],
                        vec![CallItem::ClassOrVariable {
                            name: "c".to_string(),
                            range: Range {
                                start_byte: 117,
                                end_byte: 118,
                                start_point: Point { row: 5, column: 21 },
                                end_point: Point { row: 5, column: 22 },
                            }
                        }],
                    ],
                    active_param: 0
                },
                CallItem::ClassOrVariable {
                    name: "b".to_string(),
                    range: Range {
                        start_byte: 113,
                        end_byte: 114,
                        start_point: Point { row: 5, column: 17 },
                        end_point: Point { row: 5, column: 18 },
                    }
                }
            ])
        );
    }

    #[test]
    fn call_chain_argument_second_var_no_dot() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        a.concat(b, c );
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 22));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ArgumentList {
                    prev: vec![
                        CallItem::ClassOrVariable {
                            name: "a".to_string(),
                            range: Range {
                                start_byte: 104,
                                end_byte: 105,
                                start_point: Point { row: 5, column: 8 },
                                end_point: Point { row: 5, column: 9 },
                            },
                        },
                        CallItem::MethodCall {
                            name: "concat".to_string(),
                            range: Range {
                                start_byte: 106,
                                end_byte: 112,
                                start_point: Point { row: 5, column: 10 },
                                end_point: Point { row: 5, column: 16 },
                            },
                        },
                    ],
                    range: Range {
                        start_byte: 112,
                        end_byte: 119,
                        start_point: Point { row: 5, column: 16 },
                        end_point: Point { row: 5, column: 23 },
                    },
                    filled_params: vec![
                        vec![CallItem::ClassOrVariable {
                            name: "b".to_string(),
                            range: Range {
                                start_byte: 113,
                                end_byte: 114,
                                start_point: Point { row: 5, column: 17 },
                                end_point: Point { row: 5, column: 18 },
                            }
                        }],
                        vec![CallItem::ClassOrVariable {
                            name: "c".to_string(),
                            range: Range {
                                start_byte: 116,
                                end_byte: 117,
                                start_point: Point { row: 5, column: 20 },
                                end_point: Point { row: 5, column: 21 },
                            }
                        }]
                    ],
                    active_param: 1
                },
                CallItem::ClassOrVariable {
                    name: "c".to_string(),
                    range: Range {
                        start_byte: 116,
                        end_byte: 117,
                        start_point: Point { row: 5, column: 20 },
                        end_point: Point { row: 5, column: 21 },
                    }
                }
            ])
        );
    }

    #[test]
    fn call_chain_argument_field() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        a.concat(b.a  );
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 22));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ArgumentList {
                    prev: vec![
                        CallItem::ClassOrVariable {
                            name: "a".to_string(),
                            range: Range {
                                start_byte: 104,
                                end_byte: 105,
                                start_point: Point { row: 5, column: 8 },
                                end_point: Point { row: 5, column: 9 },
                            },
                        },
                        CallItem::MethodCall {
                            name: "concat".to_string(),
                            range: Range {
                                start_byte: 106,
                                end_byte: 112,
                                start_point: Point { row: 5, column: 10 },
                                end_point: Point { row: 5, column: 16 },
                            },
                        },
                    ],
                    range: Range {
                        start_byte: 112,
                        end_byte: 119,
                        start_point: Point { row: 5, column: 16 },
                        end_point: Point { row: 5, column: 23 },
                    },
                    filled_params: vec![vec![
                        CallItem::ClassOrVariable {
                            name: "b".to_string(),
                            range: Range {
                                start_byte: 113,
                                end_byte: 114,
                                start_point: Point { row: 5, column: 17 },
                                end_point: Point { row: 5, column: 18 },
                            }
                        },
                        CallItem::FieldAccess {
                            name: "a".to_string(),
                            range: Range {
                                start_byte: 115,
                                end_byte: 116,
                                start_point: Point { row: 5, column: 19 },
                                end_point: Point { row: 5, column: 20 }
                            },
                        },
                    ]],
                    active_param: 0
                },
                CallItem::ClassOrVariable {
                    name: "b".to_string(),
                    range: Range {
                        start_byte: 113,
                        end_byte: 114,
                        start_point: Point { row: 5, column: 17 },
                        end_point: Point { row: 5, column: 18 },
                    }
                },
                CallItem::FieldAccess {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 115,
                        end_byte: 116,
                        start_point: Point { row: 5, column: 19 },
                        end_point: Point { row: 5, column: 20 }
                    },
                },
            ])
        );
    }
    #[test]
    fn call_chain_arguments() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        a.concat( );
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 18));
        assert_eq!(
            out,
            Some(vec![CallItem::ArgumentList {
                prev: vec![
                    CallItem::ClassOrVariable {
                        name: "a".to_string(),
                        range: Range {
                            start_byte: 78,
                            end_byte: 79,
                            start_point: Point { row: 4, column: 8 },
                            end_point: Point { row: 4, column: 9 },
                        },
                    },
                    CallItem::MethodCall {
                        name: "concat".to_string(),
                        range: Range {
                            start_byte: 80,
                            end_byte: 86,
                            start_point: Point { row: 4, column: 10 },
                            end_point: Point { row: 4, column: 16 },
                        },
                    }
                ],
                range: Range {
                    start_byte: 86,
                    end_byte: 89,
                    start_point: Point { row: 4, column: 16 },
                    end_point: Point { row: 4, column: 19 },
                },
                filled_params: vec![vec![]],
                active_param: 0
            }])
        );
    }

    #[test]
    fn call_chain_argument_method() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        Thing local = \"\";
        a.concat(b.a() );
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(5, 23));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ArgumentList {
                    prev: vec![
                        CallItem::ClassOrVariable {
                            name: "a".to_string(),
                            range: Range {
                                start_byte: 104,
                                end_byte: 105,
                                start_point: Point { row: 5, column: 8 },
                                end_point: Point { row: 5, column: 9 },
                            },
                        },
                        CallItem::MethodCall {
                            name: "concat".to_string(),
                            range: Range {
                                start_byte: 106,
                                end_byte: 112,
                                start_point: Point { row: 5, column: 10 },
                                end_point: Point { row: 5, column: 16 },
                            },
                        },
                    ],
                    range: Range {
                        start_byte: 112,
                        end_byte: 120,
                        start_point: Point { row: 5, column: 16 },
                        end_point: Point { row: 5, column: 24 },
                    },
                    filled_params: vec![vec![
                        CallItem::ClassOrVariable {
                            name: "b".to_string(),
                            range: Range {
                                start_byte: 113,
                                end_byte: 114,
                                start_point: Point { row: 5, column: 17 },
                                end_point: Point { row: 5, column: 18 }
                            }
                        },
                        CallItem::MethodCall {
                            name: "a".to_string(),
                            range: Range {
                                start_byte: 115,
                                end_byte: 116,
                                start_point: Point { row: 5, column: 19 },
                                end_point: Point { row: 5, column: 20 },
                            }
                        },
                    ]],
                    active_param: 0
                },
                CallItem::ClassOrVariable {
                    name: "b".to_string(),
                    range: Range {
                        start_byte: 113,
                        end_byte: 114,
                        start_point: Point { row: 5, column: 17 },
                        end_point: Point { row: 5, column: 18 }
                    }
                },
                CallItem::MethodCall {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 115,
                        end_byte: 116,
                        start_point: Point { row: 5, column: 19 },
                        end_point: Point { row: 5, column: 20 },
                    }
                },
            ])
        );
    }

    #[test]
    fn call_chain_if() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        if (a ) {
        }
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 14));
        assert_eq!(
            out,
            Some(vec![CallItem::ClassOrVariable {
                name: "a".to_string(),
                range: Range {
                    start_byte: 82,
                    end_byte: 83,
                    start_point: Point { row: 4, column: 12 },
                    end_point: Point { row: 4, column: 13 },
                }
            }])
        );
    }

    #[test]
    fn call_chain_if_condition() {
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
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 19));
        assert_eq!(
            out,
            Some(vec![CallItem::ClassOrVariable {
                name: "b".to_string(),
                range: Range {
                    start_byte: 87,
                    end_byte: 88,
                    start_point: Point { row: 4, column: 17 },
                    end_point: Point { row: 4, column: 18 },
                }
            },])
        );
    }

    #[test]
    fn call_chain_return() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        return a. ;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 18));
        assert_eq!(
            out,
            Some(vec![CallItem::ClassOrVariable {
                name: "a".to_string(),
                range: Range {
                    start_byte: 85,
                    end_byte: 86,
                    start_point: Point { row: 4, column: 15 },
                    end_point: Point { row: 4, column: 16 }
                }
            },])
        );
    }

    #[test]
    fn call_chain_return_method_call() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        return a.b(). ;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 22));
        assert_eq!(
            out,
            Some(vec![
                CallItem::ClassOrVariable {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 85,
                        end_byte: 86,
                        start_point: Point { row: 4, column: 15 },
                        end_point: Point { row: 4, column: 16 },
                    }
                },
                CallItem::MethodCall {
                    name: "b".to_string(),
                    range: Range {
                        start_byte: 87,
                        end_byte: 88,
                        start_point: Point { row: 4, column: 17 },
                        end_point: Point { row: 4, column: 18 },
                    }
                }
            ])
        );
    }

    #[test]
    fn call_chain_new_instance() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        new String()
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 22));
        assert_eq!(
            out,
            Some(vec![CallItem::Class {
                name: "String".to_string(),
                range: Range {
                    start_byte: 82,
                    end_byte: 88,
                    start_point: Point { row: 4, column: 12 },
                    end_point: Point { row: 4, column: 18 }
                }
            }])
        );
    }

    #[test]
    fn call_chain_new_instance_field() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        new String().a.
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 22));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Class {
                    name: "String".to_string(),
                    range: Range {
                        start_byte: 82,
                        end_byte: 88,
                        start_point: Point { row: 4, column: 12 },
                        end_point: Point { row: 4, column: 18 },
                    }
                },
                CallItem::FieldAccess {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 91,
                        end_byte: 92,
                        start_point: Point { row: 4, column: 21 },
                        end_point: Point { row: 4, column: 22 },
                    }
                }
            ])
        );
    }

    #[test]
    fn call_chain_new_instance_method() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        new String().a(). ;
        return;
    }
}
";
        let (_, tree) = tree_sitter_util::parse(&content).unwrap();

        let out = get_call_chain(&tree, content.as_bytes(), &Point::new(4, 25));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Class {
                    name: "String".to_string(),
                    range: Range {
                        start_byte: 82,
                        end_byte: 88,
                        start_point: Point { row: 4, column: 12 },
                        end_point: Point { row: 4, column: 18 },
                    }
                },
                CallItem::MethodCall {
                    name: "a".to_string(),
                    range: Range {
                        start_byte: 91,
                        end_byte: 92,
                        start_point: Point { row: 4, column: 21 },
                        end_point: Point { row: 4, column: 22 },
                    }
                }
            ])
        );
    }
}
