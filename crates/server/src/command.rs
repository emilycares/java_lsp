use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use common::{
    Dependency, TaskProgress, cache_dir, project_cache_dir,
    project_kind::{ProjectKind, get_project_kind},
};
use dto::Class;
use gradle::project::get_gradle_cache_path;
use lsp_extra::SERVER_NAME;
use lsp_server::Connection;
use lsp_types::{Diagnostic, DiagnosticSeverity, ProgressToken, Range};
use maven::{tree::MavenTreeError, update};
use my_string::MyString;

use crate::backend::{
    Backend, project_deps, read_forward, report_maven_gradle_diagnostic, update_report,
};

pub const COMMAND_RELOAD_DEPENDENCIES: &str = "ReloadDependencies";
#[must_use]
pub fn reload_dependencies(
    con: &Arc<Connection>,
    progress: Option<ProgressToken>,
    project_kind: &ProjectKind,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    project_dir: &Path,
) -> Option<serde_json::Value> {
    match project_kind.clone() {
        ProjectKind::Maven { .. } => {
            let repos = Arc::new(maven::get_repositories(project_dir));
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
                let (sender, receiver) =
                    tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
                        percentage: 0,
                        error: false,
                        message: "...".to_string(),
                    });

                let cache = PathBuf::from(maven::compile::CLASSPATH_FILE);
                if cache.exists() {
                    let _ = fs::remove_file(cache);
                }
                if let Some(tree) = tree {
                    let cache = cache_dir();
                    tokio::select! {
                        () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
                        () = project_deps(sender, project_kind, class_map.clone(), false, &project_dir, &cache, &tree, repos) => {}
                    }
                }
                Backend::progress_end_option_token(&con.clone(), &progress, &task);
            });
        }
        ProjectKind::Gradle { executable, .. } => {
            reload_gradle_project(con, class_map, project_dir, executable);
        }
        ProjectKind::Unknown => (),
    }
    None
}

/// For gradle there is no difference between update and reload
/// Does not use a cache
fn reload_gradle_project(
    con: &Arc<Connection>,
    class_map: &Arc<Mutex<HashMap<my_string::smol_str::SmolStr, Class>>>,
    project_dir: &Path,
    executable: String,
) {
    let project_dir = project_dir.to_owned();
    let con = con.clone();
    let class_map = class_map.clone();
    tokio::spawn(async move {
        let task = "Load gradle project".to_string();
        let progress = Arc::new(Option::Some(ProgressToken::String(task.clone())));
        Backend::progress_start_option_token(&con.clone(), &progress, &task);
        let project_cache_dir = project_cache_dir();

        let cache_path = get_gradle_cache_path(project_dir.as_path(), project_cache_dir.as_path());
        let (sender, receiver) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
            percentage: 0,
            error: false,
            message: "...".to_string(),
        });
        tokio::select! {
            () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
            () = gradle::project::index_project(class_map.clone(), sender, false, cache_path, executable.clone()) => {}
        }
        Backend::progress_end_option_token(&con, &progress, &task);
    });
}

pub async fn reload_dependencies_cli() {
    let (con, _) = Connection::memory();
    let con = Arc::new(con);
    let Ok(project_dir) = std::env::current_dir() else {
        return;
    };
    let Some(path) = std::env::var_os("PATH") else {
        return;
    };
    let Ok(project_kind) = get_project_kind(&project_dir, &path) else {
        return;
    };
    match project_kind {
        ProjectKind::Maven { .. } => {
            reload_dependencies_maven_cli(con, project_dir, project_kind).await;
        }
        ProjectKind::Gradle { executable, .. } => {
            reload_gradle_project_cli(con, project_dir, executable).await;
        }
        ProjectKind::Unknown => (),
    }
}

async fn reload_dependencies_maven_cli(
    con: Arc<Connection>,
    project_dir: PathBuf,
    project_kind: ProjectKind,
) {
    let progress = Arc::new(None);
    let class_map = Arc::new(Mutex::new(HashMap::new()));
    let repos = Arc::new(maven::get_repositories(&project_dir));
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
        let cache = cache_dir();
        tokio::select! {
            () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
            () = project_deps(sender, project_kind, class_map.clone(), false, &project_dir, &cache, &tree, repos) => {}
        }
    }
    Backend::progress_end_option_token(&con.clone(), &progress, &task);
}

pub const UPDATE_DEPENDENCIES: &str = "UpdateDependencies";
pub fn update_dependencies(
    con: &Arc<Connection>,
    progress: Option<ProgressToken>,
    project_kind: &ProjectKind,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    project_dir: &Path,
) {
    match project_kind.clone() {
        ProjectKind::Maven { .. } => {
            update_dependencies_maven(con, progress, project_kind, class_map, project_dir);
        }
        ProjectKind::Gradle { executable, .. } => {
            reload_gradle_project(con, class_map, project_dir, executable);
        }
        ProjectKind::Unknown => (),
    }
}
pub fn update_dependencies_maven(
    con: &Arc<Connection>,
    progress: Option<ProgressToken>,
    project_kind: &ProjectKind,
    class_map: &Arc<Mutex<HashMap<MyString, Class>>>,
    project_dir: &Path,
) {
    let repos = Arc::new(maven::get_repositories(project_dir));
    let con = con.clone();
    let project_kind = project_kind.clone();
    let project_dir = project_dir.to_owned();
    let class_map = class_map.clone();
    tokio::spawn(async move {
        let progress = Arc::new(progress);

        let task = "Load Dependency Tree".to_string();
        Backend::progress_start_option_token(&con.clone(), &progress, &task);
        let tree = get_tree(&project_kind, &con).await;
        Backend::progress_end_option_token(&con.clone(), &progress, &task);

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
                () = update_report(project_kind.clone(), con.clone(), repos.clone(), &tree, sender) => {},
            }
            let (sender, receiver) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
                percentage: 0,
                error: false,
                message: "...".to_string(),
            });
            let cache = cache_dir();
            let task = format!("Command: {COMMAND_RELOAD_DEPENDENCIES}");
            Backend::progress_start_option_token(&con.clone(), &progress, &task);
            tokio::select! {
                () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
                () = project_deps(sender, project_kind, class_map.clone(), false, &project_dir, &cache, &tree, repos) => {}
            }
            Backend::progress_end_option_token(&con.clone(), &progress, &task);
        }
        Backend::progress_end_option_token(&con.clone(), &progress, &task);
    });
}
pub async fn update_dependencies_cli() {
    let (con, _) = Connection::memory();
    let con = Arc::new(con);
    let Ok(project_dir) = std::env::current_dir() else {
        return;
    };
    let Some(path) = std::env::var_os("PATH") else {
        return;
    };
    let Ok(project_kind) = get_project_kind(&project_dir, &path) else {
        return;
    };
    match project_kind {
        ProjectKind::Maven { .. } => {
            update_dependencies_maven_cli(con, project_dir, project_kind).await;
        }
        ProjectKind::Gradle { executable, .. } => {
            reload_gradle_project_cli(con, project_dir, executable).await;
        }
        ProjectKind::Unknown => (),
    }
}

/// For gradle there is no difference between update and reload
/// Does not use a cache
async fn reload_gradle_project_cli(con: Arc<Connection>, project_dir: PathBuf, executable: String) {
    let class_map = Arc::new(Mutex::new(HashMap::new()));

    let task = "Load gradle project".to_string();
    let progress = Arc::new(Option::Some(ProgressToken::String(task.clone())));
    Backend::progress_start_option_token(&con.clone(), &progress, &task);
    let project_cache_dir = project_cache_dir();

    let cache_path = get_gradle_cache_path(project_dir.as_path(), project_cache_dir.as_path());
    let (sender, receiver) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
        percentage: 0,
        error: false,
        message: "...".to_string(),
    });
    tokio::select! {
        () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
        () = gradle::project::index_project(class_map.clone(), sender, false, cache_path, executable.clone()) => {}
    }
    Backend::progress_end_option_token(&con, &progress, &task);
}

async fn update_dependencies_maven_cli(
    con: Arc<Connection>,
    project_dir: PathBuf,
    project_kind: ProjectKind,
) {
    let class_map = Arc::new(Mutex::new(HashMap::new()));
    let repos = Arc::new(maven::get_repositories(&project_dir));
    let progress = Arc::new(None);

    let task = "Load Dependency tree".to_string();
    Backend::progress_start_option_token(&con.clone(), &progress, &task);
    let tree = get_tree(&project_kind, &con).await;
    Backend::progress_end_option_token(&con.clone(), &progress, &task);

    let task = format!("Command: {UPDATE_DEPENDENCIES}");
    Backend::progress_start_option_token(&con.clone(), &progress, &task);
    if let Some(tree) = tree {
        let sender = tokio::sync::watch::Sender::default();
        let _ = update::update(repos.clone(), &tree, sender).await;
        let sender = tokio::sync::watch::Sender::default();
        let cache = cache_dir();
        let task = format!("Command: {COMMAND_RELOAD_DEPENDENCIES}");
        Backend::progress_start_option_token(&con.clone(), &progress, &task);
        project_deps(
            sender,
            project_kind,
            class_map.clone(),
            false,
            &project_dir,
            &cache,
            &tree,
            repos,
        )
        .await;
        Backend::progress_end_option_token(&con.clone(), &progress, &task);
    }
    Backend::progress_end_option_token(&con.clone(), &progress, &task);
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
        ProjectKind::Gradle { .. } | ProjectKind::Unknown => None,
    };
    report_maven_gradle_diagnostic(project_kind, con, diagnostics);
    out
}
