use std::process::Command;

use compile::{CompileErrorMessage, parse_compile_errors};

#[must_use]
pub fn compile_java(executable_gradle: &str) -> Option<Vec<CompileErrorMessage>> {
    run_compile_java(executable_gradle).map(|log| cut_and_parse(&log))
}

#[must_use]
fn cut_and_parse(log: &str) -> Vec<CompileErrorMessage> {
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

fn run_compile_java(executable_gradle: &str) -> Option<String> {
    // ./gradlew dependencies --console --plain
    let output = Command::new(executable_gradle)
        .arg("compileJava")
        .arg("-q")
        .output()
        .ok()?;

    Some(String::from_utf8_lossy(&output.stderr).to_string())
}

#[cfg(test)]
mod tests {
    use compile::CompileErrorMessage;
    use pretty_assertions::assert_eq;

    use crate::compile::cut_and_parse;

    #[test]
    fn gradle_compile() {
        let inp = include_str!("../tests/compile_basic.txt");
        let out = cut_and_parse(inp);
        assert_eq!(
            out,
            vec![
                CompileErrorMessage {
                    path: "/home/emily/tmp/vanilla-gradle/app/src/main/java/org/example/Other.java"
                        .to_string(),
                    message: "illegal start of type".to_string(),
                    row: 4,
                    col: 2,
                },
                CompileErrorMessage {
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
