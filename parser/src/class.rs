use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR};

use crate::dto::{self, ClassError, ImportUnit, JType, Parameter};
use crate::loader::SourceDestination;
use classfile_parser::attribute_info::{AttributeInfo, CodeAttribute};
use classfile_parser::code_attribute::LocalVariableTableAttribute;
use classfile_parser::constant_info::ConstantInfo;
use classfile_parser::field_info::{FieldAccessFlags, FieldInfo};
use classfile_parser::method_info::MethodAccessFlags;
use classfile_parser::{class_parser, ClassAccessFlags, ClassFile};
use itertools::Itertools;

pub fn load_class(
    bytes: &[u8],
    class_path: String,
    source: SourceDestination,
) -> Result<dto::Class, dto::ClassError> {
    let res = class_parser(bytes);
    match res {
        Result::Ok((_, c)) => {
            let code_attribute = parse_code_attribute(&c, &c.attributes);
            let mut used_classes = parse_used_classes(&c, code_attribute);

            let methods: Vec<_> = c
                .methods
                .iter()
                .filter_map(|method| {
                    let code_attribute = parse_code_attribute(&c, &method.attributes);
                    used_classes.extend(parse_used_classes(&c, code_attribute));

                    let method = parse_method(&c, method);
                    if let Some(ref m) = method {
                        used_classes.extend(
                            m.parameters
                                .iter()
                                .filter(|i| {
                                    matches!(
                                        i.jtype,
                                        JType::Class(_) | JType::Array(_) | JType::Generic(_, _)
                                    )
                                })
                                .flat_map(|i| jtype_class_names(i.jtype.clone())),
                        );
                        used_classes.extend(jtype_class_names(m.ret.clone()));
                    }
                    method
                })
                .filter(|m| m.name != "<init>")
                //.filter(|f| !f.access.contains(&dto::Access::Private))
                .collect();
            let fields: Vec<_> = c
                .fields
                .iter()
                .filter_map(|field| {
                    let field = parse_field(&c, field);

                    if let Some(ref f) = field {
                        used_classes.extend(jtype_class_names(f.jtype.clone()));
                    }

                    field
                })
                //.filter(|f| !f.access.contains(&dto::Access::Private))
                .collect();

            let name = lookup_class_name(&c, c.this_class.into()).expect("Class should have name");
            let package = class_path.trim_end_matches(&name).trim_end_matches(".");
            let mut imports = vec![ImportUnit::Package(package.to_string())];
            imports.extend(
                used_classes
                    .into_iter()
                    .filter(|i| *i != class_path)
                    .unique()
                    .map(ImportUnit::Class),
            );

            let source_file = parse_source_file(&c);
            let source = match source {
                SourceDestination::RelativeInFolder(e) => match source_file {
                    Some(file_name) => format!("{}{}{}", e, MAIN_SEPARATOR, file_name),
                    // Guessing .java extension
                    None => format!(
                        "{}{}{}.java",
                        e,
                        MAIN_SEPARATOR,
                        &class_path.replace(".", MAIN_SEPARATOR_STR)
                    ),
                },
                SourceDestination::Here(e) => e,
                SourceDestination::None => "".to_string(),
            };
            let super_interfaces: Vec<_> = c
                .interfaces
                .iter()
                .map(|index| match lookup_class_name(&c, *index as usize) {
                    Some(c) => dto::SuperClass::Name(c),
                    None => dto::SuperClass::None,
                })
                .collect();

            Ok(dto::Class {
                source,
                class_path,
                super_interfaces,
                super_class: match lookup_class_name(&c, c.super_class.into()) {
                    Some(c) if c == "Object" => dto::SuperClass::None,
                    Some(c) => dto::SuperClass::Name(c),
                    None => dto::SuperClass::None,
                },
                imports,
                access: parse_class_access(c.access_flags),
                name,
                methods,
                fields,
            })
        }
        _ => Err(ClassError::ParseError),
    }
}

fn parse_source_file(c: &ClassFile) -> Option<String> {
    c.attributes
        .iter()
        .filter_map(|attribute_info| {
            match lookup_string(c, attribute_info.attribute_name_index)?.as_str() {
                "SourceFile" => classfile_parser::attribute_info::sourcefile_attribute_parser(
                    &attribute_info.info,
                )
                .ok(),
                _ => None,
            }
        })
        .find_map(|i| lookup_string(c, i.1.sourcefile_index))
}

fn lookup_class_name(c: &ClassFile, index: usize) -> Option<String> {
    match c.const_pool.get(index.saturating_sub(1)) {
        Some(ConstantInfo::Class(class)) => lookup_string(c, class.name_index)
            .expect("Class to have name")
            .split("/")
            .last()
            .map(|a| a.to_string()),
        _ => None,
    }
}

fn parse_field(c: &ClassFile, field: &FieldInfo) -> Option<dto::Field> {
    Some(dto::Field {
        access: parse_field_access(field),
        name: lookup_string(c, field.name_index)?,
        jtype: parse_field_descriptor(&lookup_string(c, field.descriptor_index)?)?,
        source: None,
    })
}

fn parse_method(
    c: &ClassFile,
    method: &classfile_parser::method_info::MethodInfo,
) -> Option<dto::Method> {
    let (params, ret) = parse_method_descriptor(&lookup_string(c, method.descriptor_index)?);

    let mut params = params.into_iter();
    let mut parameters: Vec<dto::Parameter> = method
        .attributes
        .iter()
        .filter_map(|attribute_info| {
            match lookup_string(c, attribute_info.attribute_name_index)?.as_str() {
                "MethodParameters" => {
                    classfile_parser::attribute_info::method_parameters_attribute_parser(
                        &attribute_info.info,
                    )
                    .ok()
                }
                _ => None,
            }
        })
        .flat_map(|method_parameters| {
            method_parameters
                .1
                .parameters
                .into_iter()
                .filter_map(|pa| {
                    Some(dto::Parameter {
                        name: lookup_string(c, pa.name_index).and_then(|s| {
                            if s.is_empty() {
                                return None;
                            }
                            Some(s)
                        }),
                        jtype: params.next()?,
                    })
                })
                .collect::<Vec<Parameter>>()
        })
        .collect();
    // Remaining method descriptor data as params
    {
        for jtype in params {
            parameters.push(dto::Parameter { name: None, jtype });
        }
    }
    let throws: Vec<dto::JType> = method
        .attributes
        .iter()
        .filter_map(|attribute_info| {
            match lookup_string(c, attribute_info.attribute_name_index)?.as_str() {
                "Exceptions" => classfile_parser::attribute_info::exceptions_attribute_parser(
                    &attribute_info.info,
                )
                .ok(),
                _ => None,
            }
        })
        .flat_map(|i| {
            let exception_table = i.1.exception_table;
            exception_table
                .into_iter()
                .filter_map(|ex| c.const_pool.get((ex - 1) as usize))
                .filter_map(|ex_class| match ex_class {
                    ConstantInfo::Class(ex_class) => lookup_string(c, ex_class.name_index),
                    _ => None,
                })
                .filter_map(|name| {
                    if let Some((_, name)) = name.rsplit_once("/") {
                        return Some(name.to_string());
                    }
                    None
                })
                .map(dto::JType::Class)
        })
        .collect();
    Some(dto::Method {
        access: parse_method_access(method),
        name: lookup_string(c, method.name_index)?,
        parameters,
        ret,
        throws,
        source: None,
    })
}

fn parse_used_classes(c: &ClassFile, code_attribute: Option<CodeAttribute>) -> Vec<String> {
    if let Some(code_attribute) = code_attribute {
        let local_variable_table_attributes: Vec<LocalVariableTableAttribute> = code_attribute
            .attributes
            .iter()
            .filter_map(|attribute_info| {
                match lookup_string(c, attribute_info.attribute_name_index)?.as_str() {
                    "LocalVariableTable" => {
                        classfile_parser::code_attribute::local_variable_table_parser(
                            &attribute_info.info,
                        )
                        .ok()
                    }
                    _ => None,
                }
            })
            .map(|a| a.1)
            .collect();
        let types: Vec<JType> = local_variable_table_attributes
            .iter()
            .flat_map(|i| &i.items)
            .filter_map(|i| lookup_string(c, i.descriptor_index))
            .filter_map(|i| parse_field_descriptor(&i))
            .collect();
        return types
            .iter()
            .flat_map(|i| jtype_class_names(i.clone()))
            .collect();
    }
    vec![]
}

fn jtype_class_names(i: JType) -> Vec<String> {
    match i {
        JType::Class(class) => vec![class],
        JType::Array(jtype) => jtype_class_names(*jtype),
        JType::Generic(class, jtypes) => {
            let mut out = vec![class];
            let e: Vec<String> = jtypes.into_iter().flat_map(jtype_class_names).collect();
            out.extend(e);
            out
        }
        _ => vec![],
    }
}

fn parse_code_attribute(c: &ClassFile, attributes: &[AttributeInfo]) -> Option<CodeAttribute> {
    attributes
        .iter()
        .find_map(|attribute_info| {
            match lookup_string(c, attribute_info.attribute_name_index)?.as_str() {
                "Code" => {
                    classfile_parser::attribute_info::code_attribute_parser(&attribute_info.info)
                        .ok()
                }
                _ => None,
            }
        })
        .map(|i| i.1)
}

fn parse_class_access(flags: ClassAccessFlags) -> Vec<dto::Access> {
    let mut access = vec![];
    if flags == ClassAccessFlags::PUBLIC {
        access.push(dto::Access::Public);
    }
    if flags == ClassAccessFlags::FINAL {
        access.push(dto::Access::Final);
    }
    if flags == ClassAccessFlags::SUPER {
        access.push(dto::Access::Super);
    }
    if flags == ClassAccessFlags::INTERFACE {
        access.push(dto::Access::Interface);
    }
    if flags == ClassAccessFlags::SYNTHETIC {
        access.push(dto::Access::Synthetic);
    }
    if flags == ClassAccessFlags::ANNOTATION {
        access.push(dto::Access::Annotation);
    }
    if flags == ClassAccessFlags::ENUM {
        access.push(dto::Access::Enum);
    }
    access
}

fn parse_method_access(method: &classfile_parser::method_info::MethodInfo) -> Vec<dto::Access> {
    let mut access = vec![];
    if method.access_flags == MethodAccessFlags::PUBLIC {
        access.push(dto::Access::Public);
    }
    if method.access_flags == MethodAccessFlags::PRIVATE {
        access.push(dto::Access::Private);
    }
    if method.access_flags == MethodAccessFlags::PROTECTED {
        access.push(dto::Access::Protected);
    }
    if method.access_flags == MethodAccessFlags::STATIC {
        access.push(dto::Access::Static);
    }
    if method.access_flags == MethodAccessFlags::FINAL {
        access.push(dto::Access::Final);
    }
    if method.access_flags == MethodAccessFlags::ABSTRACT {
        access.push(dto::Access::Abstract);
    }
    if method.access_flags == MethodAccessFlags::SYNTHETIC {
        access.push(dto::Access::Synthetic);
    }
    access
}

fn parse_field_access(method: &FieldInfo) -> Vec<dto::Access> {
    let mut access = vec![];
    if method.access_flags == FieldAccessFlags::PUBLIC {
        access.push(dto::Access::Public);
    }
    if method.access_flags == FieldAccessFlags::PRIVATE {
        access.push(dto::Access::Private);
    }
    if method.access_flags == FieldAccessFlags::PROTECTED {
        access.push(dto::Access::Protected);
    }
    if method.access_flags == FieldAccessFlags::STATIC {
        access.push(dto::Access::Static);
    }
    if method.access_flags == FieldAccessFlags::FINAL {
        access.push(dto::Access::Final);
    }
    if method.access_flags == FieldAccessFlags::SYNTHETIC {
        access.push(dto::Access::Synthetic);
    }
    access
}

fn lookup_string(c: &ClassFile, index: u16) -> Option<String> {
    if index == 0 {
        return None;
    }
    let con = &c.const_pool[(index - 1) as usize];
    match con {
        ConstantInfo::Utf8(utf8) => Some(utf8.utf8_string.clone()),
        _ => None,
    }
}

fn parse_method_descriptor(descriptor: &str) -> (Vec<dto::JType>, dto::JType) {
    let mut param_types = Vec::new();
    let mut chars = descriptor.chars();
    let current = chars.next();
    match current {
        Some('(') => {
            while let Some(c) = chars.next() {
                if c == ')' {
                    break;
                }
                param_types.push(parse_field_type(Some(c), &mut chars));
            }

            let return_type = parse_field_type(chars.next(), &mut chars);
            (param_types, return_type)
        }
        Some(_) => {
            let return_type = parse_field_type(current, &mut chars);
            (param_types, return_type)
        }
        _ => (vec![], dto::JType::Void),
    }
}
fn parse_field_descriptor(descriptor: &str) -> Option<dto::JType> {
    let mut chars = descriptor.chars();
    let current = chars.next();
    Some(parse_field_type(current, &mut chars))
}

fn parse_field_type(c: Option<char>, chars: &mut std::str::Chars) -> dto::JType {
    let Some(c) = c else {
        return dto::JType::Void;
    };
    match c {
        'B' => dto::JType::Byte,
        'C' => dto::JType::Char,
        'D' => dto::JType::Double,
        'F' => dto::JType::Float,
        'I' => dto::JType::Int,
        'J' => dto::JType::Long,
        'S' => dto::JType::Short,
        'Z' => dto::JType::Boolean,
        'V' => dto::JType::Void,
        'L' => {
            let mut class_name = String::new();
            for ch in chars.by_ref() {
                if ch == ';' {
                    break;
                }
                class_name.push(ch);
            }
            dto::JType::Class(class_name.replace('/', "."))
        }
        '[' => dto::JType::Array(Box::new(parse_field_type(chars.next(), chars))),
        _ => {
            //panic!("Unknown type: {}", c);
            dto::JType::Void
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        class::load_class,
        dto::{self, ImportUnit, SuperClass},
        loader::SourceDestination,
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn relative_source() {
        let result = load_class(
            include_bytes!("../test/Everything.class"),
            "ch.emilycares.Everything".to_string(),
            SourceDestination::RelativeInFolder("/source".to_string()),
        )
        .unwrap();
        assert!(result.source.starts_with("/source"));
        assert!(result.source.ends_with("Everything.java"));
    }

    #[test]
    fn everything() {
        let result = load_class(
            include_bytes!("../test/Everything.class"),
            "ch.emilycares.Everything".to_string(),
            SourceDestination::None,
        );

        assert_eq!(crate::tests::everything_data(), result.unwrap());
    }
    #[test]
    fn super_base() {
        let result = load_class(
            include_bytes!("../test/Super.class"),
            "ch.emilycares.Super".to_string(),
            SourceDestination::None,
        );

        assert_eq!(crate::tests::super_data(), result.unwrap());
    }
    #[test]
    fn thrower() {
        let result = load_class(
            include_bytes!("../test/Thrower.class"),
            "ch.emilycares.Thrower".to_string(),
            SourceDestination::Here("/path/to/source/Thrower.java".to_string()),
        );

        let mut check = crate::java::tests::thrower_data();
        check.imports = vec![ImportUnit::Package("ch.emilycares".to_string())];
        assert_eq!(check, result.unwrap());
    }
    #[test]
    fn super_interfaces() {
        let result = load_class(
            include_bytes!("../test/SuperInterface.class"),
            "ch.emilycares.SuperInterface".to_string(),
            SourceDestination::None,
        )
        .unwrap();
        assert_eq!(
            result,
            dto::Class {
                imports: vec![
                    ImportUnit::Package("ch.emilycares".to_string()),
                    ImportUnit::Class("java.util.stream.Stream".to_string())
                ],
                class_path: "ch.emilycares.SuperInterface".to_string(),
                name: "SuperInterface".to_string(),
                super_interfaces: vec![
                    SuperClass::Name("Collection".to_string()),
                    SuperClass::Name("List".to_string())
                ],
                methods: vec![dto::Method {
                    access: vec![dto::Access::Public],
                    name: "stream".to_string(),
                    parameters: vec![],
                    ret: dto::JType::Class("java.util.stream.Stream".to_string()),
                    throws: vec![],
                    source: None,
                },],
                ..Default::default()
            }
        )
    }
    #[test]
    fn variables() {
        let result = load_class(
            include_bytes!("../test/LocalVariableTable.class"),
            "ch.emilycares.LocalVariableTable".to_string(),
            SourceDestination::None,
        );

        assert_eq!(
            dto::Class {
                class_path: "ch.emilycares.LocalVariableTable".to_string(),
                name: "LocalVariableTable".to_string(),
                imports: vec![
                    ImportUnit::Package("ch.emilycares".to_string()),
                    ImportUnit::Class("java.util.HashMap".to_string()),
                    ImportUnit::Class("java.util.HashSet".to_string())
                ],
                methods: vec![
                    dto::Method {
                        access: vec![dto::Access::Public],
                        name: "hereIsCode".to_string(),
                        parameters: vec![],
                        ret: dto::JType::Void,
                        throws: vec![],
                        source: None,
                    },
                    dto::Method {
                        access: vec![dto::Access::Public],
                        name: "hereIsCode".to_string(),
                        parameters: vec![
                            dto::Parameter {
                                name: Some("a".to_string()),
                                jtype: dto::JType::Int
                            },
                            dto::Parameter {
                                name: Some("b".to_string()),
                                jtype: dto::JType::Int
                            }
                        ],
                        ret: dto::JType::Int,
                        throws: vec![],
                        source: None,
                    },
                ],
                fields: vec![dto::Field {
                    access: vec![dto::Access::Private],
                    name: "a".to_string(),
                    jtype: dto::JType::Class("java.util.HashSet".to_string()),
                    source: None,
                },],
                ..Default::default()
            },
            result.unwrap()
        );
    }
}
