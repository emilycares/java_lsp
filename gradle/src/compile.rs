use std::process::Command;

use common::compile::{parse_compile_errors, CompileError};

pub fn compile_java() -> Option<Vec<CompileError>> {
    if let Some(log) = run_compile_java() {
        if let Some(value) = cut_and_parse(log) {
            return Some(value);
        }
    }

    None
}

fn cut_and_parse(log: String) -> Option<Vec<CompileError>> {
    let log = cut_log(log);
    parse_compile_errors(&log).ok().map(|e| e.1)
}

pub fn cut_log(inp: String) -> String {
    let mut out = String::new();

    for line in inp.lines() {
        if line.starts_with("FAILURE:") {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn run_compile_java() -> Option<String> {
    // ./gradlew dependencies --console --plain
    let output = Command::new("./gradlew")
        .arg("compileJava")
        .arg("-q")
        .output()
        .ok()?;

    Some(String::from_utf8_lossy(&output.stderr).to_string())
}

#[cfg(test)]
mod tests {
    use common::compile::CompileError;
    use pretty_assertions::assert_eq;

    use crate::compile::cut_and_parse;

    #[test]
    fn gradle_compile() {
        let inp = include_str!("../tests/compile_basic.txt");
        let out = cut_and_parse(inp.to_owned());
        assert_eq!(
            out,
            Some(vec![
                CompileError {
                    path: "/home/emily/tmp/vanilla-gradle/app/src/main/java/org/example/Other.java"
                        .to_string(),
                    message: "illegal start of type".to_string(),
                    row: 4,
                    col: 2,
                },
                CompileError {
                    path: "/home/emily/tmp/vanilla-gradle/app/src/main/java/org/example/App.java"
                        .to_string(),
                    message: "';' expected".to_string(),
                    row: 4,
                    col: 19,
                },
            ])
        );
    }
}
