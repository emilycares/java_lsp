use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use common::{Dependency, TaskProgress, deps_dir};
use dashmap::DashMap;
use maven::update::{deps_base, deps_get_cfc};
use my_string::MyString;
use parser::dto::{Class, ClassFolder};
use tokio::task::JoinSet;

#[must_use]
pub fn load_project_folders(project_dir: &Path) -> Vec<Class> {
    let mut out = vec![];

    out.extend(loader::load_java_files(project_dir.join("src/main/java")));
    out.extend(loader::load_java_files(project_dir.join("src/test/java")));

    // list modules
    // mvn help:evaluate -Dexpression=project.modules
    out
}
#[derive(Debug)]
pub enum GradleProjectError {}

pub async fn project_deps(
    class_map: &DashMap<MyString, Class>,
    sender: tokio::sync::watch::Sender<TaskProgress>,
    use_cache: bool,
    project_dir: &Path,
    project_cache_dir: &Path,
    tree: &[Dependency],
) -> Result<(), GradleProjectError> {
    let cache_path = get_gradle_cache_path(project_dir, project_cache_dir);
    if use_cache
        && cache_path.exists()
        && let Ok(classes) = loader::load_class_folder(&cache_path)
    {
        for class in classes.classes {
            class_map.insert(class.class_path.clone(), class);
        }
        return Ok(());
    }

    let mut handles = JoinSet::<Option<ClassFolder>>::new();
    let tasks_number = u32::try_from(tree.len() + 1).unwrap_or(1);
    let completed_number = Arc::new(AtomicU32::new(0));
    let deps_path = Arc::new(deps_dir());

    for dep in tree {
        let sender = sender.clone();
        let completed_number = completed_number.clone();
        let deps_path = deps_path.clone();
        let dep = Arc::new(dep.to_owned());

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
                        message: dep.artivact_id.clone(),
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

    let gradle_class_folder = ClassFolder {
        classes: done.into_iter().flatten().flat_map(|i| i.classes).collect(),
    };
    if let Err(e) = loader::save_class_folder(&cache_path, &gradle_class_folder) {
        eprintln!("Failed to save {} because: {e:?}", cache_path.display());
    }
    for class in gradle_class_folder.classes {
        class_map.insert(class.class_path.clone(), class);
    }
    Ok(())
}

#[must_use]
pub fn get_gradle_cache_path(project_dir: &Path, project_cache_dir: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    project_dir.hash(&mut hasher);
    let s = format!("{}.gradle.cfc", hasher.finish());
    project_cache_dir.join(s)
}
