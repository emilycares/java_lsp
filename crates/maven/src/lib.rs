#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]

use std::path::Path;

use crate::{m2::get_maven_m2_folder, repository::Repository};
use repository::load_repositories;
pub mod compile;
pub mod config;
pub mod m2;
pub mod metadata;
pub mod pom;
pub mod project;
pub mod repository;
pub mod settings;
pub mod tree;
pub mod update;

#[must_use]
pub fn get_repositories(project_dir: &Path) -> Vec<Repository> {
    match get_maven_m2_folder() {
        Ok(m2_folder) => match load_repositories(&m2_folder, project_dir) {
            Ok(repositories) => return repositories,
            Err(e) => eprintln!("Got error loading repos: {e:?}"),
        },
        Err(e) => eprintln!("Got error loading m2 folder: {e:?}"),
    }
    vec![repository::central()]
}
