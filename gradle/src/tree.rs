use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Dependency<'a> {
    pub group_id: &'a str,
    pub artivact_id: &'a str,
    pub version: &'a str,
}

pub fn load<'a>() -> Option<Vec<Dependency<'a>>> {
    let log: String = get_cli_output()?;
    let out = parse_tree(log);
    Some(out)
}

fn get_cli_output() -> Option<String> {
    // ./gradlew dependencies --console --plain
    let output = Command::new("./gradlew")
        .arg("dependencies")
        .arg("--console")
        .arg("plain")
        // .arg("-b")
        .output()
        .ok()?;

    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_tree<'a>(inp: String) -> Vec<Dependency<'a>> {
    let mut out = vec![];

    let mut capture = false;

    for line in inp.lines() {
        if line.contains(" - ") && line.ends_with(".") {
            capture = true;
        }

        if line.starts_with("(c) - A dependency constraint") {
            break;
        }

        if capture {
            if line.contains(" - ")
                || line.is_empty()
                || !line.contains("-")
                || line.starts_with("No dependencies")
            {
            } else {
                let line = line
                    .replace("-", "")
                    .replace("\\", "")
                    .replace(" ", "")
                    .replace("+", "")
                    .replace("|", "")
                    .replace("(*)", "")
                    .replace("(n)", "")
                    .replace("(c)", "");
                let line: &'static str = Box::leak(line.into_boxed_str());
                let mut spl = line.splitn(3, ":");
                if let Some(group_id) = spl.next() {
                    if let Some(artivact_id) = spl.next() {
                        if let Some(version) = spl.next() {
                            out.push(Dependency {
                                group_id,
                                artivact_id,
                                version,
                            })
                        }
                    }
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use crate::tree::parse_tree;

    #[test]
    fn parse_diagram() {
        let inp = include_str!("../tests/dependencies_report.txt");
        let out = parse_tree(inp.to_owned());
        insta::assert_yaml_snapshot!(out);
    }
}
