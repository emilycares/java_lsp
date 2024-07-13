mod class;
pub mod dto;
mod java;

use std::path::Path;

pub fn load_class_fs<T: AsRef<Path>>(path: T) -> Result<dto::Class, dto::ClassError> {
    let bytes = std::fs::read(path)?;
    class::load_class(&bytes)
}

pub fn load_java_fs<T: AsRef<Path>>(path: T) -> Result<dto::Class, dto::ClassError> {
    let bytes = std::fs::read(path)?;
    java::load_java(&bytes)
}

#[cfg(test)]
pub fn everything_data() -> dto::Class {
    dto::Class {
        name: "Everything".to_string(),
        access: vec![],
        methods: vec![
            dto::Method {
                access: vec![],
                name: "method".to_string(),
                parameters: vec![],
                ret: dto::JType::Void,
            },
            dto::Method {
                access: vec![dto::Access::Public],
                name: "public_method".to_string(),
                parameters: vec![],
                ret: dto::JType::Void,
            },
            dto::Method {
                access: vec![dto::Access::Private],
                name: "private_method".to_string(),
                parameters: vec![],
                ret: dto::JType::Void,
            },
            dto::Method {
                access: vec![],
                name: "out".to_string(),
                parameters: vec![],
                ret: dto::JType::Int,
            },
            dto::Method {
                access: vec![],
                name: "add".to_string(),
                parameters: vec![
                    dto::Parameter {
                        name: "a".to_string(),
                        jtype: dto::JType::Int,
                    },
                    dto::Parameter {
                        name: "b".to_string(),
                        jtype: dto::JType::Int,
                    },
                ],
                ret: dto::JType::Int,
            },
        ],
    }
}
