use std::path::Path;

pub mod compile;
pub mod fetch;
#[allow(dead_code)]
mod pom;
pub mod project;
#[allow(dead_code)]
mod tree;

#[cfg(target_os = "linux")]
const EXECUTABLE_MAVEN: &str = "./mvnw";
#[cfg(target_os = "windows")]
const EXECUTABLE_MAVEN: &str = "./mvnw.cmd";

/// Takes a class path list
pub fn class_path_to_local(cpl: Vec<&str>) -> Vec<String> {
    cpl.iter()
        .map(|p| format!("./target/dependency/{}.class", p.replace('.', "/")))
        .filter(|p| Path::new(&p).exists())
        .collect()
}
