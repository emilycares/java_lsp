use std::process::Command;

use nom::{
    branch::alt,
    character::complete::digit0,
    multi::separated_list0,
    sequence::{pair, separated_pair},
    IResult, Parser,
};
use nom::{
    bytes::{complete::take_until, streaming::tag},
    combinator::opt,
    multi::many0_count,
};

#[derive(Debug, PartialEq)]
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

pub fn parse_compile_errors(input: &str) -> IResult<&str, Vec<CompileError>> {
    let (input, errors) = separated_list0(tag("\n"), parse_error).parse(input)?;
    Ok((input, errors))
}

fn parse_error(input: &str) -> IResult<&str, CompileError> {
    let (input, _) = opt(tag("\n")).parse(input)?;
    let (input, path) = take_until(".java:")(input)?;
    let (input, _) = tag(".java:")(input)?;
    let (input, (row, (msg, col))) =
        separated_pair(digit0, tag(": error: "), parse_message_and_col).parse(input)?;
    Ok((
        input,
        CompileError {
            path: format!("{}.java", path),
            message: msg.to_string(),
            row: row.parse().unwrap_or_default(),
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
    let (input, (col, _)) = pair(many0_count(alt((tag(" "), tag("\t")))), tag("^")).parse(input)?;
    Ok((input, (message, col)))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::compile::{parse_compile_errors, CompileError};

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
        assert_eq!(
            out.1,
            vec![
                CompileError {
                    path: "src/main/java/org/acme/GreetingResource.java".to_string(),
                    message: "> or ',' expected".to_string(),
                    row: 15,
                    col: 45,
                },
                CompileError {
                    path: "src/main/java/org/acme/GreetingResource.java".to_string(),
                    message: "> or ',' expected".to_string(),
                    row: 15,
                    col: 45,
                },
            ]
        );
    }

    #[test]
    fn parse_compile_errors_real() {
        let input = "/home/emily/Documents/java/getting-started/src/main/java/org/acme/GreetingResource.java:16: error: > or ',' expected\n\tvar hash = new HashMap<String, String();\n\t                                     ^\n1 error\n";
        let out = parse_compile_errors(input).unwrap();
        assert_eq!(out.1, vec![
            CompileError {
                path: "/home/emily/Documents/java/getting-started/src/main/java/org/acme/GreetingResource.java".to_string(),
                message: "> or ',' expected".to_string(),
                row: 16,
                col: 38,
            },
        ]);
    }
}
