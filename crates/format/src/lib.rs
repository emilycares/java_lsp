#![deny(clippy::redundant_clone)]
use std::{
    io::Write,
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
    }
}

pub fn format(formatter: &FormatterConfig, content: &[u8]) -> Result<Vec<u8>, FormatError> {
    match formatter {
        FormatterConfig::None => Err(FormatError::NoFormatterSpecified),
        FormatterConfig::Google => google_java_format(content),
    }
}

pub enum Formatter {
    None,
}

fn google_java_format(content: &[u8]) -> Result<Vec<u8>, FormatError> {
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
        dbg!(&errors);
        return Err(FormatError::Diagnostic(google_java_format_parse_errors(
            errors.as_ref(),
        )?));
    }

    let buf = out.stdout.to_vec();

    Ok(buf)
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
