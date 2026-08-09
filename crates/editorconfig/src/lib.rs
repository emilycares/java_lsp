use std::{env::current_dir, fs::read_to_string, path::Path};

#[derive(Debug)]
pub struct EditorConfig {
    pub indent_size: Option<usize>,
    pub indent_style: Option<IndentStyle>,
    pub end_of_line: Option<EndOfLine>,
}
impl EditorConfig {
    pub fn to_filled(self) -> EditorConfigFilled {
        let mut out = EditorConfigFilled::default();
        if let Some(indent_size) = self.indent_size {
            out.indent_size = indent_size;
        }
        if let Some(indent_style) = self.indent_style {
            out.indent_style = indent_style;
        }
        if let Some(end_of_line) = self.end_of_line {
            out.end_of_line = end_of_line;
        }
        out
    }
}
impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            indent_size: Some(4),
            indent_style: Some(IndentStyle::Space),
            end_of_line: Some(EndOfLine::LF),
        }
    }
}

#[derive(Debug)]
pub struct EditorConfigFilled {
    pub indent_size: usize,
    pub indent_style: IndentStyle,
    pub end_of_line: EndOfLine,
}

impl Default for EditorConfigFilled {
    fn default() -> Self {
        Self {
            indent_size: 4,
            indent_style: IndentStyle::Space,
            end_of_line: EndOfLine::LF,
        }
    }
}

#[derive(Debug)]
pub enum IndentStyle {
    Space,
    Tab,
}

#[derive(Debug)]
pub enum EndOfLine {
    LF,
    CRLF,
    CR,
}

#[derive(Debug)]
pub enum EditorConfigError {
    IO(std::io::Error),
}

pub fn load_editor_config_or_default() -> EditorConfig {
    current_dir().map_or_else(
        |_| EditorConfig::default(),
        |c| load_project_editor_config(&c).unwrap_or_default(),
    )
}

pub fn load_project_editor_config(project_dir: &Path) -> Result<EditorConfig, EditorConfigError> {
    let mut file = project_dir.join("");
    file.set_extension("editorconfig");

    let content = read_to_string(file).map_err(EditorConfigError::IO)?;
    Ok(parser(&content))
}

pub fn parser(content: &str) -> EditorConfig {
    let lines = content.lines();
    let mut capture = false;
    let mut out = EditorConfig {
        indent_size: None,
        indent_style: None,
        end_of_line: None,
    };
    for l in lines {
        let l = l.trim().to_ascii_lowercase();
        if l.starts_with("#") {
            continue;
        }
        if l.is_empty() {
            capture = false;
            continue;
        }
        if l.starts_with("[") && (l == "[*]" || l.contains("java")) {
            capture = true;
            continue;
        }

        if capture {
            dbg!(&l);
            if l.starts_with("indent_style") {
                if l.ends_with("space") {
                    out.indent_style = Some(IndentStyle::Space);
                } else if l.ends_with("tab") {
                    out.indent_style = Some(IndentStyle::Tab);
                }
            } else if l.starts_with("indent_size")
                && let Some((_, e)) = l.split_once("=")
            {
                let e = e.trim();
                if let Ok(size) = e.parse::<usize>() {
                    out.indent_size = Some(size);
                }
            } else if l.starts_with("end_of_line") {
                let l = l.to_lowercase();
                if l.ends_with("lf") {
                    out.end_of_line = Some(EndOfLine::LF);
                } else if l.ends_with("cr") {
                    out.end_of_line = Some(EndOfLine::CR);
                } else if l.ends_with("crlf") {
                    out.end_of_line = Some(EndOfLine::CRLF);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use crate::parser;

    #[test]
    fn basic() {
        let content = "
            # editor config: https://editorconfig.org
            root = true

            [*]
            charset = utf-8
            indent_style = space
            indent_size = 4
            end_of_line = lf
            insert_final_newline = true
            trim_trailing_whitespace = true
            ";
        let out = parser(content);
        let expected = expect![[r#"
            EditorConfig {
                indent_size: Some(
                    4,
                ),
                indent_style: Some(
                    Space,
                ),
                end_of_line: Some(
                    LF,
                ),
            }
        "#]];
        expected.assert_debug_eq(&out);
    }
}
