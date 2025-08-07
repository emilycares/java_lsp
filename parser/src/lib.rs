mod class;
pub mod dto;
pub mod java;
pub mod loader;

use std::{fmt::Debug, path::Path};

use dto::ClassError;
use java::ParseJavaError;
use loader::SourceDestination;
use smol_str::SmolStr;

pub fn update_project_java_file<T: AsRef<Path>>(
    file: T,
    bytes: &[u8],
) -> Result<dto::Class, ParseJavaError> {
    load_java(
        bytes,
        SourceDestination::Here(file.as_ref().to_str().unwrap_or_default().into()),
    )
}

pub fn load_class_fs<T>(
    path: T,
    class_path: SmolStr,
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
