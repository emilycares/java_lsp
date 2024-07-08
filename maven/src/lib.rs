use std::path::Path;

use thiserror::Error;
#[allow(dead_code)]
mod pom;
#[allow(dead_code)]
mod tree;

#[derive(Error, Debug)]
pub enum MavenError {
    #[error("There was a error parsing a pom")]
    ParseError(#[from] serde_xml_rs::Error),
    #[error("IO error")]
    IO(#[from] std::io::Error),
    #[error("unknown")]
    Unknown,
    #[error("Unknown Dependencie scope")]
    UnknownDependencyScope,
    #[error("Run into a error running Dependency diagram")]
    TreeParseError(#[from] nom::Err<nom::error::Error<&'static str>>),
}
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

/// Takes a class path list
pub fn class_path_to_local(cpl: Vec<&str>) -> Vec<String> {
    cpl.iter()
        .map(|p| format!("./target/dependency/{}.class", p.replace('.', "/")))
        .filter(|p| Path::new(&p).exists())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}