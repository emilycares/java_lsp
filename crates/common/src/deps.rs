use std::path::{Path, PathBuf};

use crate::Dependency;

pub type DepsSource = PathBuf;
#[must_use]
pub fn deps_get_source(deps_bas: &DepsBas) -> DepsSource {
    deps_bas.join("source")
}

type DepsHash = PathBuf;
#[must_use]
pub fn deps_get_hash(deps_bas: &DepsBas, pom: &Dependency) -> DepsHash {
    let mut p = deps_bas.join("a");
    let file_name = format!("{}-{}.hash", pom.artivact_id, pom.version);
    p.set_file_name(file_name);
    p
}
type DepsCFC = PathBuf;
#[must_use]
pub fn deps_get_cfc(deps_bas: &DepsBas, pom: &Dependency) -> DepsCFC {
    let mut p = deps_bas.join("a");
    let file_name = format!("{}-{}.cfc", pom.artivact_id, pom.version);
    p.set_file_name(file_name);
    p
}

type DepsEtag = PathBuf;
#[must_use]
pub fn deps_get_etag(deps_bas: &DepsBas, pom: &Dependency) -> DepsEtag {
    let mut p = deps_bas.join("a");
    let file_name = format!("{}-{}.etag", pom.artivact_id, pom.version);
    p.set_file_name(file_name);
    p
}

pub type DepsBas = PathBuf;
#[must_use]
pub fn deps_base(pom: &Dependency, deps_path: &Path) -> DepsBas {
    let group_parts = pom.group_id.split('.');
    let mut p = deps_path.to_path_buf();
    for gp in group_parts {
        p = p.join(gp);
    }
    p = p.join(&pom.artivact_id).join(&pom.version);
    p
}
