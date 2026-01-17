use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use common::{Dependency, TaskProgress, deps_dir};
use dashmap::DashMap;
use loader::{CFC_VERSION, LoaderError};
use my_string::MyString;
use parser::dto::{Class, ClassFolder};
use tokio::task::JoinSet;

use crate::{
    m2::{self, MTwoError},
    repository::Repository,
    update::{self, deps_base, deps_get_cfc, deps_get_source},
};

#[derive(Debug)]
pub enum MavenProjectError {
    MTwo(MTwoError),
}

pub async fn project_deps(
    class_map: Arc<DashMap<MyString, Class>>,
    sender: tokio::sync::watch::Sender<TaskProgress>,
    use_cache: bool,
    tree: &[Dependency],
    cache_path: PathBuf,
    repos: Arc<Vec<Repository>>,
) -> Result<(), MavenProjectError> {
    if use_cache
        && cache_path.exists()
        && let Ok(classes) = loader::load_class_folder(&cache_path)
    {
        for class in classes.classes {
            class_map.insert(class.class_path.clone(), class);
        }
        return Ok(());
    }

    let tasks_number = u32::try_from(tree.len() + 1).unwrap_or(1);
    let completed_number = Arc::new(AtomicU32::new(0));
    let mut handles = JoinSet::<Option<ClassFolder>>::new();
    let deps_path = Arc::new(deps_dir());
    let m2 = m2::get_maven_m2_folder().map_err(MavenProjectError::MTwo)?;
    let m2 = Arc::new(m2);

    let mut update_tree = Vec::new();

    for dep in tree {
        let sender = sender.clone();
        let completed_number = completed_number.clone();
        let deps_path = deps_path.clone();
        let m2 = m2.clone();

        let sender = Arc::new(sender);
        let deps_bas = deps_base(dep, &deps_path);
        if !deps_bas.exists() {
            let _ = fs::create_dir_all(&deps_bas);
        }
        let cfc = deps_get_cfc(&deps_bas, dep);

        let pom_mtwo = m2::pom_m2(dep, &m2);
        if !pom_mtwo.exists() {
            let _ = fs::create_dir_all(&pom_mtwo);
        }
        let jar = m2::pom_classes_jar(dep, &pom_mtwo);

        if !jar.exists() {
            update_tree.push(dep.to_owned());
            continue;
        }
        let dep = Arc::new(dep.to_owned());
        handles.spawn(async move {
            let sender = sender.clone();

            match loader::load_class_folder(&cfc) {
                Ok(classes) => {
                    let a = completed_number.fetch_add(1, Ordering::Relaxed);
                    let _ = sender.send(TaskProgress {
                        percentage: (100 * a) / (tasks_number + 1),
                        error: false,
                        message: dep.artivact_id.clone(),
                    });
                    Some(classes)
                }
                Err(LoaderError::InvalidCfcCache | LoaderError::IO(_)) => {
                    reindex(
                        tasks_number,
                        completed_number,
                        &dep,
                        deps_bas,
                        cfc,
                        jar,
                        sender,
                    )
                    .await
                }
                Err(e) => {
                    eprintln!("Parse error in {dep:?}, {e:?}");
                    None
                }
            }
        });
    }

    let u = update::update(repos, &update_tree, sender.clone()).await;
    let sender = Arc::new(sender);

    if u.is_ok() {
        for dep in update_tree {
            let completed_number = completed_number.clone();
            let sender = sender.clone();
            let pom_mtwo = m2::pom_m2(&dep, &m2);
            let jar = m2::pom_classes_jar(&dep, &pom_mtwo);
            let deps_bas = deps_base(&dep, &deps_path);
            let cfc = deps_get_cfc(&deps_bas, &dep);
            let dep = Arc::new(dep.clone());

            handles.spawn(async move {
                reindex(
                    tasks_number,
                    completed_number,
                    &dep,
                    deps_bas,
                    cfc,
                    jar,
                    sender,
                )
                .await
            });
        }
    }

    let done = handles.join_all().await;

    let maven_class_folder = ClassFolder {
        classes: done.into_iter().flatten().flat_map(|i| i.classes).collect(),
        version: CFC_VERSION,
    };

    if let Err(e) = loader::save_class_folder(&cache_path, &maven_class_folder) {
        eprintln!("Failed to save {} because: {e:?}", cache_path.display());
    }
    for class in maven_class_folder.classes {
        class_map.insert(class.class_path.clone(), class);
    }
    Ok(())
}

async fn reindex(
    tasks_number: u32,
    completed_number: Arc<AtomicU32>,
    dep: &Arc<Dependency>,
    deps_bas: PathBuf,
    cfc: PathBuf,
    jar: PathBuf,
    sender: Arc<tokio::sync::watch::Sender<TaskProgress>>,
) -> Option<ClassFolder> {
    let source = deps_get_source(&deps_bas);
    if let Some(source) = source.as_path().to_str() {
        match loader::load_classes_jar(
            &jar,
            parser::SourceDestination::RelativeInFolder(source.to_owned()),
        )
        .await
        {
            Ok(classes) => {
                let a = completed_number.fetch_add(1, Ordering::Relaxed);
                let _ = sender.send(TaskProgress {
                    percentage: (100 * a) / (tasks_number + 1),
                    error: false,
                    message: dep.artivact_id.clone(),
                });
                if let Err(e) = loader::save_class_folder(&cfc, &classes) {
                    eprintln!("Failed to save cache for {}, {e:?}", cfc.display());
                }

                return Some(classes);
            }
            Err(e) => {
                eprintln!("Failed to index jar: {}, {e:?}", jar.display());
            }
        }
    }
    None
}

#[must_use]
pub fn get_maven_cache_path(project_dir: &Path, project_cache_dir: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    project_dir.hash(&mut hasher);
    let s = format!("{}.maven.cfc", hasher.finish());
    project_cache_dir.join(s)
}

#[must_use]
pub fn load_project_folders(project_dir: &Path) -> Vec<Class> {
    let mut out = vec![];

    out.extend(loader::load_java_files(project_dir.join("src/main/java")));
    out.extend(loader::load_java_files(project_dir.join("src/test/java")));

    // list modules
    // mvn help:evaluate -Dexpression=project.modules
    out
}
