use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use dashmap::DashMap;
use itertools::Itertools;
use parking_lot::Mutex;
use parser::{dto::ClassFolder, loader::SourceDestination};

use crate::tree::{self, GradleTreeError};

#[derive(Debug)]
pub enum GradleFetchError {
    Tree(GradleTreeError),
    NoWorkToDo,
    CouldNotReadBuildGradle(std::io::Error),
    CouldNotModifyBuildGradle(std::io::Error),
    GradlewExecFailed(std::io::Error),
    CouldNotGetUnpackFolder,
    StatusCode,
    StatusCodeErrMessageNotutf8,
}

#[cfg(target_os = "linux")]
pub(crate) const PATH_GRADLE: &str = "./gradlew";
#[cfg(target_os = "windows")]
pub(crate) const PATH_GRADLE: &str = "./gradlew.bat";

const GRADLE_FILE_PATH: &str = "./build.gradle";

pub async fn fetch_deps(
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Result<DashMap<std::string::String, parser::dto::Class>, GradleFetchError> {
    let file_name = ".gradle.cfc";
    let path = Path::new(&file_name);
    if path.exists() {
        if let Ok(classes) = parser::loader::load_class_folder("gradle") {
            for class in classes.classes {
                class_map.insert(class.class_path.clone(), class);
            }
        }
        return Err(GradleFetchError::NoWorkToDo);
    } else {
        let unpack_folder = unpack_dependencies()?;
        let tree = tree::load().map_err(|e| GradleFetchError::Tree(e))?;
        let class_map = Arc::new(class_map.clone());
        let maven_class_folder = Arc::new(Mutex::new(ClassFolder::default()));
        let mut handles = Vec::new();
        let folders: Vec<_> = tree
            .iter()
            .map(|dep| {
                PathBuf::from(&unpack_folder).join(format!("{}-{}", dep.artivact_id, dep.version))
            })
            .unique()
            .collect();

        for folder in folders {
            let class_map = class_map.clone();
            let maven_class_folder = maven_class_folder.clone();
            handles.push(tokio::spawn(async move {
                if !folder.exists() {
                    eprintln!("dependency folder does not exist {:?}", folder);
                } else {
                    let classes = parser::loader::load_classes(
                        folder.as_path().to_str().unwrap_or_default(),
                        SourceDestination::None,
                    );
                    {
                        let mut guard = maven_class_folder.lock();
                        guard.append(classes.clone());
                    }
                    for class in classes.classes {
                        class_map.insert(class.class_path.clone(), class);
                    }
                }
            }));
        }

        futures::future::join_all(handles).await;
        let guard = maven_class_folder.lock();
        if let Err(e) = parser::loader::save_class_folder("gradle", &guard) {
            eprintln!("Failed to save .gradle.cfc because: {e}");
        };
        Ok(Arc::try_unwrap(class_map).expect("Classmap should be free to take"))
    }
}

// println configurations.getAll()
// [configuration ':annotationProcessor'
// configuration ':apiElements'
// configuration ':archives'
// configuration ':compileClasspath'
// configuration ':compileOnly'
// configuration ':default'
// configuration ':implementation'
// configuration ':mainSourceElements'
// configuration ':runtimeClasspath'
// configuration ':runtimeElements'
// configuration ':runtimeOnly'
// configuration ':testAnnotationProcessor'
// configuration ':testCompileClasspath'
// configuration ':testCompileOnly'
// configuration ':testImplementation'
// configuration ':testResultsElementsForTest'
// configuration ':testRuntimeClasspath'
// configuration ':testRuntimeOnly']
const UNPACK_DEPENCENCIES_TASK: &str = r#"
task unpackDependencies(type: Copy) {
    from configurations.runtimeClasspath
    from configurations.compileClasspath
    from configurations.testRuntimeClasspath
    from configurations.testCompileClasspath
    into "$buildDir/unpacked-dependencies"
    println "STARTPATH_$buildDir/unpacked-dependencies"
    eachFile { file ->
        if (file.name.endsWith('.jar')) {
            // Unpack the JAR files
            def jarFile = file.file
            def destDir = new File("$buildDir/unpacked-dependencies/${file.name.replace('.jar', '')}")
            copy {
                from zipTree(jarFile)
                into destDir
            }
        }
    }
}
"#;
fn unpack_dependencies() -> Result<String, GradleFetchError> {
    match fs::read_to_string(GRADLE_FILE_PATH) {
        Err(e) => Err(GradleFetchError::CouldNotReadBuildGradle(e)),
        Ok(gradle_content) => {
            if !gradle_content.contains(UNPACK_DEPENCENCIES_TASK) {
                write_build_gradle(format!("{}\n{}", gradle_content, UNPACK_DEPENCENCIES_TASK))?;
            }
            let out = match Command::new(PATH_GRADLE).arg("unpackDependencies").output() {
                Err(e) => Err(GradleFetchError::GradlewExecFailed(e)),
                Ok(output) => match output.status.code() {
                    Some(1) => match std::str::from_utf8(&output.stderr) {
                        Ok(stderr) => {
                            eprintln!("Got error from gradle: \n {}", stderr);
                            Err(GradleFetchError::StatusCode)
                        }
                        Err(_) => Err(GradleFetchError::StatusCodeErrMessageNotutf8),
                    },
                    Some(_) => {
                        let stdout = std::str::from_utf8(&output.stdout).expect("asd");
                        match get_unpack_folder(stdout) {
                            Some(path) => Ok(path.to_owned()),
                            None => Err(GradleFetchError::CouldNotGetUnpackFolder),
                        }
                    }
                    None => todo!(),
                },
            };
            write_build_gradle(gradle_content)?;

            out
        }
    }
}

fn write_build_gradle(gradle_content: String) -> Result<(), GradleFetchError> {
    match fs::write(GRADLE_FILE_PATH, gradle_content) {
        Err(e) => Err(GradleFetchError::CouldNotModifyBuildGradle(e)),
        Ok(_) => Ok(()),
    }
}

fn get_unpack_folder(stdout: &str) -> Option<&str> {
    let (_, spl) = stdout.split_once("STARTPATH_")?;
    let (path, _) = spl.split_once("\n")?;
    Some(path)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::get_unpack_folder;

    #[test]
    fn pa() {
        let input = "> Configure project :
STARTPATH_/home/emily/tmp/vanilla-gradle/build/unpacked-dependencies

> Task :unpackDependencies UP-TO-DATE

BUILD SUCCESSFUL in 710ms
1 actionable task: 1 up-to-date";
        let out = get_unpack_folder(input);
        assert_eq!(
            out,
            Some("/home/emily/tmp/vanilla-gradle/build/unpacked-dependencies")
        )
    }
}
