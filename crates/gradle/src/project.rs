use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

use parser::dto::Class;

#[must_use]
pub fn load_project_folders(project_dir: &Path) -> Vec<Class> {
    let mut out = vec![];

    out.extend(loader::load_java_files(project_dir.join("src/main/java")));
    out.extend(loader::load_java_files(project_dir.join("src/test/java")));

    // list modules
    // mvn help:evaluate -Dexpression=project.modules
    out
}

#[must_use]
pub fn get_gradle_cache_path(project_dir: &Path, project_cache_dir: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    project_dir.hash(&mut hasher);
    let s = format!("{}.gradle.cfc", hasher.finish());
    project_cache_dir.join(s)
}
