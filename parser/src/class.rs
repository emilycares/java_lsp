use crate::dto::{self, SourceKind};
use classfile_parser::constant_info::ConstantInfo;
use classfile_parser::method_info::MethodAccessFlags;
use classfile_parser::{class_parser, ClassAccessFlags};

pub fn load_class(bytes: &[u8], source: SourceKind) -> Result<dto::Class, dto::ClassError> {
    let res = class_parser(bytes);
    match res {
        Result::Ok((_, c)) => {
            //eprintln!("Valid class file, version {},{} const_pool({}), this=const[{}], super=const[{}], interfaces({}), fields({}), methods({}), attributes({}), access({:?})", c.major_version, c.minor_version, c.const_pool_size, c.this_class, c.super_class, c.interfaces_count, c.fields_count, c.methods_count, c.attributes_count, c.access_flags);

            let methods: Vec<_> = c
                .methods
                .iter()
                .filter_map(|m| {
                    let (params, ret) = parse_method_descriptor(&lookup_string(
                        &c.const_pool[(m.descriptor_index - 1) as usize],
                    )?);
                    let mut params = params.into_iter();
                    Some(dto::Method {
                        access: match m.access_flags {
                            MethodAccessFlags::PUBLIC => vec![dto::Access::Public],
                            MethodAccessFlags::PRIVATE => vec![dto::Access::Private],
                            MethodAccessFlags::PROTECTED => vec![dto::Access::Protected],
                            MethodAccessFlags::STATIC => vec![dto::Access::Static],
                            MethodAccessFlags::FINAL => vec![dto::Access::Final],
                            MethodAccessFlags::ABSTRACT => vec![dto::Access::Abstract],
                            MethodAccessFlags::SYNTHETIC => vec![dto::Access::Synthetic],
                            _ => vec![],
                        },
                        name: lookup_string(&c.const_pool[(m.name_index - 1) as usize])?,
                        parameters: m
                            .attributes
                            .iter()
                            .filter_map(|a| {
                                if let Some(ai) =
                                    classfile_parser::attribute_info::code_attribute_parser(&a.info) .ok()
                                {
                                    let v: Vec<_> = ai.1.attributes.iter().map(|aii| aii.attribute_name_index).map(|n| 
                                        lookup_string(&c.const_pool[(n - 1) as usize])
                                        ).collect();
                                    dbg!(&v);
                                    return Some(dto::Parameter {
                                        name: lookup_string(
                                            &c.const_pool[(a.attribute_name_index) as usize],
                                        )?,
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
                    ConstantInfo::Class(class) => {
                        lookup_string(&c.const_pool[(class.name_index - 1) as usize])
                            .expect("Class to have name")
                            .split("/")
                            .last()
                            .unwrap()
                            .to_string()
                    }
                    _ => "".to_string(),
                },
                methods,
            })
        }
        _ => panic!("Not a class file"),
    }
}
fn lookup_string(con: &ConstantInfo) -> Option<String> {
    match con {
        ConstantInfo::Utf8(utf8) => Some(utf8.utf8_string.clone()),
        _ => None,
    }
}
//pub fn load_class_asm(bytes: &[u8], source: SourceKind) -> Result<dto::Class, dto::ClassError> {
//    let class = JvmsClassReader::read_class_bytes(bytes).map_err(|e| {
//        dbg!(e);
//        dbg!(&source);
//        dto::ClassError::Asm()
//    })?;
//
//    //dbg!(&class.major_version);
//
//    let methods: Vec<_> = class
//        .methods
//        .iter()
//        .filter(|method| {
//            let Some(name) = lookup_string(&class, method.name_index) else {
//                return false;
//            };
//            match name.as_str() {
//                "<init>" => false,
//                _ => true,
//            }
//        })
//        .map(|method| {
//            let Some(atr_name) = lookup_string(&class, method.name_index) else {
//                return None;
//            };
//            let Some(descriptor) = lookup_string(&class, method.descriptor_index) else {
//                return None;
//            };
//            let (params, ret) = parse_method_descriptor(&descriptor);
//            let mut params = params.into_iter();
//
//            let parameters: Vec<dto::Parameter> = method
//                .attributes
//                .iter()
//                .map(|attribute| match &attribute.info {
//                    java_asm::jvms::attr::Attribute::MethodParameters { parameters, .. } => Some(
//                        parameters
//                            .into_iter()
//                            .map(|p| {
//                                let Some(name) = lookup_string(&class, p.name_index) else {
//                                    return None;
//                                };
//                                return Some(dto::Parameter {
//                                    name,
//                                    jtype: params.next().expect("Parameter was not in descriptor"),
//                                });
//                            })
//                            .filter(|par| par.is_some())
//                            .map(|par| par.unwrap())
//                            .collect::<Vec<_>>(),
//                    ),
//                    _ => None,
//                })
//                .filter(|ps| ps.is_some())
//                .map(|ps| ps.unwrap())
//                .flatten()
//                .collect();
//
//            Some(dto::Method {
//                access: parse_access_fields(method.access_flags),
//                name: atr_name,
//                parameters,
//                ret,
//            })
//        })
//        .filter(|m| m.is_some())
//        .map(|m| m.unwrap())
//        .collect();
//
//    let class_name = lookup_class(&class, class.this_class);
//    let Some(class_name) = lookup_string(&class, class_name) else {
//        return Err(dto::ClassError::UnknownClassName);
//    };
//    let class_name = class_name.split("/").last().unwrap();
//    Ok(dto::Class {
//        source,
//        access: parse_access_fields(class.access_flags),
//        name: class_name.to_string(),
//        methods,
//    })
//}
//
//pub fn parse_access_fields(value: u16) -> Vec<dto::Access> {
//    let mut out = vec![];
//
//    let first = byte_is(value, 0);
//    if first == 1 {
//        out.push(dto::Access::Public);
//    }
//    if first == 2 {
//        out.push(dto::Access::Private);
//    }
//    if first == 4 {
//        out.push(dto::Access::Protected);
//    }
//    if first == 8 {
//        out.push(dto::Access::Static);
//    }
//    let second = byte_is(value, 1);
//    if second == 1 {
//        out.push(dto::Access::Final);
//    }
//    if second == 2 {
//        out.push(dto::Access::Super);
//    }
//    if second == 4 {
//        out.push(dto::Access::Volatile);
//    }
//    if second == 8 {
//        out.push(dto::Access::Transient);
//    }
//    let third = byte_is(value, 2);
//    if third == 2 {
//        out.push(dto::Access::Interface);
//    }
//    if third == 4 {
//        out.push(dto::Access::Abstract);
//    }
//    let fourth = byte_is(value, 3);
//    if fourth == 1 {
//        out.push(dto::Access::Synthetic);
//    }
//    if fourth == 4 {
//        out.push(dto::Access::Enum);
//    }
//
//    out
//}
//
//fn byte_is<T: Into<u64>>(value: T, byte_pos: usize) -> u8 {
//    // Convert the value to a 64-bit integer for generic handling
//    let value: u64 = value.into();
//
//    // Calculate the byte value at the specified position
//    ((value >> (byte_pos * 8)) & 0xFF) as u8
//}
//
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
//
//fn lookup_string(class: &java_asm::jvms::element::ClassFile, idx: u16) -> Option<String> {
//    let name = class
//        .constant_pool
//        .get(idx as usize)
//        .expect("The coompiler shoult ensure that the name can be looked up");
//    let Const::Utf8 { bytes, length: _ } = &name.info else {
//        return None;
//    };
//    Some(String::from_utf8_lossy(&bytes).to_string())
//}
//
//fn lookup_class(class: &java_asm::jvms::element::ClassFile, idx: u16) -> u16 {
//    let name = class
//        .constant_pool
//        .get(idx as usize)
//        .expect("The coompiler shoult ensure that the name can be looked up");
//    let Const::Class { name_index } = &name.info else {
//        panic!("A method name was not Utf8");
//    };
//    *name_index
//}

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

        assert_eq!(result.unwrap(), everything_data());
    }
}
