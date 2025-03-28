pub mod call_chain;
mod class;
pub mod dto;
pub mod java;
pub mod loader;

use std::{fmt::Debug, path::Path};

use java::ParseJavaError;
use loader::SourceDestination;

pub fn update_project_java_file<T: AsRef<Path>>(file: T, bytes: &[u8]) -> Option<dto::Class> {
    load_java(
        bytes,
        SourceDestination::Here(file.as_ref().to_str().unwrap_or_default().to_string()),
    )
    .ok()
}

pub fn src_folder_paths<T: AsRef<Path>>(folder: T) -> Vec<String> {
    let folder = folder.as_ref();
    if !folder.exists() {
        return vec![];
    }
    let Ok(current_dir) = std::env::current_dir() else {
        return vec![];
    };
    let path = current_dir.join(folder);
    loader::get_java_files_from_folder(path)
}

pub fn load_src_folder<T: AsRef<Path>>(folder: T) -> Option<dto::ClassFolder> {
    Some(loader::load_java_files(src_folder_paths(folder)))
}

pub fn load_class_fs<T>(
    path: T,
    class_path: String,
    source: SourceDestination,
) -> Result<dto::Class, dto::ClassError>
where
    T: AsRef<Path> + Debug,
{
    let bytes = std::fs::read(path)?;
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
        dto::Class {
            source: "".to_string(),
            class_path: "ch.emilycares.Everything".to_string(),
            name: "Everything".to_string(),
            access: vec![],
            methods: vec![
                dto::Method {
                    access: vec![],
                    name: "method".to_string(),
                    parameters: vec![],
                    ret: dto::JType::Void,
                    throws: vec![],
                },
                dto::Method {
                    access: vec![dto::Access::Public],
                    name: "public_method".to_string(),
                    parameters: vec![],
                    ret: dto::JType::Void,
                    throws: vec![],
                },
                dto::Method {
                    access: vec![dto::Access::Private],
                    name: "private_method".to_string(),
                    parameters: vec![],
                    ret: dto::JType::Void,
                    throws: vec![],
                },
                dto::Method {
                    access: vec![],
                    name: "out".to_string(),
                    parameters: vec![],
                    ret: dto::JType::Int,
                    throws: vec![],
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
