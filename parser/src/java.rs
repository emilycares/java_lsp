use tree_sitter::{Parser, Query, QueryCursor};
use tree_sitter_java::language;
use tree_sitter_util::CommentSkiper;

use crate::dto;

pub fn load_java(bytes: &[u8]) -> Result<crate::dto::Class, dto::ClassError> {
    let mut parser = Parser::new();
    parser.set_language(&language())?;

    let Some(tree) = parser.parse(bytes, None) else {
        return Err(dto::ClassError::ParseError);
    };

    // Define the query to match method declarations
    let query_str = r#"
    (method_declaration) @method
    "#;
    let query = Query::new(&language(), query_str).expect("Error compiling query");

    // Execute the query
    let mut query_cursor = QueryCursor::new();
    let matches = query_cursor.matches(&query, tree.root_node(), bytes);

    let methods = matches
        .into_iter()
        .flat_map(|m| m.captures)
        .map(|c| {
            let node = c.node;
            let mut cursor = node.walk();
            cursor.first_child();

            let mut method = dto::Method {
                access: vec![],
                name: "".to_owned(),
                parameters: vec![],
                ret: dto::JType::Void,
            };

            while cursor.sibling() {
                match cursor.node().kind() {
                    "modifiers" => {}
                    "type" => {}
                    "void_type" => {}
                    "identifier" => {
                        method.name = cursor.node().utf8_text(bytes).unwrap().to_owned()
                    }
                    "formal_parameters" => {
                        cursor.first_child();
                        cursor.sibling();
                        if cursor.node().kind() == "formal_parameter" {
                            cursor.first_child();
                            let jtype = match (cursor.node().kind(), cursor.node().utf8_text(bytes).unwrap()) {
                                ("integral_type", "int") => dto::JType::Int,
                                (_, _) => dto::JType::Void
                            };

                            cursor.sibling();
                            method.parameters.push(dto::Parameter {
                                name: cursor.node().utf8_text(bytes).unwrap().to_owned(),
                                jtype,
                            });
                            cursor.parent();
                        }
                        cursor.parent();
                    }
                    _ => {}
                };
            }

            method
        })
        .collect::<Vec<_>>();

    Ok(dto::Class {
        access: vec![],
        name: "".to_owned(),
        methods,
    })
}
#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::dto;

    use super::load_java;

    #[test]
    fn everything() {
        let result = load_java(include_bytes!("../test/Everything.java"));

        assert_eq!(
            dto::Class {
                access: vec![],
                name: "".to_owned(),
                methods: vec![]
            },
            result.unwrap()
        );
    }
}