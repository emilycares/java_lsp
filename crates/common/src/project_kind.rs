use std::fs::canonicalize;
#[cfg(unix)]
use std::path::Path;
use std::{ffi::OsString, fmt::Display, path::PathBuf};

#[derive(Debug, PartialEq, Clone)]
pub enum ProjectKind {
    Maven {
        executable: String,
    },
    Gradle {
        executable: String,
        path_build_gradle: PathBuf,
    },
    Unknown,
}

#[derive(Debug)]
pub enum ProjectKindError {
    PathToString,
    MvnNotInPath,
    GradleNotInPath,
    ExecutableNotFound(std::io::Error),
    ExecutableNoMetadata(std::io::Error),
    NoPermissionToExecute(String),
    Canonicalize(std::io::Error),
}

impl Display for ProjectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectKind::Maven { .. } => write!(f, "maven"),
            ProjectKind::Gradle { .. } => write!(f, "gradle"),
            ProjectKind::Unknown => write!(f, "unknown"),
        }
    }
}

pub fn get_project_kind(
    project_dir: &PathBuf,
    path: &OsString,
) -> Result<ProjectKind, ProjectKindError> {
    eprintln!("Current dir {:?}", project_dir);
    if project_dir.join("pom.xml").exists() {
        return get_maven_executable(path);
    }

    let build_gradle = project_dir.join("build.gradle");
    if build_gradle.exists() {
        return get_gradle_executable(build_gradle, path);
    }

    let build_gradle = project_dir.join("build.gradle.kts");
    if build_gradle.exists() {
        return get_gradle_executable(build_gradle, path);
    }

    Ok(ProjectKind::Unknown)
}

#[cfg(target_os = "windows")]
fn get_maven_executable(path: &OsString) -> Result<ProjectKind, ProjectKindError> {
    let cmd = PathBuf::from("./mvnw.cmd");
    if cmd.exists() {
        let cmd = canonicalize(cmd).map_err(ProjectKindError::Canonicalize)?;
        let st = cmd.to_str().unwrap_or_default();
        return Ok(ProjectKind::Maven {
            executable: st.to_string(),
        });
    }
    let executable = get_executable_from_path("mvn.cmd", ProjectKindError::MvnNotInPath, path)?;
    Ok(ProjectKind::Maven { executable })
}

#[cfg(not(target_os = "windows"))]
fn get_maven_executable(path: &OsString) -> Result<ProjectKind, ProjectKindError> {
    let executable = PathBuf::from("./mvnw");
    if executable.exists() {
        let executable = canonicalize(executable).map_err(ProjectKindError::Canonicalize)?;
        #[cfg(unix)]
        check_executable_permission_path(&executable)?;
        let st = executable.to_str().unwrap_or_default();
        return Ok(ProjectKind::Maven {
            executable: st.to_owned(),
        });
    }
    let executable = get_executable_from_path("mvn", ProjectKindError::MvnNotInPath, path)?;
    Ok(ProjectKind::Maven { executable })
}
#[cfg(target_os = "windows")]
fn get_gradle_executable(
    path_build_gradle: PathBuf,
    path: &OsString,
) -> Result<ProjectKind, ProjectKindError> {
    let bat = PathBuf::from("./gradlew.bat");
    if bat.exists() {
        let bat = canonicalize(bat).map_err(ProjectKindError::Canonicalize)?;
        let st = bat.to_str().unwrap_or_default();
        return Ok(ProjectKind::Gradle {
            executable: st.to_string(),
            path_build_gradle,
        });
    }
    let executable =
        get_executable_from_path("gradle.exe", ProjectKindError::GradleNotInPath, path)?;
    Ok(ProjectKind::Gradle {
        executable,
        path_build_gradle,
    })
}

#[cfg(not(target_os = "windows"))]
fn get_gradle_executable(
    path_build_gradle: PathBuf,
    path: &OsString,
) -> Result<ProjectKind, ProjectKindError> {
    let executable = PathBuf::from("./gradlew");
    if executable.exists() {
        let executable = canonicalize(executable).map_err(ProjectKindError::Canonicalize)?;
        #[cfg(unix)]
        check_executable_permission_path(&executable)?;
        let st = executable.to_str().unwrap_or_default();
        return Ok(ProjectKind::Gradle {
            executable: st.to_string(),
            path_build_gradle,
        });
    }
    let executable = get_executable_from_path("gradle", ProjectKindError::GradleNotInPath, path)?;
    Ok(ProjectKind::Gradle {
        executable,
        path_build_gradle,
    })
}

fn get_executable_from_path(
    executable: &str,
    e: ProjectKindError,
    path: &OsString,
) -> Result<String, ProjectKindError> {
    let path = std::env::split_paths(path)
        .map(|i| i.join(executable))
        .find(|i| i.is_file())
        .ok_or(e)?;
    let executable = path.to_str().ok_or(ProjectKindError::PathToString)?;
    #[cfg(unix)]
    check_executable_permission(executable)?;
    Ok(executable.to_owned())
}

#[cfg(unix)]
fn check_executable_permission(executable: &str) -> Result<(), ProjectKindError> {
    use std::{fs::File, os::unix::fs::PermissionsExt};
    let f = File::open(executable).map_err(ProjectKindError::ExecutableNotFound)?;
    let meta = f
        .metadata()
        .map_err(ProjectKindError::ExecutableNoMetadata)?;
    let mode_exec = meta.permissions().mode() & 0o111 != 0;
    if !mode_exec {
        return Err(ProjectKindError::NoPermissionToExecute(
            executable.to_owned(),
        ));
    }
    Ok(())
}
#[cfg(unix)]
fn check_executable_permission_path(executable: &Path) -> Result<(), ProjectKindError> {
    use std::{fs::File, os::unix::fs::PermissionsExt};
    let f = File::open(executable).map_err(ProjectKindError::ExecutableNotFound)?;
    let meta = f
        .metadata()
        .map_err(ProjectKindError::ExecutableNoMetadata)?;
    let mode_exec = meta.permissions().mode() & 0o111 != 0;
    if !mode_exec {
        let st = executable.to_str().unwrap_or_default();
        return Err(ProjectKindError::NoPermissionToExecute(st.to_string()));
    }
    Ok(())
}
