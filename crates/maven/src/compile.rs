use std::{
    fs::{self, read_to_string},
    path::Path,
    process::Command,
    str::{Utf8Error, from_utf8},
};

use crate::config::overwrite_settings_xml;
pub const CLASSPATH_FILE: &str = "target/classpath.txt";

#[derive(Debug)]
pub enum MavenClasspathError {
    ReadExisting(std::io::Error),
    BuildClassPath(std::io::Error),
    GotError(String),
    Utf8(Utf8Error),
    Overwrite(std::io::Error),
}

pub fn generate_classpath(maven_executable: &str) -> Result<String, MavenClasspathError> {
    if Path::new(&CLASSPATH_FILE).exists() {
        let classpath =
            read_to_string(CLASSPATH_FILE).map_err(MavenClasspathError::ReadExisting)?;
        return Ok(classpath.trim().to_string());
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
