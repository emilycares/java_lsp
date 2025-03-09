use tree_sitter_util::CommentSkiper;

use crate::{dto, loader::SourceDestination};

pub fn load_java(
    bytes: &[u8],
    source: SourceDestination,
) -> Result<crate::dto::Class, dto::ClassError> {
    let Some((_, tree)) = tree_sitter_util::parse(bytes) else {
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
    match cursor.node().kind() {
        "class_declaration" => {
            cursor.first_child();
            cursor.sibling();
            cursor.sibling();
            if cursor.node().kind() == "identifier" {
                class_name = Some(get_string(&cursor, bytes))
            }
            cursor.sibling();
            cursor.first_child();
            while cursor.sibling() {
                match cursor.node().kind() {
                    "field_declaration" => fields.push(parse_field(cursor.node(), bytes)),
                    "method_declaration" => methods.push(parse_method(cursor.node(), bytes)),
                    "{" | "}" => (),
                    unknown => eprintln!("Missing implementation for: {} in class body", unknown),
                }
            }
        }
        "interface_declaration" => {
            cursor.first_child();
            cursor.sibling();
            cursor.sibling();
            if cursor.node().kind() == "identifier" {
                class_name = Some(get_string(&cursor, bytes))
            }
            cursor.sibling();
            cursor.first_child();
            while cursor.sibling() {
                match cursor.node().kind() {
                    "constant_declaration" => {
                        fields.push(parse_interface_constant(&mut cursor, bytes))
                    }
                    "," | "{" | "}" => (),
                    unknown => eprintln!("Missing implementation for: {} in interface", unknown),
                }
            }
        }
        "enum_declaration" => {
            cursor.first_child();
            cursor.sibling();
            cursor.sibling();
            if cursor.node().kind() == "identifier" {
                class_name = Some(get_string(&cursor, bytes))
            }
            cursor.sibling();
            cursor.first_child();
            while cursor.sibling() {
                match cursor.node().kind() {
                    "enum_constant" => fields.push(parse_enum_constant(cursor.node(), bytes)),
                    "," | "{" | "}" => (),
                    unknown => eprintln!("Missing implementation for: {}", unknown),
                }
            }
        }
        missing => eprintln!("Missing implementation for : {} in base", missing),
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
        SourceDestination::Here(e) => e.replace("\\", "/"),
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

fn parse_interface_constant(cursor: &mut tree_sitter::TreeCursor<'_>, bytes: &[u8]) -> dto::Field {
    cursor.first_child();
    if cursor.node().kind() == "modifiers" {
        cursor.sibling();
    }
    let jtype = parse_jtype(&cursor.node(), bytes);
    cursor.sibling();
    cursor.first_child();
    let name = get_string(cursor, bytes);

    cursor.parent();
    cursor.parent();
    dto::Field {
        access: vec![],
        name,
        jtype,
    }
}

fn parse_enum_constant(node: tree_sitter::Node<'_>, bytes: &[u8]) -> dto::Field {
    let mut cursor = node.walk();
    cursor.first_child();

    dto::Field {
        access: vec![],
        name: get_string(&cursor, bytes),
        jtype: dto::JType::Void,
    }
}

fn parse_method(node: tree_sitter::Node<'_>, bytes: &[u8]) -> dto::Method {
    let mut cursor = node.walk();
    cursor.first_child();

    let mut method = dto::Method {
        access: vec![],
        name: "".to_owned(),
        parameters: vec![],
        throws: vec![],
        ret: dto::JType::Void,
    };

    loop {
        match cursor.node().kind() {
            "modifiers" => {
                method.access = parser_modifiers(get_string(&cursor, bytes));
            }
            "identifier" => method.name = get_string(&cursor, bytes),
            "formal_parameters" => {
                method.parameters = parse_formal_parameters(&mut cursor, bytes);
            }
            "block" => (),
            "type_parameters" => (),
            "throws" => {
                cursor.first_child();
                while cursor.sibling() {
                    if cursor.node().kind() == "," {
                        continue;
                    }
                    method.throws.push(parse_jtype(&cursor.node(), bytes));
                }
                cursor.parent();
            }
            _ => {
                method.ret = parse_jtype(&cursor.node(), bytes);
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
            "variable_declarator" => {
                cursor.first_child();
                field.name = get_string(&cursor, bytes);
                cursor.parent();
            }
            ";" => (),
            _ => {
                field.jtype = parse_jtype(&cursor.node(), bytes);
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
        let jtype = parse_jtype(&cursor.node(), bytes);

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
    cursor
        .node()
        .utf8_text(bytes)
        .unwrap_or_default()
        .to_owned()
}

fn parse_jtype(node: &tree_sitter::Node<'_>, bytes: &[u8]) -> dto::JType {
    match (node.kind(), node.utf8_text(bytes).unwrap_or_default()) {
        ("integral_type", "int") => dto::JType::Int,
        ("integral_type", "long") => dto::JType::Long,
        ("integral_type", "short") => dto::JType::Short,
        ("integral_type", "byte") => dto::JType::Byte,
        ("integral_type", "char") => dto::JType::Char,
        ("floating_point_type", "double") => dto::JType::Double,
        ("floating_point_type", "float") => dto::JType::Float,
        ("type_identifier", class) => dto::JType::Class(class.to_string()),
        ("boolean_type", "boolean") => dto::JType::Boolean,
        ("void_type", "void") => dto::JType::Void,
        ("array_type", _) => {
            let mut cursor = node.walk();
            cursor.first_child();
            let out = dto::JType::Array(Box::new(parse_jtype(&cursor.node(), bytes)));
            out
        }
        ("generic_type", _) => {
            let mut cursor = node.walk();
            cursor.first_child();
            let class = get_string(&cursor, bytes);
            cursor.sibling();
            cursor.first_child();
            cursor.sibling();
            let mut type_args = vec![];
            loop {
                match cursor.node().kind() {
                    ">" | "," => (),
                    _ => {
                        type_args.push(parse_jtype(&cursor.node(), bytes));
                    }
                };
                if !cursor.sibling() {
                    break;
                }
            }
            dto::JType::Generic(class, type_args)
        }
        (kind, text) => {
            eprintln!("unhandled type: {} {}", kind, text);
            dto::JType::Void
        }
    }
}
#[cfg(test)]
pub mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        dto::{self, Access, JType, Method, Parameter},
        loader::SourceDestination,
    };

    use super::load_java;

    #[test]
    fn jtype_recognition() {
        let content = r#"
package a.test;
public class Test {
  Logger LOG = Logger.getLogger(Test.class);
  boolean IS_ACTIVE = true;
  byte one_byte = 0;
  int one_int = 0;
  short one_short = 0;
  long one_long = 111l;
  double one_double = 0.0d;
  float one_float = 1.11f;
  char one_char = 'a';
  String one_string = "hihi";
  List<String> one_list = List.of("haha");
  Map<int, String> one_map = new HashMap();
  public static void main(String[] args) {}
}
        "#;
        let result = load_java(
            content.as_bytes(),
            SourceDestination::Here("/path/to/source/Test.java".to_string()),
        );
        assert_eq!(
            result.unwrap(),
            dto::Class {
                class_path: "a.test.Test".to_string(),
                source: "/path/to/source/Test.java".to_string(),
                access: vec![],
                name: "Test".to_string(),
                methods: vec![Method {
                    access: vec![Access::Static, Access::Public],
                    name: "main".to_string(),
                    parameters: vec![Parameter {
                        name: Some("args".to_string()),
                        jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_string())))
                    }],
                    ret: dto::JType::Void,
                    throws: vec![]
                }],
                fields: vec![
                    dto::Field {
                        access: vec![],
                        name: "LOG".to_string(),
                        jtype: dto::JType::Class("Logger".to_string())
                    },
                    dto::Field {
                        access: vec![],
                        name: "IS_ACTIVE".to_string(),
                        jtype: dto::JType::Boolean
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_byte".to_string(),
                        jtype: dto::JType::Byte
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_int".to_string(),
                        jtype: dto::JType::Int
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_short".to_string(),
                        jtype: dto::JType::Short
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_long".to_string(),
                        jtype: dto::JType::Long
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_double".to_string(),
                        jtype: dto::JType::Double
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_float".to_string(),
                        jtype: dto::JType::Float
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_char".to_string(),
                        jtype: dto::JType::Char
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_string".to_string(),
                        jtype: dto::JType::Class("String".to_string())
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_list".to_string(),
                        jtype: dto::JType::Generic(
                            "List".to_string(),
                            vec![dto::JType::Class("String".to_string())]
                        ),
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_map".to_string(),
                        jtype: dto::JType::Generic(
                            "Map".to_string(),
                            vec![dto::JType::Int, dto::JType::Class("String".to_string())]
                        ),
                    },
                ]
            }
        )
    }

    #[test]
    fn generic_type_declare() {
        let content = r#"
package a.test;
public class Test {
  public static <T> int add(Collection<T> list, T item){}
}
        "#;
        let result = load_java(
            content.as_bytes(),
            SourceDestination::Here("/path/to/source/Test.java".to_string()),
        );
        assert_eq!(
            result.unwrap(),
            dto::Class {
                class_path: "a.test.Test".to_string(),
                source: "/path/to/source/Test.java".to_string(),
                access: vec![],
                name: "Test".to_string(),
                methods: vec![Method {
                    access: vec![Access::Static, Access::Public],
                    name: "add".to_string(),
                    parameters: vec![
                        Parameter {
                            name: Some("list".to_string()),
                            jtype: dto::JType::Generic(
                                "Collection".to_string(),
                                vec![dto::JType::Class("T".to_string())]
                            )
                        },
                        Parameter {
                            name: Some("item".to_string()),
                            jtype: dto::JType::Class("T".to_string())
                        }
                    ],
                    ret: dto::JType::Int,
                    throws: vec![]
                }],
                fields: vec![]
            }
        )
    }

    pub fn thrower_data() -> dto::Class {
        dto::Class {
            class_path: "ch.emilycares.Thrower".to_string(),
            source: "/path/to/source/Thrower.java".to_string(),
            access: vec![],
            name: "Thrower".to_string(),
            methods: vec![
                Method {
                    access: vec![Access::Public],
                    name: "ioThrower".to_string(),
                    parameters: vec![],
                    ret: dto::JType::Void,
                    throws: vec![JType::Class("IOException".to_string())],
                },
                Method {
                    access: vec![Access::Public],
                    name: "ioThrower".to_string(),
                    parameters: vec![Parameter {
                        name: Some("a".to_string()),
                        jtype: dto::JType::Int,
                    }],
                    ret: dto::JType::Void,
                    throws: vec![
                        JType::Class("IOException".to_string()),
                        JType::Class("IOException".to_string()),
                    ],
                },
            ],
            fields: vec![],
        }
    }
    #[test]
    fn thrower() {
        let content = include_str!("../test/Thrower.java");
        let result = load_java(
            content.as_bytes(),
            SourceDestination::Here("/path/to/source/Thrower.java".to_string()),
        );
        assert_eq!(result.unwrap(), thrower_data())
    }

    #[test]
    fn interface() {
        let result = load_java(
            include_bytes!("../test/Interface.java"),
            SourceDestination::RelativeInFolder("/path/to/source".to_string()),
        );

        assert_eq!(
            result.unwrap(),
            dto::Class {
                class_path: "ch.emilycares.Constants".to_string(),
                source: "/path/to/source/ch/emilycares/Constants.java".to_string(),
                access: vec![],
                name: "Constants".to_string(),
                methods: vec![],
                fields: vec![
                    dto::Field {
                        access: vec![],
                        name: "CONSTANT_A".to_string(),
                        jtype: dto::JType::Class("String".to_string())
                    },
                    dto::Field {
                        access: vec![],
                        name: "CONSTANT_B".to_string(),
                        jtype: dto::JType::Class("String".to_string())
                    },
                    dto::Field {
                        access: vec![],
                        name: "CONSTANT_C".to_string(),
                        jtype: dto::JType::Class("String".to_string())
                    }
                ]
            }
        )
    }

    #[test]
    fn jenum() {
        let result = load_java(
            include_bytes!("../test/Enum.java"),
            SourceDestination::RelativeInFolder("/path/to/source".to_string()),
        );
        assert_eq!(
            result.unwrap(),
            dto::Class {
                class_path: "ch.emilycares.Variants".to_string(),
                source: "/path/to/source/ch/emilycares/Variants.java".to_string(),
                access: vec![],
                name: "Variants".to_string(),
                methods: vec![],
                fields: vec![
                    dto::Field {
                        access: vec![],
                        name: "A".to_string(),
                        jtype: dto::JType::Void
                    },
                    dto::Field {
                        access: vec![],
                        name: "B".to_string(),
                        jtype: dto::JType::Void
                    },
                    dto::Field {
                        access: vec![],
                        name: "C".to_string(),
                        jtype: dto::JType::Void
                    },
                ]
            }
        )
    }

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
