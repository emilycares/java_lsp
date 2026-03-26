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
    util::{JResult, expect_data, get_i32, get_u8, get_u16},
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
    let (pos, header) = parse_header(data, pos)?;
    dbg!(&header);
    Err(JimageError::Todo)
}

fn parse_header(data: &[u8], pos: usize) -> JResult<JimageHeader> {
    debug_assert!(pos == 0);
    let pos = expect_data(data, pos, &[0xDA, 0xDA, 0xFE, 0xCA])?; // 4

    let (pos, major_version) = get_u16(data, pos)?; // 2
    let (pos, minor_version) = get_u16(data, pos)?; // 2
    if major_version == 0 && minor_version == 0 {
        return Err(JimageError::VersionNotSupported);
    }
    let (pos, flags) = get_i32(data, pos)?; // 1
    let (pos, resources_count) = get_i32(data, pos)?; // 4
    let (pos, table_len) = get_i32(data, pos)?; // 4
    let (pos, locations_size) = get_i32(data, pos)?; // 4
    let (pos, strings_size) = get_i32(data, pos)?; // 4

    debug_assert!(pos == 28);

    Ok((
        pos,
        JimageHeader {
            major_version,
            minor_version,
            flags,
            resources_count,
            table_len,
            locations_size,
            strings_size,
        },
    ))
}
