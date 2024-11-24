use std::vec;

use tree_sitter::{Parser, Query, QueryCursor};
use tree_sitter_util::CommentSkiper;

use crate::dto::{self};

pub fn load_java(bytes: &[u8], class_path: String) -> Result<crate::dto::Class, dto::ClassError> {
    let mut parser = Parser::new();
    let language = tree_sitter_java::LANGUAGE;
    parser.set_language(&language.into())?;

    let Some(tree) = parser.parse(bytes, None) else {
        return Err(dto::ClassError::ParseError);
    };

    let methods = parse_methods(&tree, bytes);
    let fields = parse_fields(&tree, bytes);

    Ok(dto::Class {
        class_path,
        access: vec![],
        name: get_class_name(tree, bytes),
        methods,
        fields,
    })
}

/// Define the query to match method declarations
fn parse_methods(tree: &tree_sitter::Tree, bytes: &[u8]) -> Vec<dto::Method> {
    let query_str = r#"
    (method_declaration) @method
    "#;
    let language = tree_sitter_java::LANGUAGE;
    let query = Query::new(&language.into(), query_str).expect("Error compiling query");

    // Execute the query
    let mut query_cursor = QueryCursor::new();
    let matches = query_cursor.matches(&query, tree.root_node(), bytes);

    let methods = matches
        .into_iter()
        .flat_map(|m| m.captures)
        .map(|c| parse_method(c.node, bytes))
        .collect::<Vec<_>>();
    methods
}

fn parse_fields(tree: &tree_sitter::Tree, bytes: &[u8]) -> Vec<dto::Field> {
    let query_str = r#"
    (field_declaration) @field
    "#;
    let language = tree_sitter_java::LANGUAGE;
    let query = Query::new(&language.into(), query_str).expect("Error compiling query");

    // Execute the query
    let mut query_cursor = QueryCursor::new();
    let matches = query_cursor.matches(&query, tree.root_node(), bytes);

    let fields = matches
        .into_iter()
        .flat_map(|m| m.captures)
        .map(|c| parse_field(c.node, bytes))
        .collect::<Vec<_>>();
    fields
}

fn parse_method(node: tree_sitter::Node<'_>, bytes: &[u8]) -> dto::Method {
    let mut cursor = node.walk();
    cursor.first_child();

    let mut method = dto::Method {
        access: vec![],
        name: "".to_owned(),
        parameters: vec![],
        ret: dto::JType::Void,
    };

    loop {
        match cursor.node().kind() {
            "modifiers" => {
                method.access = parser_modifiers(get_string(&cursor, bytes));
            }
            "integral_type" => {
                method.ret = parse_jtype(&cursor, bytes);
            }
            "identifier" => method.name = get_string(&cursor, bytes),
            "formal_parameters" => {
                method.parameters = parse_formal_parameters(&mut cursor, bytes);
            }
            _ => {
                //dbg!(cursor.node().kind());
                //dbg!(get_string(&cursor, bytes));
            }
        };
        if !cursor.sibling() {
            break;
        }
    }

    method
}

fn parse_field(node: tree_sitter::Node<'_>, bytes: &[u8]) -> dto::Field {
    let mut cursor = node.walk();
    cursor.first_child();

    let mut field = dto::Field {
        access: vec![],
        name: "".to_owned(),
        jtype: dto::JType::Void,
    };

    loop {
        match cursor.node().kind() {
            "modifiers" => {
                field.access = parser_modifiers(get_string(&cursor, bytes));
            }
            "integral_type" => {
                field.jtype = parse_jtype(&cursor, bytes);
            }
            "variable_declarator" => field.name = get_string(&cursor, bytes),
            _ => {
                // dbg!(cursor.node().kind());
                // dbg!(get_string(&cursor, bytes));
            }
        };
        if !cursor.sibling() {
            break;
        }
    }

    field
}

fn parser_modifiers(input: String) -> Vec<dto::Access> {
    let mut out = vec![];
    if input.contains("public") {
        out.push(dto::Access::Public);
    }
    if input.contains("private") {
        out.push(dto::Access::Private);
    }
    out
}

fn get_class_name(tree: tree_sitter::Tree, bytes: &[u8]) -> String {
    let mut cursor = tree.walk();
    cursor.first_child();
    cursor.sibling();
    cursor.first_child();
    cursor.sibling();
    cursor.sibling();
    get_string(&cursor, bytes)
}

fn parse_formal_parameters(
    cursor: &mut tree_sitter::TreeCursor<'_>,
    bytes: &[u8],
) -> Vec<dto::Parameter> {
    let mut out = vec![];
    cursor.first_child();
    while cursor.sibling() {
        if cursor.node().kind() != "formal_parameter" {
            continue;
        }
        cursor.first_child();
        let jtype = parse_jtype(&*cursor, bytes);

        cursor.sibling();
        out.push(dto::Parameter {
            name: Some(get_string(&*cursor, bytes)),
            jtype,
        });
        cursor.parent();
    }
    cursor.parent();
    out
}

fn get_string(cursor: &tree_sitter::TreeCursor<'_>, bytes: &[u8]) -> String {
    cursor.node().utf8_text(bytes).unwrap().to_owned()
}

fn parse_jtype(cursor: &tree_sitter::TreeCursor<'_>, bytes: &[u8]) -> dto::JType {
    match (
        cursor.node().kind(),
        cursor.node().utf8_text(bytes).unwrap(),
    ) {
        ("integral_type", "int") => dto::JType::Int,
        (_, _) => dto::JType::Void,
    }
}
#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::load_java;

    #[test]
    fn everything() {
        let result = load_java(include_bytes!("../test/Everything.java"), "".to_string());

        assert_eq!(crate::tests::everything_data(), result.unwrap());
    }
}
