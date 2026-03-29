use std::{array::TryFromSliceError, str::Utf8Error};

use crate::mutf8::Mutf8Error;

#[derive(Debug)]
pub enum JimageError {
    Todo,
    /// Tried to reach outside ouf bounds
    EOF,
    /// Region was not as expected
    NotAsExpected {
        pos: usize,
        len: usize,
    },
    /// Parser does not support this version
    VersionNotSupported,
    /// Prasing int
    Number(TryFromSliceError),
    StringLength,
    Mutf8(Mutf8Error),
    Utf8(Utf8Error),
    InvalidLocationAttribute,
    InvalidAttributeKind,
}

#[derive(Debug)]
pub struct JimageHeader {
    pub major_version: u16,
    pub minor_version: u16,
    pub flags: u32,
    pub resources_count: u32,
    pub table_len: u32,
    pub locations_size: u32,
    pub strings_size: u32,
}
impl JimageHeader {
    #[allow(unused)]
    #[must_use]
    pub const fn get_redirect_offset() -> usize {
        7 * 4
    }

    #[allow(unused)]
    #[must_use]
    pub const fn get_redirect_size(&self) -> usize {
        self.table_len as usize * 4
    }
    #[allow(unused)]
    #[must_use]
    pub const fn get_offsets_offset(&self) -> usize {
        Self::get_redirect_offset() + self.get_redirect_size()
    }

    #[allow(unused)]
    #[must_use]
    pub const fn get_locations_offset(&self) -> usize {
        self.get_offsets_offset() + self.get_redirect_size()
    }

    #[allow(unused)]
    #[must_use]
    pub const fn get_strings_offset(&self) -> usize {
        self.get_locations_offset() + self.locations_size as usize
    }
}
#[derive(Debug, Default)]
pub struct JimageLocation {
    pub module: u64,
    pub parent: u64,
    pub base: u64,
    pub extension: u64,
    pub offset: u64,
    pub compressed: u64,
    pub uncompressed: u64,
}
