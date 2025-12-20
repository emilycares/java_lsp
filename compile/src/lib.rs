#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
pub struct CompileError {
    pub path: String,
    pub message: String,
    pub row: usize,
    pub col: usize,
}

#[must_use]
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
    Some(parse_compile_errors(stdout))
}

#[must_use]
pub fn parse_compile_errors(input: &str) -> Vec<CompileError> {
    let mut out = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut index = 0;
    loop {
        let ch = chars.get(index);
        let Some(ch) = ch else {
            break;
        };

        if !ch.is_alphabetic() && ch != &'/' {
            index += 1;
            continue;
        }
        let mut path = String::new();
        while let Some(ch) = chars.get(index)
            && ch != &':'
        {
            path.push(*ch);
            index += 1;
        }
        if path.starts_with("error") {
            break;
        }
        dbg!(&path);
        index += 1;
        let mut row = String::new();
        while let Some(ch) = chars.get(index)
            && ch.is_numeric()
        {
            row.push(*ch);
            index += 1;
        }
        dbg!(&row);
        index += 1;
        while let Some(ch) = chars.get(index)
            && ch != &':'
        {
            index += 1;
        }
        index += 2;

        let mut message = String::new();
        while let Some(ch) = chars.get(index)
            && ch != &'\n'
        {
            if ch == &'\r' {
                index += 1;
                continue;
            }
            message.push(*ch);
            index += 1;
        }
        dbg!(&message);
        // skip newline
        index += 1;
        // Skip code
        while let Some(ch) = chars.get(index)
            && ch != &'\n'
        {
            if ch == &'\r' {
                index += 1;
                continue;
            }
            index += 1;
        }
        // skip newline
        index += 1;
        let mut col = 0;
        while let Some(ch) = chars.get(index)
            && ch != &'^'
        {
            col += 1;
            index += 1;
        }
        dbg!(col);
        out.push(CompileError {
            path,
            message,
            row: row.parse().unwrap_or_default(),
            col,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{CompileError, parse_compile_errors};

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
        assert_eq!(
            out,
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
        let out = parse_compile_errors(input);
        assert_eq!(out, vec![
            CompileError {
                path: "/home/emily/Documents/java/getting-started/src/main/java/org/acme/GreetingResource.java".to_string(),
                message: "> or ',' expected".to_string(),
                row: 16,
                col: 38,
            },
        ]);
    }
}
