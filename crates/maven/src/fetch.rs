use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use common::TaskProgress;
use dashmap::DashMap;
use my_string::MyString;
use parser::{
    SourceDestination,
    dto::{Class, ClassFolder},
};
use tokio::{process::Command, task::JoinSet};

use crate::{
    config::overwrite_settings_xml_tokio,
    tree::{self, Pom},
};

// New plan for resolving sources from mavnen classes and sources
// Inside of a class we must know the dependency identifier. to load the source and docs for the
// class that this information is needed.
// mvn dependency:unpack-dependencies -Dmdep.useRepositoryLayout=true
//
// The sources and docs will be downloaded into the m2 with
// mvn dependency:resolve -Dclassifier=sources
// mvn dependency:resolve -Dclassifier=javadoc
//
// We also need to adjust that we load the classes from all dependencies using the tree.
// mvn dependency:tree | grep io.quarkus.quarkus-resteasy-reactive
//
//  Whit this tree we then know witch classes to load
//  target/dependency/io/quarkus/quarkus-resteasy-reactive/3.7.2/**
//  example:
//  target/dependency/io/quarkus/quarkus-resteasy-reactive/3.7.2/io/quarkus/resteasy/reactive/server/Closer.class
//
//  and where to load the docs and sources from
//
// /home/emily/.m2/repository/io/quarkus/quarkus-resteasy-reactive/3.7.2/quarkus-resteasy-reactive-3.7.2-sources.jar
// /home/emily/.m2/repository/io/quarkus/quarkus-resteasy-reactive/3.7.2/quarkus-resteasy-reactive-3.7.2-javadoc.jar
//
// The idea is for definition that the editor can load files from inside zip files. If not the the
// we can extract it.
//
// Inside of the source file we should also be able to go to definition. Because everining should
// be in the class_map.
//
// Also there might be the need for multiple classmaps because the of the scope that we are
// currently in. (There might be overwrites. Less important) But we should only find test classes
// from a test. And not from the implementation

#[derive(Debug)]
pub enum MavenFetchError {
    NoHomeFound,
    Tree(tree::MavenTreeError),
    ParserLoader(loader::LoaderError),
    NoM2Folder,
    DownloadSources(String),
    FailedToResolveSources(std::io::Error),
}
const MAVEN_CFC: &str = ".maven.cfc";

pub async fn fetch_deps(
    class_map: Arc<DashMap<MyString, Class>>,
    sender: tokio::sync::watch::Sender<TaskProgress>,
    use_cache: bool,
    download: bool,
    maven_executable: &str,
) -> Result<(), MavenFetchError> {
    let path = Path::new(&MAVEN_CFC);
    if use_cache
        && path.exists()
        && let Ok(classes) = loader::load_class_folder(path)
    {
        for class in classes.classes {
            class_map.insert(class.class_path.clone(), class);
        }
        return Ok(());
    }

    if download {
        download_sources(&sender, maven_executable).await?;
    }
    let tree = tree::load(maven_executable).map_err(MavenFetchError::Tree)?;
    let m2 = Arc::new(get_maven_m2_folder()?);

    let tasks_number = u32::try_from(tree.deps.len() + 1).unwrap_or(1);
    let mut handles = JoinSet::<Option<ClassFolder>>::new();
    let completed_number = Arc::new(AtomicU32::new(0));
    let sender = Arc::new(sender);

    for dep in tree.deps {
        let m2 = m2.clone();
        let sender = sender.clone();
        let completed_number = completed_number.clone();

        let source_jar = pom_sources_jar(&dep, &m2);
        let mut source_dir = source_jar.clone();
        source_dir.set_file_name("");
        source_dir = source_dir.join("source");
        let source_dir_string = source_dir.as_path().to_str().unwrap_or_default().into();

        if !source_dir.exists() {
            handles.spawn(async move {
                match zip_util::extract_jar(&source_jar, &source_dir).await {
                    Ok(()) => (),
                    Err(e) => eprintln!("unable to extract jar {e:?}"),
                }
                None
            });
        }

        handles.spawn(async move {
            let sender = sender.clone();
            let classes_jar = pom_classes_jar(&dep, &m2);

            match loader::load_classes_jar(
                classes_jar,
                SourceDestination::RelativeInFolder(source_dir_string),
            )
            .await
            {
                Ok(classes) => {
                    let a = completed_number.fetch_add(1, Ordering::Relaxed);
                    let _ = sender.send(TaskProgress {
                        percentage: (100 * a) / (tasks_number + 1),
                        error: false,
                        message: dep.artivact_id,
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

    let maven_class_folder = ClassFolder {
        classes: done.into_iter().flatten().flat_map(|i| i.classes).collect(),
    };

    if let Err(e) = loader::save_class_folder(MAVEN_CFC, &maven_class_folder) {
        eprintln!("Failed to save {MAVEN_CFC} because: {e:?}");
    }
    for class in maven_class_folder.classes {
        class_map.insert(class.class_path.clone(), class);
    }
    Ok(())
}

async fn download_sources(
    sender: &tokio::sync::watch::Sender<TaskProgress>,
    maven_executable: &str,
) -> Result<(), MavenFetchError> {
    let _ = sender.send(TaskProgress {
        percentage: 0,
        error: false,
        message: "Downloading sources ...".to_string(),
    });
    // https://maven.apache.org/plugins/maven-dependency-plugin/resolve-mojo.html
    // mvn dependency:resolve -Dclassifier=sources
    let mut e = Command::new(maven_executable);
    let e = e.args(["dependency:resolve", "-Dclassifier=sources"]);
    let e = overwrite_settings_xml_tokio(e);
    let e = e
        .output()
        .await
        .map_err(MavenFetchError::FailedToResolveSources)?;
    if e.status.success() {
        let _ = sender.send(TaskProgress {
            percentage: 0,
            error: false,
            message: "Downloading sources Done".to_string(),
        });
    } else {
        let error = String::from_utf8_lossy(&e.stderr).to_string();
        return Err(MavenFetchError::DownloadSources(error));
    }
    Ok(())
}

fn get_maven_m2_folder() -> Result<PathBuf, MavenFetchError> {
    let Some(home) = dirs::home_dir() else {
        eprintln!("Could not find home");
        return Err(MavenFetchError::NoHomeFound);
    };
    let m2 = home.join(".m2");
    if !m2.exists() {
        return Err(MavenFetchError::NoM2Folder);
    }
    Ok(m2)
}

#[must_use]
pub fn pom_classes_jar(pom: &Pom, m2: &Path) -> PathBuf {
    get_pom_m2_classifier_path(pom, m2, None)
}
#[must_use]
pub fn pom_sources_jar(pom: &Pom, m2: &Path) -> PathBuf {
    get_pom_m2_classifier_path(pom, m2, Some("sources"))
}
#[must_use]
pub fn pom_javadoc_jar(pom: &Pom, m2: &Path) -> PathBuf {
    get_pom_m2_classifier_path(pom, m2, Some("javadoc"))
}

fn get_pom_m2_classifier_path(pom: &Pom, m2: &Path, classifier: Option<&str>) -> PathBuf {
    let group_parts = pom.group_id.split('.');
    let mut p = m2.join("repository");
    for gp in group_parts {
        p = p.join(gp);
    }
    p = p
        .join(&pom.artivact_id)
        .join(&pom.version)
        .join(&pom.version);

    if let Some(classifier) = classifier {
        let file_name = format!("{}-{}-{}.jar", pom.artivact_id, pom.version, classifier);
        p.set_file_name(file_name);
    } else {
        let file_name = format!("{}-{}.jar", pom.artivact_id, pom.version);
        p.set_file_name(file_name);
    }

    p
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use pretty_assertions::assert_eq;

    use crate::fetch::{pom_javadoc_jar, pom_sources_jar};

    #[test]
    fn sources_path_base() {
        let pom = crate::tree::Pom {
            group_id: "io.quarkus".to_string(),
            artivact_id: "quarkus-resteasy-reactive".to_string(),
            version: "3.7.2".to_string(),
            scope: crate::tree::DependencyScope::Compile,
        };
        let out = pom_sources_jar(&pom, Path::new("~/.m2/"));

        assert_eq!(
            out,
            PathBuf::from(
                "~/.m2/repository/io/quarkus/quarkus-resteasy-reactive/3.7.2/quarkus-resteasy-reactive-3.7.2-sources.jar"
            )
        );
    }

    #[test]
    fn javadoc_path_base() {
        let pom = crate::tree::Pom {
            group_id: "io.quarkus".to_string(),
            artivact_id: "quarkus-resteasy-reactive".to_string(),
            version: "3.7.2".to_string(),
            scope: crate::tree::DependencyScope::Compile,
        };
        let out = pom_javadoc_jar(&pom, Path::new("~/.m2/"));

        assert_eq!(
            out,
            PathBuf::from(
                "~/.m2/repository/io/quarkus/quarkus-resteasy-reactive/3.7.2/quarkus-resteasy-reactive-3.7.2-javadoc.jar"
            )
        );
    }
}
