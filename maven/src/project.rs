use std::path::PathBuf;

use parser::{dto::Class, load_src_folder};

pub fn load_project_folders() -> Vec<Class> {
    let mut out = vec![];
    // default maven project
    if let Some(nc) = load_src_folder(PathBuf::from("src/main/java")) {
        out.extend(nc.classes);
    }
    if let Some(nc) = load_src_folder(PathBuf::from("src/test/java")) {
        out.extend(nc.classes);
    }

    // list modules
    // mvn help:evaluate -Dexpression=project.modules
    out
}
