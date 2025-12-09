#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
pub mod compile;
pub mod config;
pub mod fetch;
pub mod project;
mod tree;

#[cfg(not(target_os = "windows"))]
const EXECUTABLE_MAVEN: &str = "./mvnw";
#[cfg(target_os = "windows")]
const EXECUTABLE_MAVEN: &str = "mvn.cmd";
