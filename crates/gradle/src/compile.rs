use std::process::Command;

use compile::{CompileError, parse_compile_errors};

use crate::fetch::PATH_GRADLE;

#[must_use]
pub fn compile_java() -> Option<Vec<CompileError>> {
    run_compile_java().map(|log| cut_and_parse(&log))
}

#[must_use]
fn cut_and_parse(log: &str) -> Vec<CompileError> {
    let log = cut_log(log);
    parse_compile_errors(&log)
}

#[must_use]
pub fn cut_log(inp: &str) -> String {
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
    let output = Command::new(PATH_GRADLE)
        .arg("compileJava")
        .arg("-q")
        .output()
        .ok()?;

    Some(String::from_utf8_lossy(&output.stderr).to_string())
}

#[cfg(test)]
mod tests {
    use compile::CompileError;
    use pretty_assertions::assert_eq;

    use crate::compile::cut_and_parse;

    #[test]
    fn gradle_compile() {
        let inp = include_str!("../tests/compile_basic.txt");
        let out = cut_and_parse(inp);
        assert_eq!(
            out,
            vec![
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
            ]
        );
    }
}
