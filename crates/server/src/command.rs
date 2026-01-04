use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use common::{TaskProgress, project_cache_dir, project_kind::ProjectKind};
use dashmap::DashMap;
use lsp_server::Connection;
use lsp_types::ProgressToken;
use maven::update;
use my_string::MyString;
use parser::dto::Class;

use crate::backend::{Backend, fetch_deps, read_forward, update_report};

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
        let task = format!("Command: {COMMAND_RELOAD_DEPENDENCIES}");
        let progress = Arc::new(progress);
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
        let cache = project_cache_dir();
        tokio::select! {
            () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
            () = fetch_deps(con.clone(), sender, project_kind, class_map.clone(), false, &project_dir, &cache) => {}
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
) {
    let repos = Arc::new(vec!["https://repo.maven.apache.org/maven2/".to_owned()]);
    let con = con.clone();
    let project_kind = Arc::new(project_kind.clone());
    tokio::spawn(async move {
        let task = format!("Command: {UPDATE_DEPENDENCIES}");
        let progress = Arc::new(progress);
        Backend::progress_start_option_token(&con.clone(), &progress, &task);
        let (sender, receiver) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
            percentage: 0,
            error: false,
            message: "...".to_string(),
        });
        let tree = match &*project_kind {
            ProjectKind::Maven { executable } => match maven::tree::load(executable) {
                Ok(t) => Some(t),
                Err(e) => {
                    eprintln!("Failed to load tree: {e:?}");
                    None
                }
            },
            ProjectKind::Gradle {
                executable,
                path_build_gradle: _,
            } => match gradle::tree::load(executable) {
                Ok(t) => Some(t),
                Err(e) => {
                    eprintln!("Failed to load tree: {e:?}");
                    None
                }
            },
            ProjectKind::Unknown => None,
        };
        dbg!(&tree);
        if let Some(tree) = tree {
            tokio::select! {
                () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
                () = update_report(con.clone(), update::update(repos, tree, sender).await) => {},
            }
        }
        Backend::progress_end_option_token(&con.clone(), &progress, &task);
    });
}
