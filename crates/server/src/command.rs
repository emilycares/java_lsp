use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use common::{Dependency, TaskProgress, project_cache_dir, project_kind::ProjectKind};
use dashmap::DashMap;
use lsp_extra::SERVER_NAME;
use lsp_server::Connection;
use lsp_types::{Diagnostic, DiagnosticSeverity, ProgressToken, Range};
use maven::tree::MavenTreeError;
use my_string::MyString;
use parser::dto::Class;

use crate::backend::{
    Backend, project_deps, read_forward, report_maven_gradle_diagnostic, update_report,
};

pub const COMMAND_RELOAD_DEPENDENCIES: &str = "ReloadDependencies";
#[must_use]
pub fn reload_dependencies(
    con: &Arc<Connection>,
    progress: Option<ProgressToken>,
    project_kind: &ProjectKind,
    class_map: &Arc<DashMap<MyString, Class>>,
    project_dir: &Path,
) -> Option<serde_json::Value> {
    let con = con.clone();
    let project_kind = project_kind.clone();
    let class_map = class_map.clone();
    let project_dir = project_dir.to_owned();
    tokio::spawn(async move {
        let progress = Arc::new(progress);
        let tree_task = "Load Dependencies".to_string();
        Backend::progress_start_option_token(&con.clone(), &progress, &tree_task);
        let tree = get_tree(&project_kind, &con).await;
        Backend::progress_end_option_token(&con.clone(), &progress, &tree_task);

        let task = format!("Command: {COMMAND_RELOAD_DEPENDENCIES}");
        Backend::progress_start_option_token(&con.clone(), &progress, &task);
        let (sender, receiver) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
            percentage: 0,
            error: false,
            message: "...".to_string(),
        });
        if let ProjectKind::Maven { executable: _ } = project_kind {
            let cache = PathBuf::from(maven::compile::CLASSPATH_FILE);
            if cache.exists() {
                let _ = fs::remove_file(cache);
            }
        }
        if let Some(tree) = tree {
            let cache = project_cache_dir();
            tokio::select! {
                () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
                () = project_deps(sender, project_kind, class_map.clone(), false, &project_dir, &cache, &tree) => {}
            }
        }
        Backend::progress_end_option_token(&con.clone(), &progress, &task);
    });
    None
}
pub const UPDATE_DEPENDENCIES: &str = "UpdateDependencies";
pub fn update_dependencies(
    con: &Arc<Connection>,
    progress: Option<ProgressToken>,
    project_kind: &ProjectKind,
    class_map: &Arc<DashMap<MyString, Class>>,
    project_dir: &Path,
) {
    let repos = Arc::new(vec!["https://repo.maven.apache.org/maven2/".to_owned()]);
    let con = con.clone();
    let project_kind = project_kind.clone();
    let class_map = class_map.clone();
    let project_dir = project_dir.to_owned();
    tokio::spawn(async move {
        let progress = Arc::new(progress);
        let tree_task = "Load Dependencies".to_string();
        Backend::progress_start_option_token(&con.clone(), &progress, &tree_task);
        let tree = get_tree(&project_kind, &con).await;
        Backend::progress_end_option_token(&con.clone(), &progress, &tree_task);
        let task = format!("Command: {UPDATE_DEPENDENCIES}");
        Backend::progress_start_option_token(&con.clone(), &progress, &task);
        if let Some(tree) = tree {
            let (sender, receiver) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
                percentage: 0,
                error: false,
                message: "...".to_string(),
            });
            tokio::select! {
                () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
                () = update_report(project_kind.clone(), con.clone(), repos, &tree, sender) => {},
            }
            let (sender, receiver) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
                percentage: 0,
                error: false,
                message: "...".to_string(),
            });
            let cache = project_cache_dir();
            let task = format!("Command: {COMMAND_RELOAD_DEPENDENCIES}");
            Backend::progress_start_option_token(&con.clone(), &progress, &task);
            tokio::select! {
                () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
                () = project_deps(sender, project_kind, class_map.clone(), false, &project_dir, &cache, &tree) => {}
            }
            Backend::progress_end_option_token(&con.clone(), &progress, &task);
        }
        Backend::progress_end_option_token(&con.clone(), &progress, &task);
    });
}

#[must_use]
pub async fn get_tree(
    project_kind: &ProjectKind,
    con: &Arc<Connection>,
) -> Option<Vec<Dependency>> {
    let mut diagnostics = Vec::new();
    let range = Range::default();
    let out = match *project_kind {
        ProjectKind::Maven { ref executable } => match maven::tree::load(executable).await {
            Ok(t) => {
                let cache = PathBuf::from(maven::compile::CLASSPATH_FILE);
                if cache.exists() {
                    let _ = fs::remove_file(cache);
                }
                Some(t)
            }
            Err(MavenTreeError::GotError(e)) => {
                let message = format!("Unable load maven dependency tree {e:?}");
                diagnostics.push(Diagnostic::new(
                    range,
                    Some(DiagnosticSeverity::ERROR),
                    None,
                    Some(String::from(SERVER_NAME)),
                    message,
                    None,
                    None,
                ));
                None
            }
            Err(MavenTreeError::Cli(e)) => {
                let message = format!("Unable load maven dependency tree {e:?}");
                diagnostics.push(Diagnostic::new(
                    range,
                    Some(DiagnosticSeverity::ERROR),
                    None,
                    Some(String::from(SERVER_NAME)),
                    message,
                    None,
                    None,
                ));
                None
            }
            Err(MavenTreeError::UnknownDependencyScope(scope)) => {
                let message = format!("Unsupported dependency scope found: {scope:?}");
                diagnostics.push(Diagnostic::new(
                    range,
                    Some(DiagnosticSeverity::ERROR),
                    None,
                    Some(String::from(SERVER_NAME)),
                    message,
                    None,
                    None,
                ));
                None
            }
            Err(e) => {
                eprintln!("Failed to load tree: {e:?}");
                None
            }
        },
        ProjectKind::Gradle { ref executable, .. } => match gradle::tree::load(executable).await {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("Failed to load tree: {e:?}");
                None
            }
        },
        ProjectKind::Unknown => None,
    };
    report_maven_gradle_diagnostic(project_kind, con, diagnostics);
    out
}
