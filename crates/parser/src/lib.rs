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
use my_string::MyString;

use crate::dto::Class;

#[derive(Debug, Clone)]
pub enum SourceDestination {
    Here(MyString),
    RelativeInFolder(MyString),
    None,
}

pub fn update_project_java_file<T: AsRef<Path>>(file: T, ast: &AstFile) -> Class {
    java::load_java_tree(
        ast,
        SourceDestination::Here(file.as_ref().to_str().unwrap_or_default().into()),
    )
}
