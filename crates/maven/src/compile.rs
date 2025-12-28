use std::{
    fs::{self, read_to_string},
    io::{self, Write},
    path::Path,
    process::Command,
};

use crate::config::overwrite_settings_xml;
const CLASSPATH_FILE: &str = "target/classpath.txt";

#[must_use]
pub fn generate_classpath(maven_executable: &str) -> Option<String> {
    if Path::new(&CLASSPATH_FILE).exists() {
        let classpath = read_to_string(CLASSPATH_FILE).ok()?;
        return Some(classpath.trim().to_string());
    }

    // mvn dependency:build-classpath -Dmdep.outputFile=target/classpath.txt
    let mut output = Command::new(maven_executable);
    let output = output.args([
        "dependency:build-classpath",
        "-Dmdep.outputFile=target/classpath.txt",
    ]);
    let output = overwrite_settings_xml(output);
    let output = output.output().ok()?;

    if !output.status.success() {
        io::stderr().write_all(&output.stderr).ok()?;
        return None;
    }

    let classpath = read_to_string(CLASSPATH_FILE).ok()?;
    let full_classpath = format!("{}:target/classes", classpath.trim());

    fs::write(CLASSPATH_FILE, &full_classpath).ok()?;

    Some(full_classpath)
}
