use std::{fs, path::PathBuf, sync::Arc};

use common::{TaskProgress, project_kind::ProjectKind};
use lsp_server::Connection;
use lsp_types::ProgressToken;
use my_string::MyString;

use crate::backend::{Backend, fetch_deps, read_forward};

pub const COMMAND_RELOAD_DEPENDENCIES: &str = "ReloadDependencies";
#[must_use]
pub fn reload_dependencies(
    con: &Arc<Connection>,
    progress: Option<ProgressToken>,
    project_kind: &ProjectKind,
    class_map: &Arc<dashmap::DashMap<MyString, parser::dto::Class>>,
) -> Option<serde_json::Value> {
    let con = con.clone();
    let project_kind = project_kind.clone();
    let class_map = class_map.clone();
    tokio::spawn(async move {
        let task = format!("Command: {COMMAND_RELOAD_DEPENDENCIES}");
        let progress = Arc::new(progress);
        Backend::progress_start_option_token(&con.clone(), &progress, &task);
        let (sender, receiver) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
            percentage: 0,
            error: false,
            message: "...".to_string(),
        });
        match project_kind {
            ProjectKind::Maven { executable: _ } => {
                let cache = PathBuf::from(maven::fetch::MAVEN_CFC);
                if cache.exists() {
                    let _ = fs::remove_file(cache);
                }
                let cache = PathBuf::from(maven::compile::CLASSPATH_FILE);
                if cache.exists() {
                    let _ = fs::remove_file(cache);
                }
            }
            ProjectKind::Gradle {
                executable: _,
                path_build_gradle: _,
            } => {
                let cache = PathBuf::from(gradle::fetch::GRADLE_CFC);
                if cache.exists() {
                    let _ = fs::remove_file(cache);
                }
            }
            ProjectKind::Unknown => (),
        }
        tokio::select! {
            () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
            () = fetch_deps(con.clone(), sender, project_kind, class_map.clone(), false) => {}
        }
        Backend::progress_end_option_token(&con.clone(), &progress, &task);
    });
    None
}
