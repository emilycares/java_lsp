use tree_sitter::Parser;
use tree_sitter_util::CommentSkiper;

use crate::{dto, loader::SourceDestination};

pub fn load_java(
    bytes: &[u8],
    source: SourceDestination,
) -> Result<crate::dto::Class, dto::ClassError> {
    let mut parser = Parser::new();
    let language = tree_sitter_java::LANGUAGE;
    parser.set_language(&language.into())?;

    let Some(tree) = parser.parse(bytes, None) else {
        return Err(dto::ClassError::ParseError);
    };

    let mut methods = vec![];
    let mut fields = vec![];
    let mut class_name = None;
    let mut class_path_base: Option<String> = None;

    let mut cursor = tree.walk();
    cursor.first_child();
    if cursor.node().kind() == "package_declaration" {
        cursor.first_child();
        cursor.sibling();
        class_path_base = Some(get_string(&cursor, bytes));
        cursor.parent();
    }
    cursor.sibling();
    while let "import_declaration" = cursor.node().kind() {
        cursor.sibling();
    }
    if cursor.node().kind() == "class_declaration" {
        cursor.first_child();
        cursor.sibling();
        cursor.sibling();
        if cursor.node().kind() == "identifier" {
            class_name = Some(get_string(&cursor, bytes))
        }
        cursor.sibling();
        cursor.first_child();
        while cursor.sibling() {
            if cursor.node().kind() == "field_declaration" {
                fields.push(parse_field(cursor.node(), bytes))
            }
            if cursor.node().kind() == "method_declaration" {
                methods.push(parse_method(cursor.node(), bytes))
            }
        }
    }

    let Some(name) = class_name else {
        return Err(dto::ClassError::UnknownClassName);
    };
    let Some(class_path_base) = class_path_base else {
        return Err(dto::ClassError::UnknownClassPath);
    };
    let source = match source {
        SourceDestination::RelativeInFolder(e) => {
            format!("{}/{}/{}.java", e, &class_path_base.replace(".", "/"), name)
        }
        SourceDestination::Here(e) => e,
        SourceDestination::None => "".to_string(),
    };
    Ok(dto::Class {
        source,
        class_path: format!("{}.{}", class_path_base, name),
        access: vec![],
        name,
        methods,
        fields,
    })
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
            _ => {}
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
            _ => {}
        };
        if !cursor.sibling() {
            break;
        }
    }

    field
}

fn parser_modifiers(input: String) -> Vec<dto::Access> {
    let mut out = vec![];
    if input.contains("static") {
        out.push(dto::Access::Static);
    }
    if input.contains("public") {
        out.push(dto::Access::Public);
    }
    if input.contains("private") {
        out.push(dto::Access::Private);
    }
    out
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

    use crate::{dto, loader::SourceDestination};

    use super::load_java;

    #[test]
    fn everything() {
        let result = load_java(
            include_bytes!("../test/Everything.java"),
            SourceDestination::RelativeInFolder("/path/to/source".to_string()),
        );

        assert_eq!(crate::tests::everything_data(), result.unwrap());
    }

    #[test]
    fn int() {
        let src = r#"
package a.test;

import jakarta.inject.Inject;
import jakarta.ws.rs.GET;

import jakarta.ws.rs.Path;

public class Test {
}
 "#;
        let result = load_java(
            src.as_bytes(),
            SourceDestination::RelativeInFolder("/path/to/source".to_string()),
        );
        assert_eq!(
            result.unwrap(),
            dto::Class {
                class_path: "a.test.Test".to_string(),
                source: "/path/to/source/a/test/Test.java".to_string(),
                access: vec![],
                name: "Test".to_string(),
                methods: vec![],
                fields: vec![]
            }
        );
    }
}
