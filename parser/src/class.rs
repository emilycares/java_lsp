use crate::dto::{self, ClassError, Parameter};
use classfile_parser::constant_info::ConstantInfo;
use classfile_parser::field_info::{FieldAccessFlags, FieldInfo};
use classfile_parser::method_info::MethodAccessFlags;
use classfile_parser::{class_parser, ClassAccessFlags, ClassFile};

pub fn load_class(bytes: &[u8], class_path: String) -> Result<dto::Class, dto::ClassError> {
    let res = class_parser(bytes);
    match res {
        Result::Ok((_, c)) => {
            let methods: Vec<_> = c
                .methods
                .iter()
                .filter_map(|method| parse_method(&c, method))
                .filter(|m| m.name != "<init>")
                //.filter(|f| !f.access.contains(&dto::Access::Private))
                .collect();
            let fields: Vec<_> = c
                .fields
                .iter()
                .filter_map(|field| parse_field(&c, field))
                //.filter(|f| !f.access.contains(&dto::Access::Private))
                .collect();
            Ok(dto::Class {
                class_path,
                access: parse_class_access(c.access_flags),
                name: match &c.const_pool[(c.this_class - 1) as usize] {
                    ConstantInfo::Class(class) => lookup_string(&c, class.name_index)
                        .expect("Class to have name")
                        .split("/")
                        .last()
                        .unwrap()
                        .to_string(),
                    _ => "".to_string(),
                },
                methods,
                fields,
            })
        }
        _ => Err(ClassError::ParseError),
    }
}

fn parse_field(c: &ClassFile, field: &FieldInfo) -> Option<dto::Field> {
    Some(dto::Field {
        access: parse_field_access(field),
        name: lookup_string(c, field.name_index)?,
        jtype: parse_field_descriptor(&lookup_string(c, field.descriptor_index)?)?,
    })
}

fn parse_method(
    c: &ClassFile,
    method: &classfile_parser::method_info::MethodInfo,
) -> Option<dto::Method> {
    let (params, ret) = parse_method_descriptor(&lookup_string(c, method.descriptor_index)?);
    let mut params = params.into_iter();
    let mut methods: Vec<dto::Parameter> = method
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
                        name: lookup_string(c, pa.name_index)
                            .map(|s| {
                                if s.is_empty() {
                                    return None;
                                }
                                Some(s)
                            })
                            .flatten(),
                        jtype: params.next()?,
                    })
                })
                .collect::<Vec<Parameter>>()
        })
        .collect();
    // Remaining method descriptor data as params
    {
        while let Some(jtype) = params.next() {
            methods.push(dto::Parameter { name: None, jtype });
        }
    }
    Some(dto::Method {
        access: parse_method_access(method),
        name: lookup_string(c, method.name_index)?,
        parameters: methods,
        ret,
    })
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
                param_types.push(parse_field_type(c, &mut chars));
            }

            let return_type = parse_field_type(chars.next().unwrap(), &mut chars);
            (param_types, return_type)
        }
        Some(_) => {
            let return_type = parse_field_type(current.unwrap(), &mut chars);
            (param_types, return_type)
        }
        _ => (vec![], dto::JType::Void),
    }
}
fn parse_field_descriptor(descriptor: &str) -> Option<dto::JType> {
    let mut chars = descriptor.chars();
    let current = chars.next()?;
    Some(parse_field_type(current, &mut chars))
}

fn parse_field_type(c: char, chars: &mut std::str::Chars) -> dto::JType {
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
            while let Some(ch) = chars.next() {
                if ch == ';' {
                    break;
                }
                class_name.push(ch);
            }
            dto::JType::Class(class_name.replace('/', "."))
        }
        '[' => dto::JType::Array(Box::new(parse_field_type(chars.next().unwrap(), chars))),
        _ => {
            //panic!("Unknown type: {}", c);
            dto::JType::Void
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::class::load_class;
    use pretty_assertions::assert_eq;

    #[test]
    fn everything() {
        let result = load_class(include_bytes!("../test/Everything.class"), "".to_string());

        assert_eq!(crate::tests::everything_data(), result.unwrap());
    }
}
