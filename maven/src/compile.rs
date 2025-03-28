use std::{
    fs::{self, read_to_string},
    io::{self, Write},
    path::Path,
    process::Command,
};

use crate::EXECUTABLE_MAVEN;

pub fn generate_classpath() -> Option<String> {
    let classpath_file = "target/classpath.txt";

    if Path::new(&classpath_file).exists() {
        let classpath = read_to_string(classpath_file).ok()?;
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

    let classpath = read_to_string(classpath_file).ok()?;
    let full_classpath = format!("{}:target/classes", classpath.trim());

    fs::write(classpath_file, &full_classpath).ok()?;

    Some(full_classpath)
}
