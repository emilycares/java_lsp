#![deny(warnings)]
#![deny(clippy::redundant_clone)]

use std::{fs, path::PathBuf};
pub mod project_kind;

#[derive(Debug, Default, Clone)]
pub struct TaskProgress {
    pub percentage: u32,
    pub error: bool,
    pub message: String,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Dependency {
    pub group_id: String,
    pub artivact_id: String,
    pub version: String,
}

pub fn project_cache_dir() -> PathBuf {
    let mut dir = dirs::cache_dir().expect("There should be a cache dir");
    dir = dir.join("java_lsp").join("project_cache");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn deps_dir() -> PathBuf {
    let mut dir = dirs::cache_dir().expect("There should be a cache dir");
    dir = dir.join("java_lsp").join("deps");
    let _ = fs::create_dir_all(&dir);
    dir
}
