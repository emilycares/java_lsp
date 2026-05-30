#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::{process::Command, str::Utf8Error};

#[derive(Debug)]
pub enum CompileError {
    JavacIo(std::io::Error),
    Utf8(Utf8Error),
}

#[derive(Debug, PartialEq, Eq)]
pub struct CompileErrorMessage {
    pub path: String,
    pub message: String,
    pub row: usize,
    pub col: usize,
}

pub fn maven_compile_java_file(
    file_path: &str,
    classpath: &str,
) -> Result<Vec<CompileErrorMessage>, CompileError> {
    // Compile the Java file using `javac` with the generated classpath
    let out = Command::new("javac")
        .arg("-cp")
        .arg(classpath)
        .arg("-d")
        .arg("target/classes")
        .arg(file_path)
        .output()
        .map_err(CompileError::JavacIo)?;

    let stdout = std::str::from_utf8(&out.stderr).map_err(CompileError::Utf8)?;
    Ok(parse_compile_errors(stdout))
}

pub fn compile_java_file(file_path: &str) -> Result<Vec<CompileErrorMessage>, CompileError> {
    // Compile the Java file using `javac` with the generated classpath
    let out = Command::new("javac")
        .arg(file_path)
        .output()
        .map_err(CompileError::JavacIo)?;

    let stdout = std::str::from_utf8(&out.stderr).map_err(CompileError::Utf8)?;
    Ok(parse_compile_errors(stdout))
}

#[must_use]
pub fn parse_compile_errors(input: &str) -> Vec<CompileErrorMessage> {
    let mut out = Vec::new();

    let mut index = 0;
    let lines: Vec<_> = input.lines().collect();
    while let Some(line) = lines.get(index) {
        if line.contains(": error: ") {
            let mut spl = line.splitn(4, ':');
            if let Some(path) = spl.next()
                && let Some(row) = spl.next()
                && let Ok(row) = row.trim().parse()
                && let Some(_) = spl.next()
                && let Some(message) = spl.next()
            {
                let mut col = 0;
                if let Some(line) = lines.get(index + 2) {
                    let mut chars = line.chars();
                    while let Some(c) = chars.next()
                        && c != '^'
                    {
                        col += 1;
                    }
                    index += 3;
                    out.push(CompileErrorMessage {
                        path: path.trim().to_string(),
                        row,
                        col,
                        message: message.trim().to_string(),
                    });
                    continue;
                }
                out.push(CompileErrorMessage {
                    path: path.to_string(),
                    row,
                    col: 0,
                    message: message.trim().to_string(),
                });
            }
        }

        index += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use crate::parse_compile_errors;
    use expect_test::expect;

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
        let out = parse_compile_errors(input);
        let expected = expect![[r#"
            [
                CompileErrorMessage {
                    path: "src/main/java/org/acme/GreetingResource.java",
                    message: "> or ',' expected",
                    row: 15,
                    col: 45,
                },
                CompileErrorMessage {
                    path: "src/main/java/org/acme/GreetingResource.java",
                    message: "> or ',' expected",
                    row: 15,
                    col: 45,
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }

    #[test]
    fn parse_compile_errors_real() {
        let input = r"/home/emily/Documents/java/getting-started/src/main/java/org/acme/GreetingResource.java:16: error: > or ',' expected
	var hash = new HashMap<String, String();
	                                     ^
1 error
";
        let out = parse_compile_errors(input);
        let expected = expect![[r#"
            [
                CompileErrorMessage {
                    path: "/home/emily/Documents/java/getting-started/src/main/java/org/acme/GreetingResource.java",
                    message: "> or ',' expected",
                    row: 16,
                    col: 38,
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }
    #[test]
    fn parse_compile_errors_could_not_find_symbol() {
        let input = r#"/home/emily/Documents/java/getting-started/src/main/java/org/acme/GreetingResource.java:27: error: cannot find symbol
    public Uni<Response> createCampaign(SomeRequest request) {
                                        ^
  symbol:   class SomeRequest
  location: class GreetingResource
/home/emily/Documents/java/getting-started/src/main/java/org/acme/GreetingResource.java:42: error: cannot find symbol
    public Uni<Response> addQuest(@PathParam("slug") String slug, SomeRequest request) {
                                                                  ^
  symbol:   class SomeRequest
  location: class GreetingResource
2 error
"#;
        let out = parse_compile_errors(input);
        let expected = expect![[r#"
            [
                CompileErrorMessage {
                    path: "/home/emily/Documents/java/getting-started/src/main/java/org/acme/GreetingResource.java",
                    message: "cannot find symbol",
                    row: 27,
                    col: 40,
                },
                CompileErrorMessage {
                    path: "/home/emily/Documents/java/getting-started/src/main/java/org/acme/GreetingResource.java",
                    message: "cannot find symbol",
                    row: 42,
                    col: 66,
                },
            ]
        "#]];
        expected.assert_debug_eq(&out);
    }
    #[test]
    fn parse_compile_errors_end_note() {
        let input = r"
Note: Some messages have been simplified; recompile with -Xdiags:verbose to get full output
100 errors
only showing the first 100 errors, of 115 total; use -Xmaxerrs if you would like to see more
";
        let out = parse_compile_errors(input);
        let expected = expect![[r"
            []
        "]];
        expected.assert_debug_eq(&out);
    }
}
