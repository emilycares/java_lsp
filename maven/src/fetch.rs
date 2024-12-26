use std::{
    path::{Path, PathBuf},
    process::Command,
};

use dashmap::DashMap;
use parser::dto::ClassFolder;

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

pub fn fetch_deps<'a>(
    class_map: &'a DashMap<std::string::String, parser::dto::Class>,
) -> Option<()> {
    let file_name = ".maven.cfc";
    let path = Path::new(&file_name);
    if path.exists() {
        if let Ok(classes) = parser::loader::load_class_folder("maven") {
            for class in classes.classes {
                class_map.insert(class.class_path.clone(), class);
            }
        }
    } else {
        eprintln!("dependency:unpack-dependencies start");
        let _output = Command::new("mvn")
            .args([
                "dependency:unpack-dependencies",
                "-Dmdep.useRepositoryLayout=true",
            ])
            .output()
            .ok()?;
        eprintln!("dependency:unpack-dependencies done");

        eprintln!("dependency:resolve (sources) start");
        let _output = Command::new("mvn")
            .args(["dependency:resolve", "-Dclassifier=sources"])
            .output()
            .ok()?;
        eprintln!("dependency:resolve (sources) done");

        eprintln!("dependency:resolve (javadoc) start");
        let _output = Command::new("mvn")
            .args(["dependency:resolve", "-Dclassifier=javadoc"])
            .output()
            .ok()?;
        eprintln!("dependency:resolve (javadoc) done");

        let tree = match tree::load() {
            Ok(tree) => tree,
            Err(e) => {
                eprintln!("failed to load tree: {:?}", e);
                return None;
            }
        };
        let mut maven_class_folder = ClassFolder::new();
        for dep in tree.deps {
            eprintln!("Loading dependency: {}", dep.artivact_id);
            let folder = pom_get_class_folder(&dep);
            if !folder.exists() {
                eprintln!("dependency folder does not exist {:?}", folder);
                continue;
            }
            let classes =
                parser::loader::load_classes(folder.as_path().to_str().unwrap_or_default());
            maven_class_folder.append(classes.clone());
            eprintln!("classes: {}", classes.classes.len());
            for class in classes.classes {
                class_map.insert(class.class_path.clone(), class);
            }
        }
        parser::loader::save_class_folder("maven", &maven_class_folder).unwrap();
    }

    None
}

pub fn pom_sources_jar<'a>(pom: &'a Pom, m2: &Path) -> PathBuf {
    get_pom_m2_classifier_path(pom, m2, "sources")
}
pub fn pom_javadoc_jar<'a>(pom: &'a Pom, m2: &Path) -> PathBuf {
    get_pom_m2_classifier_path(pom, m2, "javadoc")
}

fn get_pom_m2_classifier_path<'a>(pom: &'a Pom, m2: &Path, classifier: &str) -> PathBuf {
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

fn pom_get_class_folder<'a>(pom: &'a Pom) -> PathBuf {
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
