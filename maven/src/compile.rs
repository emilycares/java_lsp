use std::{
    fs::{self, read_to_string},
    io::{self, Write},
    path::Path,
    process::Command,
};

use crate::EXECUTABLE_MAVEN;
const CLASSPATH_FILE: &str = "target/classpath.txt";

pub fn generate_classpath() -> Option<String> {
    if Path::new(&CLASSPATH_FILE).exists() {
        let classpath = read_to_string(CLASSPATH_FILE).ok()?;
        return Some(format!("{}:target/classes", classpath.trim()));
    }

    // mvn dependency:build-classpath -Dmdep.outputFile=target/classpath.txt
    let output = Command::new(EXECUTABLE_MAVEN)
        .args([
            "dependency:build-classpath",
            "-Dmdep.outputFile=target/classpath.txt",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        io::stderr().write_all(&output.stderr).ok()?;
        return None;
    }

    let classpath = read_to_string(CLASSPATH_FILE).ok()?;
    let full_classpath = format!("{}:target/classes", classpath.trim());

    fs::write(CLASSPATH_FILE, &full_classpath).ok()?;

    Some(full_classpath)
}
