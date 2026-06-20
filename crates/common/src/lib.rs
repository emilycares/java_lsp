#![deny(clippy::redundant_clone)]

use std::{fs, path::PathBuf, sync::LazyLock};
pub mod deps;
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
    pub version_suffix: Option<String>,
}

pub static CACHE_DIR: LazyLock<String> = LazyLock::new(|| {
    cache_dir_no_create()
        .to_str()
        .unwrap_or_default()
        .to_owned()
});
pub fn cache_dir() -> PathBuf {
    let dir = dirs::cache_dir().expect("There should be a cache dir");
    let _ = fs::create_dir_all(&dir);
    dir
}
pub fn cache_dir_no_create() -> PathBuf {
    dirs::cache_dir().expect("There should be a cache dir")
}

pub fn project_cache_dir() -> PathBuf {
    let mut dir = dirs::cache_dir().expect("There should be a cache dir");
    dir = dir.join("project_cache");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub static DEPS_DIR: LazyLock<Option<String>> =
    LazyLock::new(|| deps_dir_no_create().to_str().map(ToOwned::to_owned));
pub fn deps_dir() -> PathBuf {
    let mut dir = dirs::cache_dir().expect("There should be a cache dir");
    dir = dir.join("deps");
    let _ = fs::create_dir_all(&dir);
    dir
}
pub fn deps_dir_no_create() -> PathBuf {
    let mut dir = dirs::cache_dir().expect("There should be a cache dir");
    dir = dir.join("deps");
    dir
}

pub fn java_dir() -> PathBuf {
    let mut dir = dirs::cache_dir().expect("There should be a cache dir");
    dir = dir.join("java");
    let _ = fs::create_dir_all(&dir);
    dir
}
pub fn java_dir_no_create() -> PathBuf {
    let mut dir = dirs::cache_dir().expect("There should be a cache dir");
    dir = dir.join("java");
    dir
}
