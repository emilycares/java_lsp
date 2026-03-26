use std::array::TryFromSliceError;

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
}

#[derive(Debug)]
pub struct JimageHeader {
    pub major_version: u16,
    pub minor_version: u16,
    pub flags: i32,
    pub resources_count: i32,
    pub table_len: i32,
    pub locations_size: i32,
    pub strings_size: i32,
}
impl JimageHeader {
    #[allow(unused)]
    #[must_use]
    pub const fn get_redirect_offset() -> i32 {
        7 * 4
    }

    #[allow(unused)]
    #[must_use]
    pub const fn get_redirect_size(&self) -> i32 {
        self.table_len * 4
    }
    #[allow(unused)]
    #[must_use]
    pub const fn get_offsets_offset(&self) -> i32 {
        Self::get_redirect_offset() + self.get_redirect_size()
    }

    #[allow(unused)]
    #[must_use]
    pub const fn get_locations_offset(&self) -> i32 {
        self.get_offsets_offset() + self.get_redirect_size()
    }

    #[allow(unused)]
    #[must_use]
    pub const fn get_strings_offset(&self) -> i32 {
        self.get_locations_offset() + self.locations_size
    }
}
