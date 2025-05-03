pub mod compile;
pub mod fetch;
mod pom;
pub mod project;
mod tree;

#[cfg(not(target_os = "windows"))]
const EXECUTABLE_MAVEN: &str = "./mvnw";
#[cfg(target_os = "windows")]
const EXECUTABLE_MAVEN: &str = "./mvnw.cmd";
