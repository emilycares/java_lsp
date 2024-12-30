use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum ProjectKind {
    Maven,
    Gradle,
    Unknown
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
