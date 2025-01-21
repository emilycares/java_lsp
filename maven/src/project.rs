use std::path::PathBuf;

use parser::{dto::Class, src_folder_paths};

pub fn load_project_folders() -> Vec<Class> {
    let paths = get_paths();

    parser::loader::load_java_files(paths).classes
    // list modules
    // mvn help:evaluate -Dexpression=project.modules
}

pub fn get_paths() -> Vec<String> {
    let mut paths = vec![];
    paths.extend(src_folder_paths(PathBuf::from("src/main/java")));
    paths.extend(src_folder_paths(PathBuf::from("src/test/java")));
    paths
}
