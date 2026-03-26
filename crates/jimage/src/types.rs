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
    /// Prasing int whent wrong
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
