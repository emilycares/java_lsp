use tree_sitter::Point;
use tree_sitter_util::{get_string, CommentSkiper};

use crate::Document;

#[derive(Debug, PartialEq)]
pub enum CallItem {
    MethodCall(String),
    FieldAccess(String),
    Variable(String),
    Class(String),
}

/// Provides data abuilt the current variable before the cursor
/// ``` java
/// Long other = 1l;
/// other.
///       ^
/// ```
/// Then it would return info about the variable other
pub fn get_call_chain(document: &Document, point: &Point) -> Option<Vec<CallItem>> {
    let tree = &document.tree;
    let bytes = document.as_bytes();

    let mut cursor = tree.walk();
    let mut level = 0;
    let mut out = vec![];
    loop {
        match cursor.node().kind() {
            "argument_list" => {
                out.clear();
                cursor.first_child();
                out.extend(parse_argument_list_argument(&mut cursor, bytes));
                while cursor.node().kind() == "," {
                    out.extend(parse_argument_list_argument(&mut cursor, bytes));
                }
                cursor.parent();
            }
            "scoped_type_identifier" => {
                let l = get_string(&cursor, bytes);
                let l = l.split_once("\n").unwrap_or_default().0;
                let l = l.trim();
                let l = l.trim_end_matches('.');

                if let Some(c) = class_or_variable(l.to_owned()) {
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

fn parse_argument_list_argument(
    cursor: &mut tree_sitter::TreeCursor<'_>,
    bytes: &[u8],
) -> Vec<CallItem> {
    let mut out = vec![];
    cursor.sibling();
    match cursor.node().kind() {
        "identifier" => 'block: {
            if cursor.node().kind() == "identifier" {
                let identifier = get_string(&*cursor, bytes);
                cursor.sibling();
                if cursor.node().kind() == ")" {
                    if let Some(c) = class_or_variable(identifier) {
                        out.push(c);
                    }
                    break 'block;
                }
                if cursor.node().kind() == "ERROR" {
                    if let Some(c) = class_or_variable(identifier) {
                        out.push(c);
                    }
                }
            }
        }
        _ => {
            out.extend(parse_value(cursor, bytes));
        }
    }

    out
}

fn parse_value(cursor: &tree_sitter::TreeCursor<'_>, bytes: &[u8]) -> Vec<CallItem> {
    match cursor.node().kind() {
        "identifier" => {
            if let Some(c) = class_or_variable(get_string(cursor, bytes)) {
                return vec![c];
            }
            vec![]
        }
        "field_access" => parse_field_access(cursor.node(), bytes),
        "method_invocation" => parse_method_invocation(cursor.node(), bytes),
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
            out.extend(vec![CallItem::MethodCall(method_name)]);
        }
        "identifier" => {
            let var_name = get_string(&cursor, bytes);
            if let Some(c) = class_or_variable(var_name) {
                out.push(c);
            }
            cursor.sibling();
            cursor.sibling();
            let method_name = get_string(&cursor, bytes);
            out.push(CallItem::MethodCall(method_name));
        }
        "method_invocation" => {
            out.extend(parse_method_invocation(cursor.node(), bytes));
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
            let var_name = get_string(&cursor, bytes);
            cursor.sibling();
            cursor.sibling();
            let field_name = get_string(&cursor, bytes);
            if let Some(item) = class_or_variable(var_name) {
                out.push(item);
            }
            if field_name != "return" {
                out.push(CallItem::FieldAccess(field_name));
            }
        }
        "method_invocation" => {
            out.extend(parse_method_invocation(cursor.node(), bytes));
            cursor.sibling();
            cursor.sibling();
            let field_name = get_string(&cursor, bytes);
            out.extend(vec![CallItem::FieldAccess(field_name)]);
        }
        "field_access" => {
            out.extend(parse_field_access(cursor.node(), bytes));
        }
        _ => {}
    }

    cursor.parent();
    out
}
pub fn class_or_variable(var_name: String) -> Option<CallItem> {
    if let Some(c) = var_name.chars().next() {
        return Some(match c.is_uppercase() {
            true => CallItem::Class(var_name),
            false => CallItem::Variable(var_name),
        });
    }
    None
}

#[cfg(test)]
pub mod tests {
    use pretty_assertions::assert_eq;
    use tree_sitter::Point;

    use crate::{
        call_chain::{get_call_chain, CallItem},
        Document,
    };

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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(8, 24));
        assert_eq!(out, Some(vec![CallItem::Variable("local".to_string()),]));
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
        let doc = Document::setup(SYMBOL_METHOD).unwrap();

        let out = get_call_chain(&doc, &Point::new(8, 40));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::MethodCall("concat".to_string()),
                CallItem::FieldAccess("return".to_string()),
            ])
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 26));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::FieldAccess("a".to_string())
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 24));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("a".to_string()),
                CallItem::MethodCall("concat".to_string())
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
        let doc = Document::setup(content).unwrap();

        // the cursor is on the concat method_call
        let out = get_call_chain(&doc, &Point::new(4, 14));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("a".to_string()),
                CallItem::MethodCall("concat".to_string())
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 30));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::FieldAccess("a".to_string()),
                CallItem::MethodCall("b".to_string()),
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 30));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::MethodCall("a".to_string()),
                CallItem::FieldAccess("b".to_string())
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 23));
        assert_eq!(out, Some(vec![CallItem::Variable("local".to_string())]));
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 28));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::MethodCall("a".to_string()),
                CallItem::FieldAccess("c".to_string()),
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 28));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::FieldAccess("a".to_string()),
                CallItem::MethodCall("c".to_string()),
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 20));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("local".to_string()),
                CallItem::FieldAccess("a".to_string()),
                CallItem::MethodCall("c".to_string()),
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 16));
        assert_eq!(out, Some(vec![CallItem::Class("String".to_string()),]));
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 28));
        assert_eq!(out, Some(vec![CallItem::Class("String".to_string()),]));
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 22));
        assert_eq!(out, None);
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 27));
        assert_eq!(out, Some(vec![CallItem::Variable("local".to_string()),]));
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 27));
        assert_eq!(out, Some(vec![CallItem::Variable("local".to_string()),]));
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 23));
        assert_eq!(out, Some(vec![CallItem::Variable("c".to_string()),]));
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 22));
        assert_eq!(out, Some(vec![CallItem::Variable("c".to_string()),]));
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 22));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("b".to_string()),
                CallItem::FieldAccess("a".to_string()),
            ])
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(5, 23));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("b".to_string()),
                CallItem::MethodCall("a".to_string()),
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(4, 14));
        assert_eq!(out, Some(vec![CallItem::Variable("a".to_string()),]));
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(4, 19));
        assert_eq!(out, Some(vec![CallItem::Variable("b".to_string()),]));
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(4, 18));
        assert_eq!(out, Some(vec![CallItem::Variable("a".to_string()),]));
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
        let doc = Document::setup(content).unwrap();

        let out = get_call_chain(&doc, &Point::new(4, 22));
        assert_eq!(
            out,
            Some(vec![
                CallItem::Variable("a".to_string()),
                CallItem::MethodCall("b".to_string())
            ])
        );
    }
}
