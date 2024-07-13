use crate::dto;
use java_asm::jvms::{element::Const, read::JvmsClassReader};

pub fn load_class(bytes: &[u8]) -> Result<dto::Class, dto::ClassError> {
    let class = JvmsClassReader::read_class_bytes(bytes).unwrap();

    dbg!(class.major_version);
    dbg!(class.minor_version);

    let methods: Vec<_> = class
        .methods
        .iter()
        .filter(|method| {
            let name = lookup_string(&class, method.name_index);
            match name.as_str() {
                "<init>" => false,
                _ => true,
            }
        })
        .map(|method| {
            let descriptor = lookup_string(&class, method.descriptor_index);
            let (params, ret) = parse_method_descriptor(&descriptor);
            let mut params = params.into_iter();

            let parameters: Vec<dto::Parameter> = method
                .attributes
                .iter()
                .map(|attribute| match &attribute.info {
                    java_asm::jvms::attr::Attribute::MethodParameters { parameters, .. } => Some(
                        parameters
                            .into_iter()
                            .map(|p| dto::Parameter {
                                name: lookup_string(&class, p.name_index),
                                jtype: params.next().expect("Parameter was not in descriptor"),
                            })
                            .collect::<Vec<_>>(),
                    ),
                    _ => None,
                })
                .filter(|ps| ps.is_some())
                .map(|ps| ps.unwrap())
                .flatten()
                .collect();

            let atr_name = lookup_string(&class, method.name_index);

            dto::Method {
                access: parse_access_fields(method.access_flags),
                name: atr_name,
                parameters,
                ret,
            }
        })
        .collect();

    let class_name = lookup_class(&class, class.this_class);
    let class_name = lookup_string(&class, class_name);
    let class_name = class_name.split("/").last().unwrap();
    Ok(dto::Class {
        access: parse_access_fields(class.access_flags),
        name: class_name.to_string(),
        methods,
    })
}

pub fn parse_access_fields(value: u16) -> Vec<dto::Access> {
    let mut out = vec![];

    let first = byte_is(value, 0);
    if first == 1 {
        out.push(dto::Access::Public);
    }
    if first == 2 {
        out.push(dto::Access::Private);
    }
    if first == 4 {
        out.push(dto::Access::Protected);
    }
    if first == 8 {
        out.push(dto::Access::Static);
    }
    let second = byte_is(value, 1);
    if second == 1 {
        out.push(dto::Access::Final);
    }
    if second == 2 {
        out.push(dto::Access::Super);
    }
    if second == 4 {
        out.push(dto::Access::Volatile);
    }
    if second == 8 {
        out.push(dto::Access::Transient);
    }
    let third = byte_is(value, 2);
    if third == 2 {
        out.push(dto::Access::Interface);
    }
    if third == 4 {
        out.push(dto::Access::Abstract);
    }
    let fourth = byte_is(value, 3);
    if fourth == 1 {
        out.push(dto::Access::Synthetic);
    }
    if fourth == 4 {
        out.push(dto::Access::Enum);
    }

    out
}

fn byte_is<T: Into<u64>>(value: T, byte_pos: usize) -> u8 {
    // Convert the value to a 64-bit integer for generic handling
    let value: u64 = value.into();

    // Calculate the byte value at the specified position
    ((value >> (byte_pos * 8)) & 0xFF) as u8
}

fn parse_method_descriptor(descriptor: &str) -> (Vec<dto::JType>, dto::JType) {
    let mut param_types = Vec::new();
    let mut chars = descriptor.chars();
    assert_eq!(chars.next(), Some('('));

    while let Some(c) = chars.next() {
        if c == ')' {
            break;
        }
        param_types.push(parse_field_type(c, &mut chars));
    }

    let return_type = parse_field_type(chars.next().unwrap(), &mut chars);
    (param_types, return_type)
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
        _ => panic!("Unknown type: {}", c),
    }
}

fn lookup_string(class: &java_asm::jvms::element::ClassFile, idx: u16) -> String {
    let name = class
        .constant_pool
        .get(idx as usize)
        .expect("The coompiler shoult ensure that the name can be looked up");
    let Const::Utf8 { bytes, length: _ } = &name.info else {
        panic!("A method name was not Utf8");
    };
    String::from_utf8_lossy(&bytes).to_string()
}

fn lookup_class(class: &java_asm::jvms::element::ClassFile, idx: u16) -> u16 {
    let name = class
        .constant_pool
        .get(idx as usize)
        .expect("The coompiler shoult ensure that the name can be looked up");
    let Const::Class { name_index } = &name.info else {
        panic!("A method name was not Utf8");
    };
    *name_index
}

#[cfg(test)]
mod tests {
    use crate::{class::load_class, everything_data};
    use pretty_assertions::assert_eq;

    #[test]
    fn everything() {
        let result = load_class(include_bytes!("../test/Everything.class"));

        assert_eq!(result.unwrap(), everything_data());
    }
}
