#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]

pub mod java;

use ast::types::AstFile;
use dto::{Class, SourceDestination};
use my_string::NuVec;

#[must_use]
pub fn update_project_java_file(file: NuVec, ast: &AstFile) -> Class {
    java::load_java_tree(ast, SourceDestination::Here(file))
}
