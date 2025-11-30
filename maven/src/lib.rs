#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
pub mod compile;
pub mod config;
pub mod fetch;
pub mod project;
mod tree;

#[cfg(not(target_os = "windows"))]
const EXECUTABLE_MAVEN: &str = "./mvnw";
#[cfg(target_os = "windows")]
const EXECUTABLE_MAVEN: &str = "mvn.cmd";
