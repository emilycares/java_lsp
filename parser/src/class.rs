use crate::dto::{self, SourceKind};
use classfile_parser::constant_info::ConstantInfo;
use classfile_parser::method_info::MethodAccessFlags;
use classfile_parser::{class_parser, ClassAccessFlags, ClassFile};

pub fn load_class(bytes: &[u8], source: SourceKind) -> Result<dto::Class, dto::ClassError> {
    let res = class_parser(bytes);
    match res {
        Result::Ok((_, c)) => {
            let methods: Vec<_> = c
                .methods
                .iter()
                .filter_map(|method| {
                    let (params, ret) =
                        parse_method_descriptor(&lookup_string(&c, method.descriptor_index)?);
                    let mut params = params.into_iter();
                    Some(dto::Method {
                        access: match method.access_flags {
                            MethodAccessFlags::PUBLIC => vec![dto::Access::Public],
                            MethodAccessFlags::PRIVATE => vec![dto::Access::Private],
                            MethodAccessFlags::PROTECTED => vec![dto::Access::Protected],
                            MethodAccessFlags::STATIC => vec![dto::Access::Static],
                            MethodAccessFlags::FINAL => vec![dto::Access::Final],
                            MethodAccessFlags::ABSTRACT => vec![dto::Access::Abstract],
                            MethodAccessFlags::SYNTHETIC => vec![dto::Access::Synthetic],
                            _ => vec![],
                        },
                        name: lookup_string(&c, method.name_index)?,
                        parameters: method
                            .attributes
                            .iter()
                            .filter_map(|attribute_info| {
                                let attribute_parsed =
                                    classfile_parser::attribute_info::code_attribute_parser(
                                        &attribute_info.info,
                                    )
                                    .ok();
                                if let Some(attribute_parsed) = attribute_parsed {
                                    return Some(dto::Parameter {
                                        name: lookup_string(&c, attribute_parsed.1.max_stack)?,
                                        jtype: params.next()?,
                                    });
                                }
                                None
                            })
                            .collect(),
                        ret,
                    })
                })
                .collect();
            Ok(dto::Class {
                source,
                access: match c.access_flags {
                    ClassAccessFlags::PUBLIC => vec![dto::Access::Public],
                    ClassAccessFlags::FINAL => vec![dto::Access::Final],
                    ClassAccessFlags::ABSTRACT => vec![dto::Access::Abstract],
                    ClassAccessFlags::SYNTHETIC => vec![dto::Access::Synthetic],
                    _ => vec![],
                },
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
            })
        }
        _ => panic!("Not a class file"),
    }
}

fn lookup_string(c: &ClassFile, index: u16) -> Option<String> {
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
    use crate::{class::load_class, everything_data};
    use pretty_assertions::assert_eq;

    #[test]
    fn everything() {
        let result = load_class(
            include_bytes!("../test/Everything.class"),
            crate::dto::SourceKind::Jdk("".to_string()),
        );

        assert_eq!(everything_data(), result.unwrap());
    }
}
