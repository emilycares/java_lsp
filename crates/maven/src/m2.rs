use std::path::{Path, PathBuf};

use common::Dependency;

#[derive(Debug)]
pub enum MTwoError {
    NoHomeFound,
    NoM2Folder,
}

pub fn get_maven_m2_folder() -> Result<PathBuf, MTwoError> {
    let Some(home) = dirs::home_dir() else {
        eprintln!("Could not find home");
        return Err(MTwoError::NoHomeFound);
    };
    let m2 = home.join(".m2");
    if !m2.exists() {
        return Err(MTwoError::NoM2Folder);
    }
    Ok(m2)
}

#[must_use]
pub fn pom_classes_jar(pom: &Dependency, pom_mtwo: &PomMTwo) -> PathBuf {
    pom_m2_classifier_path(pom, pom_mtwo, None)
}
#[must_use]
pub fn pom_sources_jar(pom: &Dependency, pom_mtwo: &PomMTwo) -> PathBuf {
    pom_m2_classifier_path(pom, pom_mtwo, Some("sources"))
}
#[must_use]
pub fn pom_javadoc_jar(pom: &Dependency, pom_mtwo: &PomMTwo) -> PathBuf {
    pom_m2_classifier_path(pom, pom_mtwo, Some("javadoc"))
}

#[must_use]
fn pom_m2_classifier_path(
    pom: &Dependency,
    pom_mtwo: &PomMTwo,
    classifier: Option<&str>,
) -> PathBuf {
    let mut p = pom_mtwo.join("a");

    if let Some(classifier) = classifier {
        let file_name = format!("{}-{}-{}.jar", pom.artivact_id, pom.version, classifier);
        p.set_file_name(file_name);
    } else {
        let file_name = format!("{}-{}.jar", pom.artivact_id, pom.version);
        p.set_file_name(file_name);
    }

    p
}

#[must_use]
pub fn pom_m2_sha1(pom: &Dependency, pom_mtwo: &PomMTwo) -> PathBuf {
    let mut p = pom_mtwo.clone();

    let file_name = format!("{}-{}.jar.sha1", pom.artivact_id, pom.version);
    p.set_file_name(file_name);

    p
}

pub type PomMTwo = PathBuf;
#[must_use]
pub fn pom_m2(pom: &Dependency, m2: &Path) -> PomMTwo {
    let group_parts = pom.group_id.split('.');
    let mut p = m2.join("repository");
    for gp in group_parts {
        p = p.join(gp);
    }
    p.join(&pom.artivact_id).join(&pom.version)
}
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use common::Dependency;
    use pretty_assertions::assert_eq;

    use crate::m2::{pom_javadoc_jar, pom_m2, pom_sources_jar};

    #[test]
    fn sources_path_base() {
        let pom = Dependency {
            group_id: "io.quarkus".to_string(),
            artivact_id: "quarkus-resteasy-reactive".to_string(),
            version: "3.7.2".to_string(),
        };
        let pom_mtwo = pom_m2(&pom, &PathBuf::from("~/.m2/"));
        let out = pom_sources_jar(&pom, &pom_mtwo);

        assert_eq!(
            out,
            PathBuf::from(
                "~/.m2/repository/io/quarkus/quarkus-resteasy-reactive/3.7.2/quarkus-resteasy-reactive-3.7.2-sources.jar"
            )
        );
    }

    #[test]
    fn javadoc_path_base() {
        let pom = Dependency {
            group_id: "io.quarkus".to_string(),
            artivact_id: "quarkus-resteasy-reactive".to_string(),
            version: "3.7.2".to_string(),
        };
        let pom_mtwo = pom_m2(&pom, &PathBuf::from("~/.m2/"));
        let out = pom_javadoc_jar(&pom, &pom_mtwo);

        assert_eq!(
            out,
            PathBuf::from(
                "~/.m2/repository/io/quarkus/quarkus-resteasy-reactive/3.7.2/quarkus-resteasy-reactive-3.7.2-javadoc.jar"
            )
        );
    }
}
