#![allow(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::enum_glob_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
//! Parser for jimage binary file
//!
//! copied/modified from <https://openjdk.org/index.html>

pub mod mutf8;
pub mod types;
mod util;

use std::{
    path::{MAIN_SEPARATOR, Path, PathBuf},
    str::from_utf8,
};

use dto::{CFC_VERSION, ClassFolder, SourceDestination};
use my_string::{
    MyString,
    smol_str::{StrExt, ToSmolStr, format_smolstr},
};
use parser::class::{ModuleInfo, load_class, load_module};

use crate::{
    mutf8::{get_string_len, mutf8_to_utf8},
    types::{JimageError, JimageHeader, JimageLocation, ModuleList},
    util::{JResult, expect_data, get_u16, get_u32},
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
pub fn parser(
    data: &[u8],
    pos: usize,
    source_dir: &MyString,
    filter: bool,
) -> Result<ClassFolder, JimageError> {
    let (_pos, header) = parse_header(data, pos)?;
    let index_size = header.get_index_size();

    let strings_data = data
        .get(header.get_strings_offset()..data.len())
        .ok_or(JimageError::EOF)?;

    let locations = data
        .get(header.get_locations_offset()..header.get_strings_offset() - 1)
        .ok_or(JimageError::EOF)?;
    let mut pos = 0;

    let (rules, module_info_id) = load_module_info(data, index_size, strings_data, locations, pos)?;
    let mut classes = vec![];

    'locations: while let Ok((npos, location)) = parse_location(locations, pos) {
        pos = npos;
        if let Some(id) = module_info_id
            && id == location.base
        {
            continue;
        }
        let (_, class_path) = parse_string(strings_data, location.parent)?;
        let (_, class_name) = parse_string(strings_data, location.base)?;

        for r in &rules {
            if r.0 == location.module && !r.1.exports.iter().any(|e| e == &class_path) {
                continue 'locations;
            }
        }

        let (_, module_name) = parse_string(strings_data, location.module)?;
        let source = SourceDestination::RelativeInFolder(format_smolstr!(
            "{source_dir}{}{module_name}",
            MAIN_SEPARATOR
        ));

        let bytes = get_content_bytes(data, index_size, &location)?;
        let class_path = format_smolstr!("{}.{}", class_path.replace_smolstr("/", "."), class_name);
        let class = load_class(bytes, class_path, source, filter);
        if let Ok(class) = class {
            classes.push(class);
        }
    }

    Ok(ClassFolder {
        version: CFC_VERSION,
        classes,
    })
}

fn load_module_info(
    data: &[u8],
    index_size: usize,
    strings_data: &[u8],
    locations: &[u8],
    pos: usize,
) -> Result<(ModuleList, Option<usize>), JimageError> {
    let mut pos = pos;
    let mut rules: Vec<(usize, ModuleInfo)> = Vec::new();
    let mut module_info_id = None;
    while let Ok((npos, location)) = parse_location(locations, pos) {
        pos = npos;
        if let Some(id) = module_info_id
            && id == location.base
        {
            let bytes = get_content_bytes(data, index_size, &location)?;
            match load_module(bytes) {
                Ok(c) => {
                    rules.push((location.module, c));
                }
                Err(e) => {
                    eprintln!("Unable to load class: ({e:?}");
                }
            }
        } else {
            let (_, class_name) = parse_string(strings_data, location.base)?;
            if class_name == "module-info" {
                module_info_id = Some(location.base);
                let bytes = get_content_bytes(data, index_size, &location)?;
                match load_module(bytes) {
                    Ok(c) => {
                        rules.push((location.module, c));
                    }
                    Err(e) => {
                        eprintln!("Unable to load class: ({e:?}");
                    }
                }
            }
        }
    }
    Ok((rules, module_info_id))
}

fn get_content_bytes<'a>(
    data: &'a [u8],
    index_size: usize,
    location: &'a JimageLocation,
) -> Result<&'a [u8], JimageError> {
    if location.compressed > 0 {
        return Err(JimageError::Todo);
    }

    debug_assert!(location.compressed == 0);
    let start = location.offset + index_size;
    data.get(start..start + location.uncompressed)
        .ok_or(JimageError::DataNotFound)
}

fn parse_header(data: &[u8], pos: usize) -> JResult<JimageHeader> {
    debug_assert!(pos == 0);
    let pos = expect_data(data, pos, &[0xDA, 0xDA, 0xFE, 0xCA])?;

    let (pos, major_version) = get_u16(data, pos)?;
    let (pos, minor_version) = get_u16(data, pos)?;
    if !(major_version == 0 && minor_version == 1) {
        return Err(JimageError::VersionNotSupported);
    }
    let (pos, flags) = get_u32(data, pos)?;
    let (pos, resources_count) = get_u32(data, pos)?;
    let (pos, table_len) = get_u32(data, pos)?;
    let (pos, locations_size) = get_u32(data, pos)?;
    let (pos, strings_size) = get_u32(data, pos)?;

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

const LOCATION_ATTRIBUTE_MODULE: u8 = 1;
const LOCATION_ATTRIBUTE_PARENT: u8 = 2;
const LOCATION_ATTRIBUTE_BASE: u8 = 3;
const LOCATION_ATTRIBUTE_EXTENSION: u8 = 4;
const LOCATION_ATTRIBUTE_OFFSET: u8 = 5;
const LOCATION_ATTRIBUTE_COMPRESSED: u8 = 6;
const LOCATION_ATTRIBUTE_UNCOMPRESSED: u8 = 7;
const LOCATION_ATTRIBUTE_COUNT: u8 = 8;

pub fn parse_location(data: &[u8], pos: usize) -> JResult<JimageLocation> {
    let mut pos = pos;

    let mut out = JimageLocation::default();

    loop {
        let lo = data.get(pos).ok_or(JimageError::EOF)?;
        pos += 1;
        let lo = *lo;
        // ATTRIBUTE_END
        if lo <= 0x7 {
            break;
        }

        let kind = lo >> 3;
        if LOCATION_ATTRIBUTE_COUNT <= kind {
            return Err(JimageError::InvalidAttributeKind);
        }

        let len = (lo & 0x7) + 1;
        let (_, val) = parse_location_value(data, pos, len)?;
        pos += len as usize;
        match kind {
            // LOCATION_ATTRIBUTE_END => {
            //     out.end = val;
            // }
            LOCATION_ATTRIBUTE_MODULE => {
                out.module = val;
            }
            LOCATION_ATTRIBUTE_PARENT => {
                out.parent = val;
            }
            LOCATION_ATTRIBUTE_BASE => {
                out.base = val;
            }
            LOCATION_ATTRIBUTE_EXTENSION => {
                out.extension = val;
            }
            LOCATION_ATTRIBUTE_OFFSET => {
                out.offset = val;
            }
            LOCATION_ATTRIBUTE_COMPRESSED => {
                out.compressed = val;
            }
            LOCATION_ATTRIBUTE_UNCOMPRESSED => {
                out.uncompressed = val;
            }
            // LOCATION_ATTRIBUTE_COUNT => {
            //     out.count = val;
            // }
            _ => return Err(JimageError::InvalidLocationAttribute),
        }
    }
    Ok((pos, out))
}
pub fn parse_location_value(data: &[u8], pos: usize, len: u8) -> JResult<usize> {
    let mut pos = pos;
    let mut out: u64 = 0;

    for _ in 0..len {
        out <<= 8;
        let get = data.get(pos).ok_or(JimageError::EOF)?;
        pos += 1;
        // let (npos, get) = get_u32(data, pos)?;
        let get = *get;
        out |= u64::from(get);
    }

    let out = usize::try_from(out).map_err(|_| JimageError::Usize)?;

    Ok((pos, out))
}

pub fn parse_string(data: &[u8], pos: usize) -> JResult<MyString> {
    let start = pos;
    let (pos, end) = get_string_len(data, pos)?;
    let slice = data.get(start..end).ok_or(JimageError::EOF)?;
    let cow = mutf8_to_utf8(slice).map_err(JimageError::Mutf8)?;
    let out = from_utf8(&cow).map_err(JimageError::Utf8)?;
    Ok((pos, out.to_smolstr()))
}
