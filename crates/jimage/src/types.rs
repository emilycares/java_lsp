use std::{array::TryFromSliceError, str::Utf8Error};

use class::ModuleInfo;
use dto::ClassParserError;
use mutf8::Mutf8Error;
use my_string::smol_str::SmolStr;

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
    DataNotFound,
    Usize,
    Module(ClassParserError),
    Class {
        re: SmolStr,
        e: ClassParserError,
    },
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
    pub const fn get_header_size(&self) -> usize {
        7 * 4
    }

    #[allow(unused)]
    #[must_use]
    pub const fn get_redirect_size(&self) -> usize {
        (self.table_len as usize).saturating_mul(4)
    }
    #[allow(unused)]
    #[must_use]
    pub const fn get_offset_size(&self) -> usize {
        (self.table_len as usize).saturating_mul(4)
    }
    #[allow(unused)]
    #[must_use]
    pub const fn get_offsets_offset(&self) -> usize {
        Self::get_redirect_offset().saturating_add(self.get_redirect_size())
    }

    #[allow(unused)]
    #[must_use]
    pub const fn get_locations_offset(&self) -> usize {
        self.get_offsets_offset()
            .saturating_add(self.get_redirect_size())
    }

    #[allow(unused)]
    #[must_use]
    pub const fn get_strings_offset(&self) -> usize {
        self.get_locations_offset()
            .saturating_add(self.locations_size as usize)
    }
    #[allow(unused)]
    #[must_use]
    pub const fn get_index_size(&self) -> usize {
        self.get_header_size()
            .saturating_add(self.get_redirect_size())
            .saturating_add(self.get_offset_size())
            .saturating_add(self.locations_size as usize)
            .saturating_add(self.strings_size as usize)
    }
}
#[derive(Debug, Default)]
pub struct JimageLocation {
    pub module: usize,
    pub parent: usize,
    pub base: usize,
    pub extension: usize,
    pub offset: usize,
    pub compressed: usize,
    pub uncompressed: usize,
}

pub type ModuleList = Vec<(usize, ModuleInfo)>;
