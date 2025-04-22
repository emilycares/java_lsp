use std::{
    fs::{self},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use common::TaskProgress;
use dashmap::DashMap;
use parking_lot::Mutex;
use parser::{dto::ClassFolder, loader::SourceDestination};

use crate::tree::GradleTreeError;

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

const GRADLE_CFC: &str = ".gradle.cfc";

pub async fn fetch_deps(
    class_map: &DashMap<std::string::String, parser::dto::Class>,
    build_gradle: PathBuf,
    sender: tokio::sync::mpsc::Sender<TaskProgress>,
) -> Result<DashMap<std::string::String, parser::dto::Class>, GradleFetchError> {
    let path = Path::new(&GRADLE_CFC);
    if path.exists() {
        if let Ok(classes) = parser::loader::load_class_folder(path) {
            for class in classes.classes {
                class_map.insert(class.class_path.clone(), class);
            }
        }
        return Err(GradleFetchError::NoWorkToDo);
    } else {
        let unpack_folder = copy_classpath(build_gradle)?;
        let class_map = Arc::new(class_map.clone());
        let maven_class_folder = Arc::new(Mutex::new(ClassFolder::default()));
        let mut handles = Vec::new();
        if let Ok(o) = fs::read_dir(unpack_folder) {
            let jars: Vec<PathBuf> = o
                .filter_map(|i| i.ok())
                .filter(|i| {
                    if let Ok(ft) = i.file_type() {
                        if ft.is_file() {
                            return true;
                        }
                    }
                    return false;
                })
                .filter(|i| i.file_name().to_string_lossy().ends_with(".jar"))
                .map(|i| i.path())
                .collect();
            let tasks_number = jars.len();
            let completed_number = Arc::new(AtomicUsize::new(0));

            for jar in jars {
                if !jar.exists() {
                    eprintln!("jar does not exist {:?}", jar);
                    continue;
                }
                let class_map = class_map.clone();
                let gradle_class_folder = maven_class_folder.clone();
                let completed_number = completed_number.clone();
                let sender = sender.clone();

                let current_name = jar.display().to_string();
                handles.push(tokio::spawn(async move {
                    match parser::loader::load_classes_jar(jar, SourceDestination::None, None).await
                    {
                        Ok(classes) => {
                            let a = completed_number.fetch_add(1, Ordering::Release);
                            let _ = sender
                                .send(TaskProgress {
                                    persentage: (100 * a) / tasks_number,
                                    message: current_name,
                                })
                                .await;
                            {
                                let mut guard = gradle_class_folder.lock();
                                guard.append(classes.clone());
                            }
                            for class in classes.classes {
                                class_map.insert(class.class_path.clone(), class);
                            }
                        }
                        Err(e) => eprintln!("Error loading graddle jar {:?}", e),
                    }
                }));
            }
        }

        futures::future::join_all(handles).await;
        let guard = maven_class_folder.lock();
        if let Err(e) = parser::loader::save_class_folder(path, &guard) {
            eprintln!("Failed to save {GRADLE_CFC} because: {e}");
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
const UNPACK_DEPENCENCIES_TASK_GROOVY: &str = r#"
task classpath(type: Copy) {
    from configurations.runtimeClasspath
    from configurations.compileClasspath
    from configurations.testRuntimeClasspath
    from configurations.testCompileClasspath
    into "$buildDir/classpath"
}
"#;
fn copy_classpath(build_gradle: PathBuf) -> Result<PathBuf, GradleFetchError> {
    let script = match build_gradle.ends_with(".kts") {
        true => unimplemented!("No support yet for build.gradle.kts"),
        false => UNPACK_DEPENCENCIES_TASK_GROOVY,
    };
    match fs::read_to_string(&build_gradle) {
        Err(e) => Err(GradleFetchError::CouldNotReadBuildGradle(e)),
        Ok(gradle_content) => {
            if !gradle_content.contains(script) {
                write_build_gradle(&build_gradle, format!("{}\n{}", gradle_content, script))?;
            }
            let out = match Command::new(PATH_GRADLE).arg("classpath").output() {
                Err(e) => Err(GradleFetchError::GradlewExecFailed(e)),
                Ok(output) => match output.status.code() {
                    Some(1) => match std::str::from_utf8(&output.stderr) {
                        Ok(stderr) => {
                            eprintln!("Got error from gradle: \n {}", stderr);
                            Err(GradleFetchError::StatusCode)
                        }
                        Err(_) => Err(GradleFetchError::StatusCodeErrMessageNotutf8),
                    },
                    Some(_) => Ok(PathBuf::from("./build/classpath")),
                    None => todo!(),
                },
            };
            write_build_gradle(&build_gradle, gradle_content)?;

            out
        }
    }
}

fn write_build_gradle(
    build_gradle: &PathBuf,
    gradle_content: String,
) -> Result<(), GradleFetchError> {
    match fs::write(build_gradle, gradle_content) {
        Err(e) => Err(GradleFetchError::CouldNotModifyBuildGradle(e)),
        Ok(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

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
