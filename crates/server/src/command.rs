use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, RwLock},
    time::UNIX_EPOCH,
};

use common::{
    Dependency, TaskProgress, cache_dir, project_cache_dir,
    project_kind::{ProjectKind, get_project_kind},
};
use dto::Class;
use gradle::project::get_gradle_cache_path;
use lsp_extra::SERVER_NAME;
use lsp_server::Connection;
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, ProgressToken, Range};
use maven::{tree::MavenTreeError, update};
use my_string::MyString;
use serde_json::Value;
use tokio::task::JoinSet;

use crate::{
    backend::{
        Backend, Project, get_project_artifacts, project_deps, read_forward,
        report_maven_gradle_diagnostic, update_report,
    },
    command,
};

#[derive(Debug)]
pub enum CommandError {
    Io(std::io::Error),
    FileCreate(std::io::Error),
    WriteFile(std::io::Error),
    Command(std::io::Error),
}

pub const COMMAND_RELOAD_DEPENDENCIES: &str = "ReloadDependencies";
#[must_use]
pub fn reload_dependencies(
    con: Arc<Connection>,
    progress: Option<ProgressToken>,
    class_map: Arc<RwLock<HashMap<MyString, Class>>>,
    projects: &Arc<RwLock<Vec<Project>>>,
) -> Option<serde_json::Value> {
    let project_artifacts = projects.read().map_or_else(
        |_| Arc::new(Vec::new()),
        |projs| get_project_artifacts(&projs),
    );
    let Ok(projs) = projects.read() else {
        return None;
    };
    let projs = projs.clone();
    tokio::spawn(async move {
        let mut handles = JoinSet::new();
        let progress = Arc::new(progress);

        for p in projs.clone() {
            match p.kind {
                ProjectKind::Maven { .. } => {
                    reload_maven_project(
                        &con,
                        progress.clone(),
                        &class_map,
                        &mut handles,
                        &project_artifacts,
                        &p,
                    );
                }
                ProjectKind::Gradle { executable, .. } => {
                    reload_gradle_project(
                        &con,
                        &class_map,
                        PathBuf::from(p.dir.clone()).as_path(),
                        executable,
                        &mut handles,
                    );
                }
                ProjectKind::Unknown => (),
            }
        }
        let _ = handles.join_all().await;
    });
    None
}

pub fn reload_maven_project(
    con: &Arc<Connection>,
    progress: Arc<Option<ProgressToken>>,
    class_map: &Arc<RwLock<HashMap<my_string::smol_str::SmolStr, Class>>>,
    handles: &mut JoinSet<()>,
    project_artifacts: &Arc<Vec<String>>,
    p: &Project,
) {
    let con = con.clone();
    let task = format!("Load maven dependencies {}", p.artifact_id);
    let class_map = class_map.clone();
    let project_dir = PathBuf::from(p.dir.clone());
    let repos = Arc::new(maven::get_repositories(&project_dir));
    let project_kind = p.kind.clone();
    let project_artifacts = project_artifacts.clone();
    handles.spawn(async move {
        let project_dir = project_dir.as_path();
        Backend::progress_start_option_token(&con.clone(), &progress, &task);
        let (sender, receiver) =
            tokio::sync::watch::channel::<TaskProgress>(TaskProgress {
                percentage: 0,
                error: false,
                message: "...".to_string(),
            });
        let cache = cache_dir();
        let tree = command::get_tree(&project_kind, &con).await;
        if let Some(tree) = tree {
            tokio::select! {
                () = read_forward(receiver, con.clone(), task.clone(), progress.clone())  => {},
                () = project_deps(sender, project_kind.clone(), class_map.clone(), true, project_dir, &cache, &tree, repos, project_artifacts) => {}
            }
        }
        Backend::progress_end_option_token(&con, &progress, &task);
    });
}

/// For gradle there is no difference between update and reload
/// Does not use a cache
pub fn reload_gradle_project(
    con: &Arc<Connection>,
    class_map: &Arc<RwLock<HashMap<my_string::smol_str::SmolStr, Class>>>,
    project_dir: &Path,
    executable: String,
    handles: &mut JoinSet<()>,
) {
    let project_dir = project_dir.to_owned();
    let con = con.clone();
    let class_map = class_map.clone();
    handles.spawn(async move {
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
    let class_map = Arc::new(RwLock::new(HashMap::new()));
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
            () = project_deps(sender, project_kind, class_map.clone(), false, &project_dir, &cache, &tree, repos, Arc::new(Vec::new())) => {}
        }
    }
    Backend::progress_end_option_token(&con.clone(), &progress, &task);
}

pub const COMMAND_UPDATE_DEPENDENCIES: &str = "UpdateDependencies";
pub fn update_dependencies(
    con: Arc<Connection>,
    progress: Option<ProgressToken>,
    class_map: Arc<RwLock<HashMap<MyString, Class>>>,
    projects: &Arc<RwLock<Vec<Project>>>,
) {
    let Ok(projs) = projects.read() else {
        return;
    };
    let projs = projs.clone();
    tokio::spawn(async move {
        let mut handles = JoinSet::new();
        let progress = Arc::new(progress);
        let project_artifacts = get_project_artifacts(&projs);

        for p in projs.clone() {
            match p.kind {
                ProjectKind::Maven { .. } => {
                    update_dependencies_maven(
                        &con,
                        progress.clone(),
                        &p.kind,
                        &class_map,
                        PathBuf::from(p.dir.clone()).as_path(),
                        &project_artifacts,
                        &mut handles,
                    );
                }
                ProjectKind::Gradle { executable, .. } => {
                    reload_gradle_project(
                        &con,
                        &class_map,
                        PathBuf::from(p.dir.clone()).as_path(),
                        executable,
                        &mut handles,
                    );
                }
                ProjectKind::Unknown => (),
            }
        }
        let _ = handles.join_all().await;
    });
}
pub fn update_dependencies_maven(
    con: &Arc<Connection>,
    progress: Arc<Option<ProgressToken>>,
    project_kind: &ProjectKind,
    class_map: &Arc<RwLock<HashMap<MyString, Class>>>,
    project_dir: &Path,
    project_artifacts: &Arc<Vec<String>>,
    handles: &mut JoinSet<()>,
) {
    let repos = Arc::new(maven::get_repositories(project_dir));
    let con = con.clone();
    let project_kind = project_kind.clone();
    let project_dir = project_dir.to_owned();
    let class_map = class_map.clone();
    let project_artifacts = project_artifacts.clone();
    handles.spawn(async move {
        let task = "Load Dependency Tree".to_string();
        Backend::progress_start_option_token(&con.clone(), &progress, &task);
        let tree = get_tree(&project_kind, &con).await;
        Backend::progress_end_option_token(&con.clone(), &progress, &task);

        let task = format!("Command: {COMMAND_UPDATE_DEPENDENCIES}");
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
                () = project_deps(sender, project_kind, class_map.clone(), false, &project_dir, &cache, &tree, repos, project_artifacts) => {}
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
    let class_map = Arc::new(RwLock::new(HashMap::new()));

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
    let class_map = Arc::new(RwLock::new(HashMap::new()));
    let repos = Arc::new(maven::get_repositories(&project_dir));
    let progress = Arc::new(None);

    let task = "Load Dependency tree".to_string();
    Backend::progress_start_option_token(&con.clone(), &progress, &task);
    let tree = get_tree(&project_kind, &con).await;
    Backend::progress_end_option_token(&con.clone(), &progress, &task);

    let task = format!("Command: {COMMAND_UPDATE_DEPENDENCIES}");
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
            Arc::new(Vec::new()),
        )
        .await;
        Backend::progress_end_option_token(&con.clone(), &progress, &task);
    }
    Backend::progress_end_option_token(&con.clone(), &progress, &task);
}

pub const COMMAND_CMD: &str = "java_lsp.cmd";
pub const COMMAND_CMD_EDITOR: &str = "java_lsp.cmd.editor";
pub fn cmd(
    con: &Arc<Connection>,
    arguments: &[Value],
    token: Option<NumberOrString>,
) -> Result<(), CommandError> {
    let mut it = arguments.iter();
    let _name = it.next();
    let Some(Value::String(e)) = it.next() else {
        return Ok(());
    };
    let mut args = Vec::with_capacity(arguments.len().saturating_sub(1));
    while let Some(Value::String(a)) = it.next() {
        args.push(a.clone());
    }
    let token = Arc::new(token);
    let mut cmd_str = String::from(e);
    cmd_str.push('_');
    let j = args.join("_");
    cmd_str.push_str(&j);

    Backend::progress_start_option_token(con, &token, &cmd_str);
    let out = Command::new(e)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(CommandError::Command)?;
    if let Some(temp) = dirs::temp_dir()
        && let Ok(now) = std::time::SystemTime::now().duration_since(UNIX_EPOCH)
    {
        #[cfg(windows)]
        let cmd_str = cmd_str.replace('\\', ".");
        #[cfg(not(windows))]
        let cmd_str = cmd_str.replace('/', ".");
        let mut path = temp.join(format!("{}_{cmd_str}", now.as_secs()));
        path.set_extension("log");
        if let Some(p) = path.to_str() {
            Backend::open_log(con, p);
        }
        File::create(&path).map_err(CommandError::FileCreate)?;
        std::fs::write(&path, out.stderr).map_err(CommandError::WriteFile)?;
        std::fs::write(path, out.stdout).map_err(CommandError::WriteFile)?;
    }

    Backend::progress_end_option_token(con, &token, "Run command");
    Ok(())
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
