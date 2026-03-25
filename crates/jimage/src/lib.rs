#![allow(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::enum_glob_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
//! Parser for jimage binary file

pub mod types;
mod util;

use std::path::{Path, PathBuf};

use dto::ClassFolder;

use crate::{
    types::{JimageError, JimageHeader},
    util::{expect_data, get_u8},
};

#[must_use]
pub fn get_modules_path(java_path: &Path) -> PathBuf {
    let mut java_path = java_path.to_path_buf();
    java_path.pop();
    let mut modules_file: PathBuf = java_path.join("lib").join("modules");
    if !modules_file.exists() {
        let lib_openjdk_lib_modules = java_path
            .join("lib")
            .join("openjdk")
            .join("lib")
            .join("modules");
        if lib_openjdk_lib_modules.exists() {
            modules_file = lib_openjdk_lib_modules;
        }
    }
    modules_file
}

/// Parser for jimage binary file
/// <https://cr.openjdk.org/~sgehwolf/leyden/jimage_file_format_investigation_leyden.pdf>
pub fn parser(data: &[u8], pos: usize, _source_dir: &str) -> Result<ClassFolder, JimageError> {
    let pos = parse_header(data, pos)?;
    dbg!(pos);
    Err(JimageError::Todo)
}

fn parse_header(data: &[u8], pos: usize) -> Result<(JimageHeader, usize), JimageError> {
    let pos = expect_data(data, pos, &[0xDA, 0xDA, 0xFE, 0xCA])?;

    let (pos, v1) = get_u8(data, pos)?;
    let (pos, v2) = get_u8(data, pos)?;
    if v1 != 0 && v2 != 0 {
        return Err(JimageError::VersionNotSupported);
    }
    let (pos, _flags) = get_u8(data, pos)?;
    let (pos, resources_count) = get_u8(data, pos)?;
    let (pos, table_len) = get_u8(data, pos)?;
    let (pos, locations_size) = get_u8(data, pos)?;
    let (pos, strings) = get_u8(data, pos)?;

    dbg!(resources_count, table_len, locations_size, strings);
    dbg!(pos);

    todo!()
}
