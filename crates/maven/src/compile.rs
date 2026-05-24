use std::{
    fs::{self, read_to_string},
    path::Path,
    process::Command,
    str::{Utf8Error, from_utf8},
};

use common::Dependency;
use tokio::sync::OnceCell;

use crate::config::overwrite_settings_xml;
pub const CLASSPATH_FILE: &str = "./target/classpath.txt";

#[derive(Debug)]
pub enum MavenClasspathError {
    ReadExisting(std::io::Error),
    BuildClassPath(std::io::Error),
    GotError(String),
    Utf8(Utf8Error),
    Overwrite(std::io::Error),
}

pub static TREE: OnceCell<Vec<Dependency>> = OnceCell::const_new();

pub fn generate_classpath(maven_executable: &str) -> Result<String, MavenClasspathError> {
    let target = Path::new("./target");
    if should_load_existing_classpath() {
        let classpath =
            read_to_string(CLASSPATH_FILE).map_err(MavenClasspathError::ReadExisting)?;
        return Ok(classpath.trim().to_string());
    }
    if !target.exists() {
        let _ = fs::create_dir(target);
    }

    // mvn dependency:build-classpath -Dmdep.outputFile=target/classpath.txt
    let mut output = Command::new(maven_executable);
    let output = output.args([
        "dependency:build-classpath",
        "-Dmdep.outputFile=target/classpath.txt",
    ]);
    let output = overwrite_settings_xml(output);
    let output = output
        .output()
        .map_err(MavenClasspathError::BuildClassPath)?;

    if !output.status.success() {
        let err = from_utf8(&output.stderr).map_err(MavenClasspathError::Utf8)?;
        return Err(MavenClasspathError::GotError(err.to_owned()));
    }

    let classpath = read_to_string(CLASSPATH_FILE).map_err(MavenClasspathError::ReadExisting)?;
    let full_classpath = format!("{}:target/classes", classpath.trim());

    fs::write(CLASSPATH_FILE, &full_classpath).map_err(MavenClasspathError::Overwrite)?;

    Ok(full_classpath)
}

fn should_load_existing_classpath() -> bool {
    let classpath_mtime = fs::metadata(CLASSPATH_FILE).and_then(|m| m.modified()).ok();
    let pom_mtime = fs::metadata("./pom.xml").and_then(|m| m.modified()).ok();
    match (classpath_mtime, pom_mtime) {
        (Some(cp), Some(pom)) => cp >= pom,
        _ => false,
    }
}
