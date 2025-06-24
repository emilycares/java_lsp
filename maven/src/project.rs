use parser::dto::Class;

pub async fn load_project_folders() -> Vec<Class> {
    let mut out = vec![];
    let Ok(current_dir) = std::env::current_dir() else {
        return vec![];
    };

    out.extend(parser::loader::load_java_files(current_dir.join("src/main/java")).await);
    out.extend(parser::loader::load_java_files(current_dir.join("src/text/java")).await);

    // list modules
    // mvn help:evaluate -Dexpression=project.modules
    out
}
