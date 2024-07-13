pub mod dto;
mod class;
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
