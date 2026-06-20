use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::FormatError;

#[cfg(windows)]
const IDEA_COMMAND: &str = "idea64.exe";
#[cfg(not(windows))]
const IDEA_COMMAND: &str = "idea-oss";

pub fn idea_java_format(path: &Path, project_dir: &Path) -> Result<Option<Vec<u8>>, FormatError> {
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
