use std::{fmt::Display, path::PathBuf};

#[derive(Debug, PartialEq, Clone)]
pub enum ProjectKind {
    Maven,
    Gradle { path_build_gradle: PathBuf },
    Unknown,
}

impl Display for ProjectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectKind::Maven => write!(f, "maven"),
            ProjectKind::Gradle {
                path_build_gradle: _,
            } => write!(f, "gradle"),
            ProjectKind::Unknown => write!(f, "unknown"),
        }
    }
}

pub fn get_project_kind() -> ProjectKind {
    eprintln!("Current dir {:?}", std::env::current_dir().ok());
    if PathBuf::from("./pom.xml").exists() {
        return ProjectKind::Maven;
    }

    let build_gradle = "./build.gradle";
    let build_gradle = PathBuf::from(build_gradle);
    if build_gradle.exists() {
        return ProjectKind::Gradle {
            path_build_gradle: build_gradle,
        };
    }

    let build_gradle = "./build.gradle.kts";
    let build_gradle = PathBuf::from(build_gradle);
    if build_gradle.exists() {
        return ProjectKind::Gradle {
            path_build_gradle: build_gradle,
        };
    }

    ProjectKind::Unknown
}
