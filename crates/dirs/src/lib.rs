//! Platform-specific directory resolution.
#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::enum_glob_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::path::PathBuf;

#[must_use]
#[cfg(windows)]
/// Returns the current user's home directory.
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| {
            let drive = std::env::var_os("HOMEDRIVE")?;
            let path = std::env::var_os("HOMEPATH")?;
            let mut p = PathBuf::from(drive);
            p.push(path);
            Some(p)
        })
}

#[must_use]
#[cfg(not(windows))]
/// Returns the current user's home directory.
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[must_use]
#[cfg(windows)]
/// Returns the user-specific cache directory.
pub fn cache_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

#[must_use]
#[cfg(target_os = "macos")]
/// Returns the user-specific cache directory.
pub fn cache_dir() -> Option<PathBuf> {
    home_dir().map(|h| h.join("Library").join("Caches"))
}

#[must_use]
#[cfg(not(any(windows, target_os = "macos")))]
/// Returns the user-specific cache directory.
pub fn cache_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".cache")))
}

#[must_use]
#[cfg(not(windows))]
/// Returns temp directory
pub fn temp_dir() -> Option<PathBuf> {
    Some(PathBuf::from("/tmp"))
}

#[must_use]
#[cfg(windows)]
/// Returns temp directory
pub fn temp_dir() -> Option<PathBuf> {
    std::env::var_os("userprofile")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join("AppData").join("Local").join("Temp.cache")))
}
