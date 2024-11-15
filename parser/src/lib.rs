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

pub fn load_java(data: &Vec<u8>, class_path: String) -> Result<dto::Class, dto::ClassError> {
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
    use std::path::Path;

    use crate::load_class_fs;

    #[test]
    fn fsbug() {
        let _ = load_class_fs(
            Path::new(
                "/home/emily/Documents/java/getting-started/jdk/classes/java/util/HashMap.class",
            ),
            "java.util.HashMap".to_string(),
        );
    }

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
                    methods: vec![],
                    ret: dto::JType::Void,
                },
                dto::Method {
                    access: vec![dto::Access::Public],
                    name: "public_method".to_string(),
                    methods: vec![],
                    ret: dto::JType::Void,
                },
                dto::Method {
                    access: vec![dto::Access::Private],
                    name: "private_method".to_string(),
                    methods: vec![],
                    ret: dto::JType::Void,
                },
                dto::Method {
                    access: vec![],
                    name: "out".to_string(),
                    methods: vec![],
                    ret: dto::JType::Int,
                },
                dto::Method {
                    access: vec![],
                    name: "add".to_string(),
                    methods: vec![
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
