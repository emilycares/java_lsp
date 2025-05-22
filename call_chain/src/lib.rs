use std::cmp;

use tree_sitter::{Point, Range};
use tree_sitter_util::{CommentSkiper, get_string, get_string_node};

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
    This {
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
        active_param: Option<usize>,
        filled_params: Vec<Vec<CallItem>>,
        range: Range,
    },
}

impl CallItem {
    pub fn get_range(&self) -> &Range {
        match self {
            CallItem::MethodCall { name: _, range } => range,
            CallItem::FieldAccess { name: _, range } => range,
            CallItem::Variable { name: _, range } => range,
            CallItem::This { range } => range,
            CallItem::Class { name: _, range } => range,
            CallItem::ClassOrVariable { name: _, range } => range,
            CallItem::ArgumentList {
                prev: _,
                active_param: _,
                filled_params: _,
                range,
            } => range,
        }
    }
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
            "field_declaration" => {
                cursor.first_child();
                if cursor.node().kind() == "modifiers" {
                    cursor.sibling();
                }
                cursor.sibling();
                if cursor.node().kind() == "variable_declarator" {
                    parse_variable_declarator(&mut cursor, &mut out, bytes, point);
                }
                cursor.parent();
            }
            "annotation_argument_list" => {
                cursor.first_child();
                cursor.sibling();
                out.extend(parse_value(&cursor, bytes, point));
                cursor.parent();
            }
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
                    out.extend(parse_method_invocation(cursor.node(), bytes, point));
                }
                cursor.parent();
            }
            "expression_statement" => {
                cursor.first_child();
                out.extend(parse_value(&cursor, bytes, point));
                cursor.parent();
            }
            "return_statement" => {
                cursor.first_child();
                cursor.sibling();
                out.extend(parse_value(&cursor, bytes, point));
                cursor.parent();
            }
            "local_variable_declaration" => {
                cursor.first_child();
                cursor.sibling();
                if cursor.node().kind() == "variable_declarator" {
                    parse_variable_declarator(&mut cursor, &mut out, bytes, point);
                }
                cursor.parent();
            }
            "parenthesized_expression" => {
                cursor.first_child();
                cursor.sibling();
                cursor.goto_first_child_for_point(Point::new(point.row, point.column - 3));
                out.extend(parse_value(&cursor, bytes, point));
                cursor.parent();
            }
            "assignment_expression" => {
                cursor.first_child();
                out.extend(parse_value(&cursor, bytes, point));
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

fn parse_binary_expression(
    cursor: &tree_sitter::TreeCursor<'_>,
    bytes: &[u8],
    point: &Point,
) -> Vec<CallItem> {
    let mut cursor = cursor.node().walk();
    let mut out = vec![];
    cursor.first_child();

    if tree_sitter_util::is_point_in_range(point, &cursor.node().range()) {
        out.extend(parse_value(&cursor, bytes, point));
    }
    cursor.sibling();
    cursor.sibling();
    if tree_sitter_util::is_point_in_range(point, &cursor.node().range()) {
        out.extend(parse_value(&cursor, bytes, point));
    }

    cursor.parent();
    out
}

fn parse_variable_declarator(
    cursor: &mut tree_sitter::TreeCursor<'_>,
    out: &mut Vec<CallItem>,
    bytes: &[u8],
    point: &Point,
) {
    cursor.first_child();
    cursor.sibling();
    cursor.sibling();
    out.extend(parse_value(&*cursor, bytes, point));
    cursor.parent();
}

pub fn parse_argument_list(
    out: &[CallItem],
    cursor: &mut tree_sitter::TreeCursor<'_>,
    bytes: &[u8],
    point: &Point,
) -> (Option<Vec<CallItem>>, CallItem) {
    let mut active_param = None;
    let mut current_param = 0;

    let arg_prev = out.to_owned();
    let arg_range = cursor.node().range();
    let mut filled_params = vec![];
    cursor.first_child();
    let Argument { range, value } = parse_argument_list_argument(cursor, bytes, point);
    filled_params.push(value.clone());
    if tree_sitter_util::is_point_in_range(point, &range.unwrap()) {
        active_param = Some(current_param);
    }
    while cursor.node().kind() == "," {
        current_param += 1;
        let Argument { range, value } = parse_argument_list_argument(cursor, bytes, point);
        filled_params.push(value.clone());
        if tree_sitter_util::is_point_in_range(point, &range.unwrap()) {
            active_param = Some(current_param);
        }
    }
    let value = CallItem::ArgumentList {
        prev: arg_prev,
        active_param,
        filled_params: filled_params.clone(),
        range: arg_range,
    };
    match active_param {
        Some(a) => {
            let after = filled_params.get(a).cloned();
            (after, value)
        }
        None => (None, value),
    }
}

fn parse_argument_list_argument(
    cursor: &mut tree_sitter::TreeCursor<'_>,
    bytes: &[u8],
    point: &Point,
) -> Argument {
    let mut out = Argument {
        range: None,
        value: vec![],
    };
    cursor.sibling();
    out.range = Some(cursor.node().range());
    out.value.extend(parse_value(cursor, bytes, point));
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

fn parse_value(cursor: &tree_sitter::TreeCursor<'_>, bytes: &[u8], point: &Point) -> Vec<CallItem> {
    match cursor.node().kind() {
        "identifier" => {
            if let Some(c) = class_or_variable(cursor.node(), bytes) {
                return vec![c];
            }
            vec![]
        }
        "field_access" => parse_field_access(cursor.node(), bytes, point),
        "method_invocation" => parse_method_invocation(cursor.node(), bytes, point),
        "object_creation_expression" => parse_object_creation(cursor.node(), bytes),
        "string_literal" => vec![CallItem::Class {
            name: "String".to_string(),
            range: cursor.node().range(),
        }],
        "binary_expression" => parse_binary_expression(cursor, bytes, point),
        _ => vec![],
    }
}

fn parse_method_invocation(
    node: tree_sitter::Node<'_>,
    bytes: &[u8],
    point: &Point,
) -> Vec<CallItem> {
    let mut cursor = node.walk();
    let mut out = vec![];
    cursor.first_child();
    // println!("Custom backtrace: {}", std::backtrace::Backtrace::force_capture());
    match cursor.node().kind() {
        "field_access" => {
            out.extend(parse_field_access(node, bytes, point));
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
            let indent_varibale = class_or_variable(var_name, bytes);
            cursor.sibling();
            match cursor.node().kind() {
                "argument_list" => {
                    out.push(CallItem::MethodCall {
                        name: get_string_node(&var_name, bytes),
                        range: var_name.range(),
                    });
                }
                _ => {
                    if let Some(i) = indent_varibale {
                        out.push(i);
                    }
                    cursor.sibling();
                    let method_name = get_string(&cursor, bytes);
                    out.push(CallItem::MethodCall {
                        name: method_name,
                        range: cursor.node().range(),
                    });
                }
            }
        }
        "method_invocation" => {
            out.extend(parse_method_invocation(cursor.node(), bytes, point));
            cursor.sibling();
            cursor.sibling();
            let method_name = get_string(&cursor, bytes);
            out.extend(vec![CallItem::MethodCall {
                name: method_name,
                range: cursor.node().range(),
            }]);
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

fn parse_field_access(node: tree_sitter::Node<'_>, bytes: &[u8], point: &Point) -> Vec<CallItem> {
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
            out.extend(parse_value(&cursor, bytes, point));
        }
        "method_invocation" => {
            out.extend(parse_method_invocation(cursor.node(), bytes, point));
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
            out.extend(parse_field_access(cursor.node(), bytes, point));
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
        "this" => {
            out.push(CallItem::This {
                range: cursor.node().range(),
            });
            cursor.sibling();
            cursor.sibling();
            out.push(CallItem::FieldAccess {
                name: get_string(&cursor, bytes),
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
    Some(CallItem::ClassOrVariable {
        name: var_name,
        range: node.range(),
    })
}

pub fn validate(call_chain: &[CallItem], point: &Point) -> (usize, Vec<CallItem>) {
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
            CallItem::This { range } => tree_sitter_util::is_point_in_range(point, range),
            CallItem::ClassOrVariable { name: _, range } => {
                tree_sitter_util::is_point_in_range(point, range)
            }
            CallItem::Class { name: _, range } => tree_sitter_util::is_point_in_range(point, range),
            CallItem::ArgumentList {
                prev,
                range,
                filled_params: _,
                active_param: _,
            } => {
                if tree_sitter_util::is_point_in_range(point, range) {
                    return true;
                }
                let mut prevs = None;
                for p in prev {
                    match prevs {
                        None => {
                            prevs = Some(*p.get_range());
                        }
                        Some(pr) => prevs = Some(tree_sitter_util::add_ranges(pr, *p.get_range())),
                    }
                }
                if let Some(r) = prevs {
                    if tree_sitter_util::is_point_in_range(point, &r) {
                        return true;
                    }
                }
                false
            }
        })
        .map(|i| i.0)
        .unwrap_or_default();

    let relevat = &call_chain[0..cmp::min(item + 1, call_chain.len())];
    (item, relevat.to_vec())
}

pub fn flatten_argument_lists(call_chain: &[CallItem]) -> Vec<CallItem> {
    let mut out = vec![];
    for ci in call_chain {
        if let CallItem::ArgumentList {
            prev,
            active_param,
            filled_params: _,
            range: _,
        } = ci
        {
            if active_param.is_none() {
                out.extend(prev.iter().map(Clone::clone));
            }
        } else {
            out.push(ci.clone());
        }
    }
    out
}
