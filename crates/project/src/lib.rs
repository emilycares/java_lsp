use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use common::{Dependency, TaskProgress, project_kind::ProjectKind};
use dto::Class;
use gradle::project::get_gradle_cache_path;
use lsp_extra::{SERVER_NAME, send_diagnostic, source_to_uri};
use lsp_server::Connection;
use lsp_types::{Diagnostic, DiagnosticSeverity, Range};
use maven::{
    pom::load_pom_xml,
    project::get_maven_cache_path,
    repository::Repository,
    update::{self, MavenUpdateError},
};
use my_string::NuVec;

#[derive(Debug, Clone)]
pub struct Project {
    pub artifact_id: String,
    pub dir: String,
    pub kind: ProjectKind,
}

#[must_use]
pub fn project_kind_to_project(dir: &str, kind: ProjectKind) -> Project {
    if let ProjectKind::Maven { .. } = kind
        && let Ok(pom) = load_pom_xml(PathBuf::from(dir).as_path())
    {
        return Project {
            artifact_id: pom.artifact_id,
            dir: dir.to_string(),
            kind,
        };
    }
    let p = Path::new(dir);
    let artifact_id: String = if let Some(name) = p.file_name()
        && let Some(name) = name.to_str()
    {
        name.to_string()
    } else {
        String::from("default")
    };
    Project {
        artifact_id,
        dir: dir.to_string(),
        kind,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn project_deps(
    sender: tokio::sync::watch::Sender<TaskProgress>,
    project_kind: ProjectKind,
    class_map: Arc<RwLock<HashMap<NuVec, Class>>>,
    use_cache: bool,
    project_dir: &Path,
    project_cache_dir: &Path,
    tree: &[Dependency],
    repos: Arc<Vec<Repository>>,
    project_artifacts: Arc<Vec<String>>,
    online: bool,
) {
    let cache_path = match project_kind {
        ProjectKind::Maven { .. } => Some(get_maven_cache_path(project_dir, project_cache_dir)),
        ProjectKind::Gradle { .. } => Some(get_gradle_cache_path(project_dir, project_cache_dir)),
        ProjectKind::Unknown => None,
    };
    if let Some(cache_path) = cache_path {
        match maven::project::project_deps(
            class_map,
            sender,
            use_cache,
            tree,
            cache_path,
            repos,
            project_artifacts,
            online,
        )
        .await
        {
            Ok(()) => (),
            Err(e) => {
                eprintln!("Got error while loading maven project: {e:?}");
            }
        }
    }
}

pub async fn update_report(
    project_kind: ProjectKind,
    con: Arc<Connection>,
    repos: Arc<Vec<Repository>>,
    tree: &[Dependency],
    sender: tokio::sync::watch::Sender<TaskProgress>,
) {
    let res = update::update(repos, tree, sender).await;
    let Err(res) = res else {
        return;
    };
    let mut diagnostics = Vec::new();
    let range = Range::default();
    match res {
        MavenUpdateError::Driver | MavenUpdateError::CurlMulti(_) | MavenUpdateError::Curl(_) => {
            diagnostics.push(Diagnostic::new(
                range,
                Some(DiagnosticSeverity::ERROR),
                None,
                Some(String::from(SERVER_NAME)),
                String::from("Error fetching dependencies"),
                None,
                None,
            ));
        }
        MavenUpdateError::WriteHash(error)
        | MavenUpdateError::WriteJar(error)
        | MavenUpdateError::CreateDir(error)
        | MavenUpdateError::WriteEtag(error) => {
            let message = format!("Io error while update: {error}");
            diagnostics.push(Diagnostic::new(
                range,
                Some(DiagnosticSeverity::ERROR),
                None,
                Some(String::from(SERVER_NAME)),
                message,
                None,
                None,
            ));
        }
        MavenUpdateError::MTwo(mtwo_error) => {
            let message = format!("m2 error while update: {mtwo_error:?}");
            diagnostics.push(Diagnostic::new(
                range,
                Some(DiagnosticSeverity::ERROR),
                None,
                Some(String::from(SERVER_NAME)),
                message,
                None,
                None,
            ));
        }
    }
    report_maven_gradle_diagnostic(&project_kind, &con, diagnostics);
}

pub fn report_maven_gradle_diagnostic(
    project_kind: &ProjectKind,
    con: &Arc<Connection>,
    diagnostics: Vec<Diagnostic>,
) {
    let source = match project_kind {
        ProjectKind::Maven { .. } => Some(PathBuf::from("./pom.xml")),
        ProjectKind::Gradle {
            path_build_gradle, ..
        } => Some(path_build_gradle.clone()),
        ProjectKind::Unknown => None,
    };
    if let Some(source) = source
        && let Ok(source) = fs::canonicalize(source)
        && let Some(source) = source.to_str()
        && let Ok(uri) = source_to_uri(&NuVec::new(source.as_bytes()))
    {
        send_diagnostic(con, uri, diagnostics);
    }
}

#[must_use]
pub fn get_project_artifacts(projs: &[Project]) -> Arc<Vec<String>> {
    Arc::new(
        projs
            .iter()
            .map(|i| i.artifact_id.clone())
            .collect::<Vec<_>>(),
    )
}
