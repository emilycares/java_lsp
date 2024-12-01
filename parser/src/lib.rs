mod class;
pub mod dto;
mod java;
pub mod loader;

use std::{fmt::Debug, path::Path};

pub fn load_class_fs<T>(path: T, class_path: String) -> Result<dto::Class, dto::ClassError>
where
    T: AsRef<Path> + Debug,
{
    let bytes = std::fs::read(path)?;
    class::load_class(&bytes, class_path)
}

pub fn load_java(data: &[u8], class_path: String) -> Result<dto::Class, dto::ClassError> {
    java::load_java(data, class_path)
}

pub fn load_java_fs<T>(path: T, class_path: String) -> Result<dto::Class, dto::ClassError>
where
    T: AsRef<Path> + Debug,
{
    let bytes = std::fs::read(path)?;
    java::load_java(&bytes, class_path)
}

#[cfg(test)]
mod tests {
    use crate::dto;

    #[cfg(test)]
    pub fn everything_data() -> dto::Class {
        dto::Class {
            class_path: "".to_string(),
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
                            name: Some("a".to_string()),
                            jtype: dto::JType::Int,
                        },
                        dto::Parameter {
                            name: Some("b".to_string()),
                            jtype: dto::JType::Int,
                        },
                    ],
                    ret: dto::JType::Int,
                },
            ],
            fields: vec![
                dto::Field {
                    access: vec![],
                    name: "noprop".to_string(),
                    jtype: dto::JType::Int,
                },
                dto::Field {
                    access: vec![dto::Access::Public],
                    name: "publicproperty".to_string(),
                    jtype: dto::JType::Int,
                },
                dto::Field {
                    access: vec![dto::Access::Private],
                    name: "privateproperty".to_string(),
                    jtype: dto::JType::Int,
                },
            ],
        }
    }
}
