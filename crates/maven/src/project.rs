use std::path::Path;

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
