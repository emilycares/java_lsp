use parser::dto::Class;

#[must_use]
pub fn load_project_folders() -> Vec<Class> {
    let mut out = vec![];
    let Ok(current_dir) = std::env::current_dir() else {
        return vec![];
    };

    out.extend(loader::load_java_files(current_dir.join("src/main/java")));
    out.extend(loader::load_java_files(current_dir.join("src/test/java")));

    // list modules
    // mvn help:evaluate -Dexpression=project.modules
    out
}
