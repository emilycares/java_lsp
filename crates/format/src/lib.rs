#![deny(clippy::redundant_clone)]
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use config::FormatterConfig;

#[derive(Debug)]
pub enum FormatError {
    IO(std::io::Error),
    Spawn(std::io::Error),
    Diagnostic(Vec<FormatLineError>),
    NoFormatterSpecified,
}

#[derive(Debug)]
pub struct FormatLineError {
    pub line: u32,
    pub col: u32,
    pub message: String,
}
pub fn get_formatter_name(formatter: &FormatterConfig) -> String {
    match formatter {
        FormatterConfig::None => String::from("No formatter"),
        FormatterConfig::Google => String::from("Google java format"),
        FormatterConfig::Idea => String::from("Idea format"),
    }
}

pub fn format(
    formatter: &FormatterConfig,
    content: &[u8],
    path: &Path,
    project_dir: &Path,
) -> Result<Option<Vec<u8>>, FormatError> {
    match formatter {
        FormatterConfig::None => Err(FormatError::NoFormatterSpecified),
        FormatterConfig::Google => google_java_format(content),
        FormatterConfig::Idea => idea_java_format(path, project_dir),
    }
}

#[cfg(windows)]
const IDEA_COMMAND: &str = "idea64.exe";
#[cfg(not(windows))]
const IDEA_COMMAND: &str = "idea-oss";

fn idea_java_format(path: &Path, project_dir: &Path) -> Result<Option<Vec<u8>>, FormatError> {
    let mut child = Command::new(IDEA_COMMAND);
    let mut args = vec![];
    if let Some(config) = idea_formatter_config(project_dir) {
        args.push(String::from("-s"));
        args.push(config);
    } else {
        args.push(String::from("-allowDefaults"));
    }
    child
        .arg("format")
        .args(args)
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(FormatError::IO)?;
    // let mut buf = String::new();
    // if let Some(mut e) = o.stderr {
    //     let _ = e.read_to_string(&mut buf);
    // }
    // if let Some(mut e) = o.stdout {
    //     let _ = e.read_to_string(&mut buf);
    // }
    // eprintln!(buf);
    Ok(None)
}

fn idea_formatter_config(project_dir: &Path) -> Option<String> {
    let mut p = PathBuf::from(project_dir)
        .join(".idea")
        .join("codeStyles")
        .join("Project");
    p.set_extension("xml");
    if !p.exists() {
        return None;
    }
    p.to_str().map(ToString::to_string)
}

fn google_java_format(content: &[u8]) -> Result<Option<Vec<u8>>, FormatError> {
    let mut child = Command::new("google-java-format")
        .arg("-")
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(FormatError::Spawn)?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content).map_err(FormatError::IO)?;
    }
    let out = child.wait_with_output().map_err(FormatError::IO)?;

    if out.stdout.is_empty() {
        let errors = String::from_utf8_lossy(&out.stderr);
        return Err(FormatError::Diagnostic(google_java_format_parse_errors(
            errors.as_ref(),
        )?));
    }

    let buf = out.stdout.to_vec();

    Ok(Some(buf))
}

fn google_java_format_parse_errors(errors: &str) -> Result<Vec<FormatLineError>, FormatError> {
    let mut out = Vec::new();
    for l in errors.lines() {
        if !l.starts_with("<stdin>") {
            continue;
        }
        let l = l.trim_start_matches("<stdin>:");

        let mut spl = l.splitn(3, ":");
        if let Some(line) = spl.next()
            && let Ok(line) = line.parse()
            && let Some(col) = spl.next()
            && let Ok(col) = col.parse()
            && let Some(message) = spl.next()
        {
            out.push(FormatLineError {
                line,
                col,
                message: message.to_string(),
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;

    #[test]
    fn google_format_errors() {
        let errors = "
<stdin>:57:57: error: ';' expected
    private boolean supportCompareVerifyIdenticalContent
                                                        ^
<stdin>:108:23: error: ';' expected
            e.getTag()
                      ^
            ";

        let out = google_java_format_parse_errors(errors);
        let expected = expect![[r#"
            Ok(
                [
                    FormatLineError {
                        line: 57,
                        col: 57,
                        message: " error: ';' expected",
                    },
                    FormatLineError {
                        line: 108,
                        col: 23,
                        message: " error: ';' expected",
                    },
                ],
            )
        "#]];
        expected.assert_debug_eq(&out);
    }
}
