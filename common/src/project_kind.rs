use std::{fmt::Display, path::PathBuf};

#[derive(Debug, PartialEq)]
pub enum ProjectKind {
    Maven,
    Gradle,
    Unknown
}

impl Display for ProjectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectKind::Maven => write!(f, "maven"),
            ProjectKind::Gradle => write!(f, "gradle"),
            ProjectKind::Unknown => write!(f, "unknown"),
        }
    }
}

pub fn get_project_kind() -> ProjectKind {
    if PathBuf::from("./pom.xml").exists() {
        return ProjectKind::Maven;
    }

    if PathBuf::from("./settings.gradle").exists() {
        return ProjectKind::Gradle;
    }

    return ProjectKind::Unknown;
}
