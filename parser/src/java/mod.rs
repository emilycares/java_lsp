use std::str::Utf8Error;

use tree_sitter_util::{CommentSkiper, TreesitterError};

use crate::{
    dto::{self, ImportUnit},
    loader::SourceDestination,
};

#[derive(Debug)]
pub enum ParseJavaError {
    Treesitter(TreesitterError),
    Utf8(Utf8Error),
    Class(dto::ClassError),
    Io(std::io::Error),
    UnknownJType(String, String),
    UnknownWildcard(String),
}
pub fn load_java(
    bytes: &[u8],
    source: SourceDestination,
) -> Result<crate::dto::Class, ParseJavaError> {
    let (_, tree) = tree_sitter_util::parse(bytes).map_err(ParseJavaError::Treesitter)?;
    load_java_tree(bytes, source, &tree)
}

pub fn load_java_tree(
    bytes: &[u8],
    source: SourceDestination,
    tree: &tree_sitter::Tree,
) -> Result<crate::dto::Class, ParseJavaError> {
    let mut imports = vec![];
    let mut methods = vec![];
    let mut fields = vec![];
    let mut super_interfaces = vec![];
    let mut class_name = None;
    let mut super_class = dto::SuperClass::None;
    let mut class_path_base: Option<String> = None;

    let mut cursor = tree.walk();
    cursor.first_child();
    if cursor.node().kind() == "package_declaration" {
        cursor.first_child();
        cursor.sibling();
        let package = get_string(&cursor, bytes).map_err(ParseJavaError::Utf8)?;
        class_path_base = Some(package.clone());
        imports.push(ImportUnit::Package(package));
        cursor.parent();
    }
    cursor.sibling();
    imports.extend(parse_import_declarations(bytes, &mut cursor));
    match cursor.node().kind() {
        "class_declaration" => {
            cursor.first_child();
            cursor.sibling();
            cursor.sibling();
            if cursor.node().kind() == "identifier" {
                let cn = get_string(&cursor, bytes).map_err(ParseJavaError::Utf8)?;
                class_name = Some(cn);
            }
            cursor.sibling();
            if cursor.node().kind() == "superclass" {
                cursor.first_child();
                cursor.sibling();
                super_class = parse_superclass(bytes, &imports, &cursor)?;
                cursor.parent();
                cursor.first_child();
            }
            cursor.first_child();
            while cursor.sibling() {
                match cursor.node().kind() {
                    "field_declaration" => fields.push(parse_field(cursor.node(), bytes)?),
                    "method_declaration" => methods.push(parse_method(cursor.node(), bytes)?),
                    "{" | "}" => (),
                    _ => (),
                }
            }
        }
        "interface_declaration" => {
            cursor.first_child();
            cursor.sibling();
            cursor.sibling();
            if cursor.node().kind() == "identifier" {
                let cn = get_string(&cursor, bytes).map_err(ParseJavaError::Utf8)?;
                class_name = Some(cn);
            }
            cursor.sibling();
            if cursor.node().kind() == "type_parameters" {
                cursor.sibling();
            }
            if cursor.node().kind() == "extends_interfaces" {
                cursor.first_child();
                cursor.sibling();
                cursor.first_child();
                super_interfaces.push(parse_superclass(bytes, &imports, &cursor)?);
                while cursor.sibling() {
                    if cursor.node().kind() == "," {
                        cursor.sibling();
                    }
                    super_interfaces.push(parse_superclass(bytes, &imports, &cursor)?);
                }
                cursor.parent();
                cursor.parent();
                cursor.sibling();
            }
            cursor.first_child();
            while cursor.sibling() {
                match cursor.node().kind() {
                    "constant_declaration" => {
                        fields.push(parse_interface_constant(&mut cursor, bytes)?)
                    }
                    "method_declaration" => {
                        methods.push(parse_interface_method(&mut cursor, bytes)?)
                    }
                    "," | "{" | "}" => (),
                    _ => {}
                }
            }
        }
        "enum_declaration" => {
            cursor.first_child();
            cursor.sibling();
            cursor.sibling();
            if cursor.node().kind() == "identifier" {
                let cn = get_string(&cursor, bytes).map_err(ParseJavaError::Utf8)?;
                class_name = Some(cn);
            }
            cursor.sibling();
            cursor.first_child();
            while cursor.sibling() {
                match cursor.node().kind() {
                    "enum_constant" => fields.push(parse_enum_constant(cursor.node(), bytes)?),
                    "," | "{" | "}" => (),
                    _ => (),
                }
            }
        }
        "annotation_type_declaration" => {
            cursor.first_child();
            cursor.sibling();
            cursor.sibling();
            if cursor.node().kind() == "identifier" {
                let cn = get_string(&cursor, bytes).map_err(ParseJavaError::Utf8)?;
                class_name = Some(cn);
            }
            cursor.sibling();
            cursor.first_child();
            while cursor.sibling() {
                match cursor.node().kind() {
                    "annotation_type_element_declaration" => fields.push(
                        parse_annotation_type_element_declaration(cursor.node(), bytes)?,
                    ),
                    "," | "{" | "}" => (),
                    _ => {}
                }
            }

            cursor.parent();
        }
        _ => (),
    }

    let Some(name) = class_name else {
        return Err(ParseJavaError::Class(dto::ClassError::UnknownClassName));
    };
    let Some(class_path_base) = class_path_base else {
        return Err(ParseJavaError::Class(dto::ClassError::UnknownClassPath));
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
        super_class,
        super_interfaces,
        imports,
        name,
        methods,
        fields,
    })
}

fn parse_superclass(
    bytes: &[u8],
    imports: &[ImportUnit],
    cursor: &tree_sitter::TreeCursor<'_>,
) -> Result<dto::SuperClass, ParseJavaError> {
    Ok(match parse_jtype(&cursor.node(), bytes, &vec![])? {
        dto::JType::Class(c) | dto::JType::Generic(c, _) => match imports
            .iter()
            .find_map(|a| a.get_imported_class_package(&c))
        {
            Some(resolved) => dto::SuperClass::ClassPath(resolved),
            None => dto::SuperClass::Name(c),
        },
        _ => dto::SuperClass::None,
    })
}

pub fn parse_import_declarations(
    bytes: &[u8],
    cursor: &mut tree_sitter::TreeCursor,
) -> Vec<ImportUnit> {
    let mut out = vec![];
    while let "import_declaration" = cursor.node().kind() {
        cursor.first_child();
        cursor.sibling();
        let mut stat = false;
        let mut prefix = false;
        if cursor.node().kind() == "static" {
            stat = true;
            cursor.sibling();
        }

        // skip import when not correctly formated
        if cursor.node().kind() == "scoped_identifier" {
            let class_path = cursor
                .node()
                .utf8_text(bytes)
                .unwrap_or_default()
                .to_string();
            if cursor.sibling() {
                if cursor.node().kind() == "." {
                    cursor.sibling();
                }
                if cursor.node().kind() == "asterisk" {
                    prefix = true;
                }
            }

            let imp = match (stat, prefix) {
                (true, true) => ImportUnit::StaticPrefix(class_path),
                (true, false) => match class_path.rsplit_once(".") {
                    Some((class, method)) => {
                        match method.chars().next().unwrap_or_default().is_lowercase() {
                            true => {
                                ImportUnit::StaticClassMethod(class.to_string(), method.to_string())
                            }
                            false => ImportUnit::StaticClass(class_path),
                        }
                    }
                    None => ImportUnit::StaticClass(class_path),
                },
                (false, true) => ImportUnit::Prefix(class_path),
                (false, false) => ImportUnit::Class(class_path),
            };
            out.push(imp);
        }
        cursor.parent();
        cursor.sibling();
    }
    out
}

fn parse_interface_method(
    cursor: &mut tree_sitter::TreeCursor<'_>,
    bytes: &[u8],
) -> Result<dto::Method, ParseJavaError> {
    cursor.first_child();
    if cursor.node().kind() == "modifiers" {
        cursor.sibling();
    }
    let mut type_parameters = vec![];
    if cursor.node().kind() == "type_parameters" {
        parse_type_parameters(cursor, bytes, &mut type_parameters)?;
        cursor.sibling();
    }
    let jtype = parse_jtype(&cursor.node(), bytes, &type_parameters)?;
    cursor.sibling();
    let name = get_string(cursor, bytes).map_err(ParseJavaError::Utf8)?;
    cursor.sibling();
    let parameters = parse_formal_parameters(cursor, bytes, &type_parameters)?;
    let mut method = dto::Method {
        access: vec![],
        name,
        parameters,
        throws: vec![],
        ret: jtype,
        source: None,
    };
    cursor.sibling();
    if cursor.node().kind() == "throws" {
        method.throws = parse_throws(bytes, cursor)?;
    }
    cursor.parent();
    Ok(method)
}

fn parse_type_parameters(
    cursor: &mut tree_sitter::TreeCursor<'_>,
    bytes: &[u8],
    type_parameters: &mut Vec<String>,
) -> Result<(), ParseJavaError> {
    cursor.first_child();
    cursor.sibling();
    let name = get_string(cursor, bytes).map_err(ParseJavaError::Utf8)?;
    type_parameters.push(name);
    cursor.parent();
    Ok(())
}

fn parse_interface_constant(
    cursor: &mut tree_sitter::TreeCursor<'_>,
    bytes: &[u8],
) -> Result<dto::Field, ParseJavaError> {
    cursor.first_child();
    if cursor.node().kind() == "modifiers" {
        cursor.sibling();
    }
    let jtype = parse_jtype(&cursor.node(), bytes, &vec![])?;
    cursor.sibling();
    cursor.first_child();
    let name = get_string(cursor, bytes).map_err(ParseJavaError::Utf8)?;

    cursor.parent();
    cursor.parent();
    Ok(dto::Field {
        access: vec![],
        name,
        jtype,
        source: None,
    })
}

fn parse_enum_constant(
    node: tree_sitter::Node<'_>,
    bytes: &[u8],
) -> Result<dto::Field, ParseJavaError> {
    let mut cursor = node.walk();
    cursor.first_child();

    Ok(dto::Field {
        access: vec![],
        name: get_string(&cursor, bytes).map_err(ParseJavaError::Utf8)?,
        jtype: dto::JType::Void,
        source: None,
    })
}

fn parse_annotation_type_element_declaration(
    node: tree_sitter::Node<'_>,
    bytes: &[u8],
) -> Result<dto::Field, ParseJavaError> {
    let mut cursor = node.walk();
    cursor.first_child();
    let jtype = parse_jtype(&cursor.node(), bytes, &vec![])?;
    cursor.sibling();
    Ok(dto::Field {
        access: vec![],
        name: get_string(&cursor, bytes).map_err(ParseJavaError::Utf8)?,
        jtype,
        source: None,
    })
}

fn parse_method(node: tree_sitter::Node<'_>, bytes: &[u8]) -> Result<dto::Method, ParseJavaError> {
    let mut cursor = node.walk();
    cursor.first_child();

    let mut method = dto::Method {
        access: vec![],
        name: "".to_owned(),
        parameters: vec![],
        throws: vec![],
        ret: dto::JType::Void,
        source: None,
    };

    loop {
        match cursor.node().kind() {
            "modifiers" => {
                method.access =
                    parser_modifiers(get_string(&cursor, bytes).map_err(ParseJavaError::Utf8)?);
            }
            "identifier" => {
                method.name = get_string(&cursor, bytes).map_err(ParseJavaError::Utf8)?
            }
            "formal_parameters" => {
                method.parameters = parse_formal_parameters(&mut cursor, bytes, &vec![])?
            }
            "block" => (),
            "type_parameters" => (),
            "throws" => {
                method.throws = parse_throws(bytes, &mut cursor)?;
            }
            _ => {
                method.ret = parse_jtype(&cursor.node(), bytes, &vec![])?;
            }
        };
        if !cursor.sibling() {
            break;
        }
    }

    Ok(method)
}

fn parse_throws(
    bytes: &[u8],
    cursor: &mut tree_sitter::TreeCursor<'_>,
) -> Result<Vec<dto::JType>, ParseJavaError> {
    let mut out = vec![];
    cursor.first_child();
    while cursor.sibling() {
        if cursor.node().kind() == "," {
            continue;
        }
        out.push(parse_jtype(&cursor.node(), bytes, &vec![])?);
    }
    cursor.parent();
    Ok(out)
}

fn parse_field(node: tree_sitter::Node<'_>, bytes: &[u8]) -> Result<dto::Field, ParseJavaError> {
    let mut cursor = node.walk();
    cursor.first_child();

    let mut field = dto::Field {
        access: vec![],
        name: "".to_owned(),
        jtype: dto::JType::Void,
        source: None,
    };

    loop {
        match cursor.node().kind() {
            "modifiers" => {
                field.access =
                    parser_modifiers(get_string(&cursor, bytes).map_err(ParseJavaError::Utf8)?);
            }
            "variable_declarator" => {
                cursor.first_child();
                field.name = get_string(&cursor, bytes).map_err(ParseJavaError::Utf8)?;
                cursor.parent();
            }
            ";" => (),
            _ => {
                field.jtype = parse_jtype(&cursor.node(), bytes, &vec![])?;
            }
        };
        if !cursor.sibling() {
            break;
        }
    }

    Ok(field)
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
    type_parameters: &Vec<String>,
) -> Result<Vec<dto::Parameter>, ParseJavaError> {
    let mut out = vec![];
    cursor.first_child();
    while cursor.sibling() {
        if cursor.node().kind() != "formal_parameter" {
            continue;
        }
        cursor.first_child();
        if cursor.node().kind() == "modifiers" {
            cursor.sibling();
        }
        let jtype = parse_jtype(&cursor.node(), bytes, type_parameters)?;

        cursor.sibling();
        out.push(dto::Parameter {
            name: Some(get_string(&*cursor, bytes).map_err(ParseJavaError::Utf8)?),
            jtype,
        });
        cursor.parent();
    }
    cursor.parent();
    Ok(out)
}

fn get_string(cursor: &tree_sitter::TreeCursor<'_>, bytes: &[u8]) -> Result<String, Utf8Error> {
    cursor.node().utf8_text(bytes).map(|a| a.to_string())
}

pub fn parse_jtype(
    node: &tree_sitter::Node<'_>,
    bytes: &[u8],
    type_parameters: &Vec<String>,
) -> Result<dto::JType, ParseJavaError> {
    let text = node.utf8_text(bytes).map_err(ParseJavaError::Utf8)?;
    match (node.kind(), text) {
        ("integral_type", "int") => Ok(dto::JType::Int),
        ("integral_type", "long") => Ok(dto::JType::Long),
        ("integral_type", "short") => Ok(dto::JType::Short),
        ("integral_type", "byte") => Ok(dto::JType::Byte),
        ("integral_type", "char") => Ok(dto::JType::Char),
        ("floating_point_type", "double") => Ok(dto::JType::Double),
        ("floating_point_type", "float") => Ok(dto::JType::Float),
        ("type_identifier", class) => {
            let class = class.to_string();
            if type_parameters.contains(&class) {
                Ok(dto::JType::Parameter(class))
            } else {
                Ok(dto::JType::Class(class))
            }
        }
        ("scoped_type_identifier", class) => {
            Ok(dto::JType::Class(class.replace('.', "$").to_string()))
        }
        ("boolean_type", "boolean") => Ok(dto::JType::Boolean),
        ("void_type", "void") => Ok(dto::JType::Void),
        ("wildcard", "?") => Ok(dto::JType::Wildcard),
        ("wildcard", w) => {
            if let Some((_, c)) = w.rsplit_once(' ') {
                let class = c.to_string();
                if type_parameters.contains(&class) {
                    return Ok(dto::JType::Parameter(class));
                } else {
                    return Ok(dto::JType::Class(class));
                }
            }
            Err(ParseJavaError::UnknownWildcard(w.to_string()))
        }
        ("array_type", _) => {
            let mut cursor = node.walk();
            cursor.first_child();
            let out = dto::JType::Array(Box::new(parse_jtype(
                &cursor.node(),
                bytes,
                type_parameters,
            )?));
            Ok(out)
        }
        ("generic_type", _) => {
            let mut cursor = node.walk();
            cursor.first_child();
            let class = get_string(&cursor, bytes).map_err(ParseJavaError::Utf8)?;
            cursor.sibling();
            cursor.first_child();
            cursor.sibling();
            let mut type_args = vec![];
            loop {
                match cursor.node().kind() {
                    ">" | "," => (),
                    _ => {
                        type_args.push(parse_jtype(&cursor.node(), bytes, type_parameters)?);
                    }
                };
                if !cursor.sibling() {
                    break;
                }
            }
            Ok(dto::JType::Generic(class, type_args))
        }
        (kind, text) => {
            eprintln!("unhandled type: {} {}", kind, text);
            Err(ParseJavaError::UnknownJType(
                kind.to_string(),
                text.to_string(),
            ))
        }
    }
}
#[cfg(test)]
pub mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        dto::{self, Access, ImportUnit, JType, Method, Parameter, SuperClass},
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
                imports: vec![ImportUnit::Package("a.test".to_string())],
                name: "Test".to_string(),
                methods: vec![Method {
                    access: vec![Access::Static, Access::Public],
                    name: "main".to_string(),
                    parameters: vec![Parameter {
                        name: Some("args".to_string()),
                        jtype: dto::JType::Array(Box::new(dto::JType::Class("String".to_string())))
                    }],
                    ret: dto::JType::Void,
                    throws: vec![],
                    source: None
                }],
                fields: vec![
                    dto::Field {
                        access: vec![],
                        name: "LOG".to_string(),
                        jtype: dto::JType::Class("Logger".to_string()),
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "IS_ACTIVE".to_string(),
                        jtype: dto::JType::Boolean,
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_byte".to_string(),
                        jtype: dto::JType::Byte,
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_int".to_string(),
                        jtype: dto::JType::Int,
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_short".to_string(),
                        jtype: dto::JType::Short,
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_long".to_string(),
                        jtype: dto::JType::Long,
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_double".to_string(),
                        jtype: dto::JType::Double,
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_float".to_string(),
                        jtype: dto::JType::Float,
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_char".to_string(),
                        jtype: dto::JType::Char,
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_string".to_string(),
                        jtype: dto::JType::Class("String".to_string()),
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_list".to_string(),
                        jtype: dto::JType::Generic(
                            "List".to_string(),
                            vec![dto::JType::Class("String".to_string())]
                        ),
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "one_map".to_string(),
                        jtype: dto::JType::Generic(
                            "Map".to_string(),
                            vec![dto::JType::Int, dto::JType::Class("String".to_string())]
                        ),
                        source: None
                    },
                ],
                ..Default::default()
            }
        )
    }

    #[test]
    fn super_class() {
        let content = r#"
package a.test;
public class Test extends a { }
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
                super_class: dto::SuperClass::Name("a".to_string()),
                imports: vec![ImportUnit::Package("a.test".to_string())],
                name: "Test".to_string(),
                ..Default::default()
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
                imports: vec![ImportUnit::Package("a.test".to_string())],
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
                    throws: vec![],
                    source: None,
                }],
                ..Default::default()
            }
        )
    }

    pub fn thrower_data() -> dto::Class {
        dto::Class {
            class_path: "ch.emilycares.Thrower".to_string(),
            source: "/path/to/source/Thrower.java".to_string(),
            imports: vec![
                ImportUnit::Package("ch.emilycares".to_string()),
                ImportUnit::Class("java.io.IOException".to_string()),
            ],
            name: "Thrower".to_string(),
            methods: vec![
                Method {
                    access: vec![Access::Public],
                    name: "ioThrower".to_string(),
                    parameters: vec![],
                    ret: dto::JType::Void,
                    throws: vec![JType::Class("IOException".to_string())],
                    source: None,
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
                    source: None,
                },
            ],
            ..Default::default()
        }
    }
    #[test]
    fn thrower() {
        let content = include_str!("../../test/Thrower.java");
        let result = load_java(
            content.as_bytes(),
            SourceDestination::Here("/path/to/source/Thrower.java".to_string()),
        );
        assert_eq!(result.unwrap(), thrower_data())
    }

    #[test]
    fn interface_constants() {
        let result = load_java(
            include_bytes!("../../test/Constants.java"),
            SourceDestination::RelativeInFolder("/path/to/source".to_string()),
        );

        assert_eq!(
            result.unwrap(),
            dto::Class {
                class_path: "ch.emilycares.Constants".to_string(),
                source: "/path/to/source/ch/emilycares/Constants.java".to_string(),
                imports: vec![
                    ImportUnit::Package("ch.emilycares".to_string()),
                    ImportUnit::Class("jdk.net.Sockets".to_string()),
                    ImportUnit::Class("java.io.IOException".to_string()),
                    ImportUnit::Class("java.net.Socket".to_string()),
                ],
                name: "Constants".to_string(),
                methods: vec![
                    dto::Method {
                        access: vec![],
                        name: "display".to_string(),
                        parameters: vec![],
                        throws: vec![],
                        ret: JType::Void,
                        source: None
                    },
                    dto::Method {
                        access: vec![],
                        name: "createSocket".to_string(),
                        parameters: vec![
                            dto::Parameter {
                                name: Some("hostname".to_string()),
                                jtype: JType::Class("String".to_string())
                            },
                            dto::Parameter {
                                name: Some("port".to_string()),
                                jtype: JType::Int
                            }
                        ],
                        throws: vec![JType::Class("IOException".to_string())],
                        ret: JType::Class("Socket".to_string()),
                        source: None
                    }
                ],
                fields: vec![
                    dto::Field {
                        access: vec![],
                        name: "CONSTANT_A".to_string(),
                        jtype: dto::JType::Class("String".to_string()),
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "CONSTANT_B".to_string(),
                        jtype: dto::JType::Class("String".to_string()),
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "CONSTANT_C".to_string(),
                        jtype: dto::JType::Class("String".to_string()),
                        source: None
                    }
                ],
                ..Default::default()
            }
        )
    }

    #[test]
    fn interface_base() {
        let result = load_java(
            include_bytes!("../../test/InterfaceBase.java"),
            SourceDestination::RelativeInFolder("/path/to/source".to_string()),
        );

        assert_eq!(
            result.unwrap(),
            dto::Class {
                class_path: "ch.emilycares.InterfaceBase".to_string(),
                source: "/path/to/source/ch/emilycares/InterfaceBase.java".to_string(),
                imports: vec![
                    ImportUnit::Package("ch.emilycares".to_string()),
                    ImportUnit::Class("java.util.function.IntFunction".to_string()),
                    ImportUnit::Class("java.util.stream.Stream".to_string()),
                ],
                name: "InterfaceBase".to_string(),
                methods: vec![
                    dto::Method {
                        access: vec![],
                        name: "mapToObj".to_string(),
                        parameters: vec![Parameter {
                            name: Some("mapper".to_string()),
                            jtype: JType::Generic(
                                "IntFunction".to_string(),
                                vec![JType::Parameter("U".to_string())]
                            )
                        }],
                        throws: vec![],
                        ret: JType::Generic(
                            "Stream".to_string(),
                            vec![JType::Parameter("U".to_string())]
                        ),
                        source: None
                    },
                    dto::Method {
                        access: vec![],
                        name: "a".to_string(),
                        parameters: vec![Parameter {
                            name: Some("arg".to_string()),
                            jtype: JType::Parameter("A".to_string())
                        }],
                        throws: vec![],
                        ret: JType::Parameter("A".to_string()),
                        source: None
                    },
                ],
                fields: vec![],
                ..Default::default()
            }
        )
    }

    #[test]
    fn jenum() {
        let result = load_java(
            include_bytes!("../../test/Variants.java"),
            SourceDestination::RelativeInFolder("/path/to/source".to_string()),
        );
        assert_eq!(
            result.unwrap(),
            dto::Class {
                class_path: "ch.emilycares.Variants".to_string(),
                source: "/path/to/source/ch/emilycares/Variants.java".to_string(),
                imports: vec![ImportUnit::Package("ch.emilycares".to_string())],
                name: "Variants".to_string(),
                fields: vec![
                    dto::Field {
                        access: vec![],
                        name: "A".to_string(),
                        jtype: dto::JType::Void,
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "B".to_string(),
                        jtype: dto::JType::Void,
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "C".to_string(),
                        jtype: dto::JType::Void,
                        source: None
                    },
                ],
                ..Default::default()
            }
        )
    }

    #[test]
    fn jannotation() {
        let result = load_java(
            include_bytes!("../../test/Annotation.java"),
            SourceDestination::RelativeInFolder("/path/to/source".to_string()),
        );
        assert_eq!(
            result.unwrap(),
            dto::Class {
                class_path: "ch.emilycares.Annotation".to_string(),
                source: "/path/to/source/ch/emilycares/Annotation.java".to_string(),
                imports: vec![ImportUnit::Package("ch.emilycares".to_string())],
                name: "Annotation".to_string(),
                fields: vec![
                    dto::Field {
                        access: vec![],
                        name: "value".to_string(),
                        jtype: dto::JType::Int,
                        source: None
                    },
                    dto::Field {
                        access: vec![],
                        name: "text".to_string(),
                        jtype: dto::JType::Class("String".to_string()),
                        source: None
                    },
                ],
                ..Default::default()
            }
        )
    }

    #[test]
    fn everything() {
        let result = load_java(
            include_bytes!("../../test/Everything.java"),
            SourceDestination::None,
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
                imports: vec![
                    ImportUnit::Package("a.test".to_string()),
                    ImportUnit::Class("jakarta.inject.Inject".to_string()),
                    ImportUnit::Class("jakarta.ws.rs.GET".to_string()),
                    ImportUnit::Class("jakarta.ws.rs.Path".to_string())
                ],
                name: "Test".to_string(),
                ..Default::default()
            }
        );
    }
    #[test]
    fn super_interfaces() {
        let result = load_java(
            include_bytes!("../../test/SuperInterface.java"),
            SourceDestination::None,
        )
        .unwrap();
        assert_eq!(
            result,
            dto::Class {
                imports: vec![
                    ImportUnit::Package("ch.emilycares".to_string()),
                    ImportUnit::Class("java.util.Collection".to_string()),
                    ImportUnit::Class("java.util.List".to_string()),
                    ImportUnit::Class("java.util.stream.Stream".to_string()),
                    ImportUnit::Class("java.util.stream.StreamSupport".to_string())
                ],
                class_path: "ch.emilycares.SuperInterface".to_string(),
                name: "SuperInterface".to_string(),
                super_interfaces: vec![
                    SuperClass::ClassPath("java.util.Collection".to_string()),
                    SuperClass::ClassPath("java.util.List".to_string()),
                ],
                methods: vec![dto::Method {
                    access: vec![],
                    name: "stream".to_string(),
                    parameters: vec![],
                    ret: dto::JType::Generic(
                        "Stream".to_string(),
                        vec![JType::Class("E".to_string())]
                    ),
                    throws: vec![],
                    source: None,
                },],
                ..Default::default()
            }
        )
    }
}
