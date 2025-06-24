mod class;
pub mod dto;
pub mod java;
pub mod loader;

use std::{fmt::Debug, path::Path};

use dto::ClassError;
use java::ParseJavaError;
use loader::SourceDestination;

pub fn update_project_java_file<T: AsRef<Path>>(
    file: T,
    bytes: &[u8],
) -> Result<dto::Class, ParseJavaError> {
    load_java(
        bytes,
        SourceDestination::Here(file.as_ref().to_str().unwrap_or_default().to_string()),
    )
}

pub fn load_class_fs<T>(
    path: T,
    class_path: String,
    source: SourceDestination,
) -> Result<dto::Class, dto::ClassError>
where
    T: AsRef<Path> + Debug,
{
    let bytes = std::fs::read(path).map_err(ClassError::IO)?;
    class::load_class(&bytes, class_path, source)
}

pub fn load_java(data: &[u8], source: SourceDestination) -> Result<dto::Class, ParseJavaError> {
    java::load_java(data, source)
}

#[cfg(test)]
mod tests {
    use crate::dto;

    #[cfg(test)]
    pub fn everything_data() -> dto::Class {
        use crate::dto::ImportUnit;

        dto::Class {
            class_path: "ch.emilycares.Everything".to_string(),
            name: "Everything".to_string(),
            imports: vec![ImportUnit::Package("ch.emilycares".to_string())],
            methods: vec![
                dto::Method {
                    access: vec![],
                    name: "method".to_string(),
                    parameters: vec![],
                    ret: dto::JType::Void,
                    throws: vec![],
                    source: None,
                },
                dto::Method {
                    access: vec![dto::Access::Public],
                    name: "public_method".to_string(),
                    parameters: vec![],
                    ret: dto::JType::Void,
                    throws: vec![],
                    source: None,
                },
                dto::Method {
                    access: vec![dto::Access::Private],
                    name: "private_method".to_string(),
                    parameters: vec![],
                    ret: dto::JType::Void,
                    throws: vec![],
                    source: None,
                },
                dto::Method {
                    access: vec![],
                    name: "out".to_string(),
                    parameters: vec![],
                    ret: dto::JType::Int,
                    throws: vec![],
                    source: None,
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
                    throws: vec![],
                    source: None,
                },
                dto::Method {
                    access: vec![dto::Access::Static],
                    name: "sadd".to_string(),
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
                    throws: vec![],
                    source: None,
                },
            ],
            fields: vec![
                dto::Field {
                    access: vec![],
                    name: "noprop".to_string(),
                    jtype: dto::JType::Int,
                    source: None,
                },
                dto::Field {
                    access: vec![dto::Access::Public],
                    name: "publicproperty".to_string(),
                    jtype: dto::JType::Int,
                    source: None,
                },
                dto::Field {
                    access: vec![dto::Access::Private],
                    name: "privateproperty".to_string(),
                    jtype: dto::JType::Int,
                    source: None,
                },
            ],
            ..Default::default()
        }
    }
    #[cfg(test)]
    pub fn super_data() -> dto::Class {
        use crate::dto::ImportUnit;

        dto::Class {
            class_path: "ch.emilycares.Super".to_string(),
            name: "Super".to_string(),
            imports: vec![ImportUnit::Package("ch.emilycares".to_string())],
            super_class: dto::SuperClass::Name("IOException".to_string()),
            ..Default::default()
        }
    }
}
