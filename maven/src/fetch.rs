use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};

use dashmap::DashMap;
use parser::{dto::ClassFolder, loader::SourceDestination};
use tokio::{process::Command, sync::Mutex};

use crate::tree::{self, Pom};

// New plan for resolving sources from mavnen classes and sources
// Inside of a class we must konw the dependency identifier. to load the source and docs for the
// class that this information is needed.
// mvn dependency:unpack-dependencies -Dmdep.useRepositoryLayout=true
//
// The sources and docs will be downloaded into the m2 with
// mvn dependency:resolve -Dclassifier=sources
// mvn dependency:resolve -Dclassifier=javadoc
//
// We also need to ajust that we load the classes from all dependencies using the tree.
// mvn dependency:tree | grep io.quarkus.quarkus-resteasy-reactive
//
//  Whit this tree we then konw witch classes to load
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
// Allso there might be the need for multiple classmaps because the of the scope that we are
// currently in. (There might be overwrites. Less important) But we should only find test classes
// from a test. And not from the implementation

pub async fn fetch_deps(
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Option<DashMap<std::string::String, parser::dto::Class>> {
    let file_name = ".maven.cfc";
    let path = Path::new(&file_name);
    if path.exists() {
        if let Ok(classes) = parser::loader::load_class_folder("maven") {
            for class in classes.classes {
                class_map.insert(class.class_path.clone(), class);
            }
        }
        None
    } else {
        // mvn dependency:unpack-dependencies -Dmdep.useRepositoryLayout=true
        let unpack = Command::new("mvn")
            .args([
                "dependency:unpack-dependencies",
                "-Dmdep.useRepositoryLayout=true",
            ])
            .output();
        // mvn dependency:resolve -Dclassifier=sources
        let res_src = Command::new("mvn")
            .args(["dependency:resolve", "-Dclassifier=sources"])
            .output();
        // mvn dependency:resolve -Dclassifier=javadoc
        let res_doc = Command::new("mvn")
            .args(["dependency:resolve", "-Dclassifier=javadoc"])
            .output();

        let _ = futures::future::join3(unpack, res_src, res_doc).await;

        let tree = match tree::load() {
            Ok(tree) => tree,
            Err(e) => {
                eprintln!("failed to load tree: {:?}", e);
                return None;
            }
        };
        let Some(home) = dirs::home_dir() else {
            eprintln!("Could not find home");
            return None;
        };
        let m2 = home.join(".m2");
        let m2 = Arc::new(m2);
        let class_map = Arc::new(class_map.clone());
        let maven_class_folder = Arc::new(Mutex::new(ClassFolder::default()));
        let mut handles = Vec::new();
        for dep in tree.deps {
            let m2 = m2.clone();
            let class_map = class_map.clone();
            let maven_class_folder = maven_class_folder.clone();
            handles.push(tokio::spawn(async move {
                eprintln!("Loading dependency: {}", dep.artivact_id);
                let folder = pom_get_class_folder(&dep);
                if !folder.exists() {
                    eprintln!("dependency folder does not exist {:?}", folder);
                } else {
                    let source_jar = pom_sources_jar(&dep, &m2);
                    let source = extract_jar(source_jar, "source");
                    let javadoc_jar = pom_javadoc_jar(&dep, &m2);
                    let _ = extract_jar(javadoc_jar, "javadoc");

                    let classes = parser::loader::load_classes(
                        folder.as_path().to_str().unwrap_or_default(),
                        SourceDestination::RelativeInFolder(source),
                    );
                    {
                        let mut guard = maven_class_folder.lock().await;
                        guard.append(classes.clone());
                    }
                    for class in classes.classes {
                        class_map.insert(class.class_path.clone(), class);
                    }
                }
            }));
        }
        futures::future::join_all(handles).await;
        let guard = maven_class_folder.lock().await;
        if let Err(e) = parser::loader::save_class_folder("maven", &guard) {
            eprintln!("Failed to save .maven.cfc because: {e}");
        };
        Some(Arc::try_unwrap(class_map).expect("Classmap should be free to take"))
    }
}

fn extract_jar(jar: PathBuf, folder_name: &str) -> String {
    let mut dir = jar.clone();
    dir.set_file_name("");
    dir = dir.join(folder_name);

    if let Ok(data) = fs::read(&jar) {
        let res = zip_extract::extract(Cursor::new(data), &dir, false);
        if let Err(e) = res {
            eprintln!("Unable to unzip: {:?}, {e}", jar);
        }
    }
    let source = dir.as_path().to_str().unwrap_or_default().to_string();
    source
}

pub fn pom_sources_jar(pom: &Pom, m2: &Path) -> PathBuf {
    get_pom_m2_classifier_path(pom, m2, "sources")
}
pub fn pom_javadoc_jar(pom: &Pom, m2: &Path) -> PathBuf {
    get_pom_m2_classifier_path(pom, m2, "javadoc")
}

fn get_pom_m2_classifier_path(pom: &Pom, m2: &Path, classifier: &str) -> PathBuf {
    let group_parts = pom.group_id.split(".");
    let mut p = m2.join("repository");
    for gp in group_parts {
        p = p.join(gp)
    }
    p = p.join(pom.artivact_id).join(pom.version).join(pom.version);
    let file_name = format!("{}-{}-{}.jar", pom.artivact_id, pom.version, classifier);
    p.set_file_name(file_name);
    p
}

fn pom_get_class_folder(pom: &Pom) -> PathBuf {
    let mut p = PathBuf::from("./target/dependency/");
    let group_parts = pom.group_id.split(".");
    for gp in group_parts {
        p = p.join(gp)
    }
    p = p.join(pom.artivact_id).join(pom.version);
    p
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use pretty_assertions::assert_eq;

    use crate::fetch::{pom_get_class_folder, pom_javadoc_jar, pom_sources_jar};

    #[test]
    fn sources_path_base() {
        let pom = crate::tree::Pom {
            group_id: "io.quarkus",
            artivact_id: "quarkus-resteasy-reactive",
            version: "3.7.2",
            scope: crate::tree::DependencyScope::Compile,
        };
        let out = pom_sources_jar(&pom, Path::new("~/.m2/"));

        assert_eq!(out, PathBuf::from("~/.m2/repository/io/quarkus/quarkus-resteasy-reactive/3.7.2/quarkus-resteasy-reactive-3.7.2-sources.jar"));
    }

    #[test]
    fn javadoc_path_base() {
        let pom = crate::tree::Pom {
            group_id: "io.quarkus",
            artivact_id: "quarkus-resteasy-reactive",
            version: "3.7.2",
            scope: crate::tree::DependencyScope::Compile,
        };
        let out = pom_javadoc_jar(&pom, Path::new("~/.m2/"));

        assert_eq!(out, PathBuf::from("~/.m2/repository/io/quarkus/quarkus-resteasy-reactive/3.7.2/quarkus-resteasy-reactive-3.7.2-javadoc.jar"));
    }

    #[test]
    fn pom_classfolder() {
        let pom = crate::tree::Pom {
            group_id: "io.quarkus",
            artivact_id: "quarkus-resteasy-reactive",
            version: "3.7.2",
            scope: crate::tree::DependencyScope::Compile,
        };
        let out = pom_get_class_folder(&pom);

        assert_eq!(
            out,
            PathBuf::from("./target/dependency/io/quarkus/quarkus-resteasy-reactive/3.7.2/")
        );
    }
}
