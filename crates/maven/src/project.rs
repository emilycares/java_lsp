use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use common::{TaskProgress, deps_dir};
use dashmap::DashMap;
use my_string::MyString;
use parser::dto::{Class, ClassFolder};
use tokio::task::JoinSet;

use crate::{
    tree::{self},
    update::{deps_base, deps_get_cfc},
};

#[derive(Debug)]
pub enum MavenProjectError {
    Tree(tree::MavenTreeError),
}

pub async fn project_deps(
    class_map: Arc<DashMap<MyString, Class>>,
    sender: tokio::sync::watch::Sender<TaskProgress>,
    use_cache: bool,
    maven_executable: &str,
    project_dir: &Path,
    project_cache_dir: &Path,
) -> Result<(), MavenProjectError> {
    let cache_path = get_maven_cache_path(project_dir, project_cache_dir);
    if use_cache
        && cache_path.exists()
        && let Ok(classes) = loader::load_class_folder(&cache_path)
    {
        for class in classes.classes {
            class_map.insert(class.class_path.clone(), class);
        }
        return Ok(());
    }

    let tree = tree::load(maven_executable).map_err(MavenProjectError::Tree)?;

    let tasks_number = u32::try_from(tree.len() + 1).unwrap_or(1);
    let completed_number = Arc::new(AtomicU32::new(0));
    let mut handles = JoinSet::<Option<ClassFolder>>::new();
    let sender = Arc::new(sender);
    let deps_path = Arc::new(deps_dir());

    for dep in tree {
        let sender = sender.clone();
        let completed_number = completed_number.clone();
        let deps_path = deps_path.clone();

        handles.spawn(async move {
            let sender = sender.clone();

            let deps_bas = deps_base(&dep, &deps_path);
            let cfc = deps_get_cfc(&deps_bas, &dep);
            match loader::load_class_folder(cfc) {
                Ok(classes) => {
                    let a = completed_number.fetch_add(1, Ordering::Relaxed);
                    let _ = sender.send(TaskProgress {
                        percentage: (100 * a) / (tasks_number + 1),
                        error: false,
                        message: dep.artivact_id,
                    });
                    Some(classes)
                }
                Err(e) => {
                    eprintln!("Parse error in {dep:?}, {e:?}");
                    None
                }
            }
        });
    }

    let done = handles.join_all().await;

    let maven_class_folder = ClassFolder {
        classes: done.into_iter().flatten().flat_map(|i| i.classes).collect(),
    };

    if let Err(e) = loader::save_class_folder(&cache_path, &maven_class_folder) {
        eprintln!("Failed to save {} because: {e:?}", cache_path.display());
    }
    for class in maven_class_folder.classes {
        class_map.insert(class.class_path.clone(), class);
    }
    Ok(())
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
