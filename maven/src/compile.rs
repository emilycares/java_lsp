use std::{
    fs::{self, read_to_string},
    io::{self, Write},
    path::Path,
    process::Command,
};

use nom::{
    branch::alt,
    character::complete::{alpha0, digit0},
    multi::separated_list0,
    sequence::{pair, separated_pair},
    IResult,
};
use nom::{
    bytes::{complete::take_until, streaming::tag},
    combinator::opt,
    multi::many0_count,
};
use serde::{Deserialize, Serialize};

pub fn generate_classpath() -> Option<String> {
    let classpath_file = "target/classpath.txt";

    if Path::new(&classpath_file).exists() {
        let classpath = read_to_string(&classpath_file).ok()?;
        return Some(format!("{}:target/classes", classpath.trim()));
    }

    let output = Command::new("mvn")
        .args(&[
            "dependency:build-classpath",
            "-Dmdep.outputFile=target/classpath.txt",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        io::stderr().write_all(&output.stderr).ok()?;
        return None;
    }

    let classpath = read_to_string(&classpath_file).ok()?;
    let full_classpath = format!("{}:target/classes", classpath.trim());

    fs::write(&classpath_file, &full_classpath).ok()?;

    Some(full_classpath)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileError {
    pub path: String,
    pub message: String,
    pub row: usize,
    pub col: usize,
}

pub fn compile_java_file(file_path: &str, classpath: &str) -> Option<Vec<CompileError>> {
    // Compile the Java file using `javac` with the generated classpath
    let out = Command::new("javac")
        .arg("-cp")
        .arg(classpath)
        .arg("-d")
        .arg("target/classes")
        .arg(file_path)
        .output()
        .ok()?;

    let stdout = out.stderr;
    let stdout = std::str::from_utf8(&stdout).ok()?;
    parse_compile_errors(stdout).ok().map(|e| e.1)
}

fn parse_compile_errors(input: &str) -> IResult<&str, Vec<CompileError>> {
    let (input, errors) = separated_list0(tag("\n"), parse_error)(input)?;
    Ok((input, errors))
}

fn parse_error(input: &str) -> IResult<&str, CompileError> {
    let (input, (_, path)) = pair(opt(tag("\n")), separated_list0(tag("/"), alpha0))(input)?;
    let (input, _) = take_until(".java:")(input)?;
    let (input, _) = tag(".java:")(input)?;
    let (input, (row, (msg, col))) =
        separated_pair(digit0, tag(": error: "), parse_message_and_col)(input)?;
    Ok((
        input,
        CompileError {
            path: format!("{}.java", path.join("/")),
            message: msg.to_string(),
            row: row.parse().unwrap(),
            col,
        },
    ))
}

fn parse_message_and_col(input: &str) -> IResult<&str, (&str, usize)> {
    let (input, message) = take_until("\n")(input)?;
    let (input, _) = take_until("\n")(input)?;
    let (input, _) = tag("\n")(input)?;
    let (input, _) = take_until("\n")(input)?;
    let (input, _) = tag("\n")(input)?;
    let (input, (col, _)) = pair(many0_count(alt((tag(" "), tag("\t")))), tag("^"))(input)?;
    Ok((input, (message, col)))
}

#[cfg(test)]
mod tests {
    use crate::compile::parse_compile_errors;

    #[test]
    fn parse_compile_errors_basic() {
        let input = "
src/main/java/org/acme/GreetingResource.java:15: error: > or ',' expected
        var hash = new HashMap<String, String();
                                             ^

src/main/java/org/acme/GreetingResource.java:15: error: > or ',' expected
        var hash = new HashMap<String, String();
                                             ^
1 error
          ";
        let out = parse_compile_errors(input).unwrap();
        insta::assert_yaml_snapshot!(out.1);
    }

    #[test]
    fn parse_compile_errors_real() {
        let input = "/home/emily/Documents/java/getting-started/src/main/java/org/acme/GreetingResource.java:16: error: > or ',' expected\n\tvar hash = new HashMap<String, String();\n\t                                     ^\n1 error\n";
        let out = parse_compile_errors(input).unwrap();
        dbg!(&out.0);
        insta::assert_yaml_snapshot!(out.1);
    }
}
