#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
pub mod class;
pub mod dto;
pub mod java;

use std::{fmt::Debug, path::Path};

use ast::types::AstFile;
use dto::ClassError;
use java::ParseJavaError;
use my_string::MyString;

#[derive(Debug, Clone)]
pub enum SourceDestination {
    Here(MyString),
    RelativeInFolder(MyString),
    None,
}

pub fn update_project_java_file<T: AsRef<Path>>(
    file: T,
    ast: &AstFile,
) -> Result<dto::Class, ParseJavaError> {
    java::load_java_tree(
        ast,
        SourceDestination::Here(file.as_ref().to_str().unwrap_or_default().into()),
    )
}

pub fn load_class_fs<T>(
    path: T,
    class_path: MyString,
    source: SourceDestination,
) -> Result<dto::Class, dto::ClassError>
where
    T: AsRef<Path> + Debug,
{
    let bytes = std::fs::read(path).map_err(ClassError::IO)?;
    class::load_class(&bytes, class_path, source)
}
