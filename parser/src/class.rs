use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR};

use crate::SourceDestination;
use crate::dto::{self, ClassError, ImportUnit, JType, Parameter};
use classfile_parser::attribute_info::{AttributeInfo, CodeAttribute};
use classfile_parser::code_attribute::LocalVariableTableAttribute;
use classfile_parser::constant_info::ConstantInfo;
use classfile_parser::field_info::{FieldAccessFlags, FieldInfo};
use classfile_parser::method_info::MethodAccessFlags;
use classfile_parser::{ClassAccessFlags, ClassFile, class_parser};
use itertools::Itertools;
use my_string::MyString;

pub fn load_class(
    bytes: &[u8],
    class_path: MyString,
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
            let package = class_path
                .trim_end_matches(name.as_str())
                .trim_end_matches(".");
            let mut imports = vec![ImportUnit::Package(package.into())];
            imports.extend(
                used_classes
                    .into_iter()
                    .filter(|i| *i != class_path)
                    .unique()
                    .map(ImportUnit::Class),
            );

            let source = match source {
                SourceDestination::RelativeInFolder(e) => format!(
                    "{}{}{}.java",
                    e,
                    MAIN_SEPARATOR,
                    &class_path.replace(".", MAIN_SEPARATOR_STR)
                ),
                SourceDestination::Here(e) => e,
                SourceDestination::None => "".into(),
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

fn lookup_class_name(c: &ClassFile, index: usize) -> Option<MyString> {
    match c.const_pool.get(index.saturating_sub(1)) {
        Some(ConstantInfo::Class(class)) => lookup_string(c, class.name_index)
            .expect("Class to have name")
            .split("/")
            .last()
            .map(|a| a.into()),
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
                        return Some(name.into());
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

fn parse_used_classes(c: &ClassFile, code_attribute: Option<CodeAttribute>) -> Vec<MyString> {
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

fn jtype_class_names(i: JType) -> Vec<MyString> {
    match i {
        JType::Class(class) => vec![class],
        JType::Array(jtype) => jtype_class_names(*jtype),
        JType::Generic(_class, jtypes) => jtypes.into_iter().flat_map(jtype_class_names).collect(),
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

fn parse_class_access(flags: ClassAccessFlags) -> dto::Access {
    let mut access = dto::Access::empty();
    if flags == ClassAccessFlags::PUBLIC {
        access.insert(dto::Access::Public);
    }
    if flags == ClassAccessFlags::FINAL {
        access.insert(dto::Access::Final);
    }
    if flags == ClassAccessFlags::SUPER {
        access.insert(dto::Access::Super);
    }
    if flags == ClassAccessFlags::INTERFACE {
        access.insert(dto::Access::Interface);
    }
    if flags == ClassAccessFlags::SYNTHETIC {
        access.insert(dto::Access::Synthetic);
    }
    if flags == ClassAccessFlags::ANNOTATION {
        access.insert(dto::Access::Annotation);
    }
    if flags == ClassAccessFlags::ENUM {
        access.insert(dto::Access::Enum);
    }
    access
}

fn parse_method_access(method: &classfile_parser::method_info::MethodInfo) -> dto::Access {
    let mut access = dto::Access::empty();
    if method.access_flags == MethodAccessFlags::PUBLIC {
        access.insert(dto::Access::Public);
    }
    if method.access_flags == MethodAccessFlags::PRIVATE {
        access.insert(dto::Access::Private);
    }
    if method.access_flags == MethodAccessFlags::PROTECTED {
        access.insert(dto::Access::Protected);
    }
    if method.access_flags == MethodAccessFlags::STATIC {
        access.insert(dto::Access::Static);
    }
    if method.access_flags == MethodAccessFlags::FINAL {
        access.insert(dto::Access::Final);
    }
    if method.access_flags == MethodAccessFlags::ABSTRACT {
        access.insert(dto::Access::Abstract);
    }
    if method.access_flags == MethodAccessFlags::SYNTHETIC {
        access.insert(dto::Access::Synthetic);
    }
    access
}

fn parse_field_access(method: &FieldInfo) -> dto::Access {
    let mut access = dto::Access::empty();
    if method.access_flags == FieldAccessFlags::PUBLIC {
        access.insert(dto::Access::Public);
    }
    if method.access_flags == FieldAccessFlags::PRIVATE {
        access.insert(dto::Access::Private);
    }
    if method.access_flags == FieldAccessFlags::PROTECTED {
        access.insert(dto::Access::Protected);
    }
    if method.access_flags == FieldAccessFlags::STATIC {
        access.insert(dto::Access::Static);
    }
    if method.access_flags == FieldAccessFlags::FINAL {
        access.insert(dto::Access::Final);
    }
    if method.access_flags == FieldAccessFlags::SYNTHETIC {
        access.insert(dto::Access::Synthetic);
    }
    access
}

fn lookup_string(c: &ClassFile, index: u16) -> Option<MyString> {
    if index == 0 {
        return None;
    }
    let con = &c.const_pool[(index - 1) as usize];
    match con {
        ConstantInfo::Utf8(utf8) => Some((&utf8.utf8_string).into()),
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
    use crate::{SourceDestination, class::load_class};

    #[cfg(not(windows))]
    #[test]
    fn relative_source() {
        let result = load_class(
            include_bytes!("../test/Everything.class"),
            "ch.emilycares.Everything".into(),
            SourceDestination::RelativeInFolder("/source".into()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn everything() {
        let result = load_class(
            include_bytes!("../test/Everything.class"),
            "ch.emilycares.Everything".into(),
            SourceDestination::None,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }
    #[test]
    fn super_base() {
        let result = load_class(
            include_bytes!("../test/Super.class"),
            "ch.emilycares.Super".into(),
            SourceDestination::None,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }
    #[test]
    fn thrower() {
        let result = load_class(
            include_bytes!("../test/Thrower.class"),
            "ch.emilycares.Thrower".into(),
            SourceDestination::Here("/path/to/source/Thrower.java".into()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }
    #[test]
    fn super_interfaces() {
        let result = load_class(
            include_bytes!("../test/SuperInterface.class"),
            "ch.emilycares.SuperInterface".into(),
            SourceDestination::None,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }
    #[test]
    fn variables() {
        let result = load_class(
            include_bytes!("../test/LocalVariableTable.class"),
            "ch.emilycares.LocalVariableTable".into(),
            SourceDestination::None,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }
}
