use std::{
    fs::{self, remove_file},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use common::TaskProgress;
use dashmap::DashMap;
use parser::{dto::ClassFolder, loader::SourceDestination};
use tokio::task::JoinSet;

use crate::tree::GradleTreeError;

#[derive(Debug)]
pub enum GradleFetchError {
    Tree(GradleTreeError),
    CouldNotReadBuildGradle(std::io::Error),
    CouldNotModifyBuildGradle(std::io::Error),
    GradlewExecFailed(std::io::Error),
    CouldNotGetUnpackFolder,
    StatusCode,
    StatusCodeErrMessageNotutf8,
    IO(std::io::Error),
}

#[cfg(not(target_os = "windows"))]
pub(crate) const PATH_GRADLE: &str = "./gradlew";
#[cfg(target_os = "windows")]
pub(crate) const PATH_GRADLE: &str = "./gradlew.bat";

const GRADLE_CFC: &str = ".gradle.cfc";

pub async fn fetch_deps(
    class_map: &DashMap<std::string::String, parser::dto::Class>,
    build_gradle: PathBuf,
    sender: tokio::sync::watch::Sender<TaskProgress>,
) -> Result<(), GradleFetchError> {
    let path = Path::new(&GRADLE_CFC);
    if path.exists() {
        if let Ok(classes) = parser::loader::load_class_folder(path) {
            for class in classes.classes {
                class_map.insert(class.class_path.clone(), class);
            }
            return Ok(());
        } else {
            remove_file(path).map_err(GradleFetchError::IO)?
        }
    }

    let unpack_folder = copy_classpath(build_gradle)?;
    let mut handles = JoinSet::<Option<ClassFolder>>::new();
    if let Ok(o) = fs::read_dir(unpack_folder) {
        let jars: Vec<PathBuf> = o
            .filter_map(|i| i.ok())
            .filter(|i| {
                if let Ok(ft) = i.file_type() {
                    if ft.is_file() {
                        return true;
                    }
                }
                false
            })
            .filter(|i| i.file_name().to_string_lossy().ends_with(".jar"))
            .map(|i| i.path())
            .collect();
        let tasks_number = jars.len();
        let completed_number = Arc::new(AtomicUsize::new(0));

        for jar in jars {
            if !jar.exists() {
                eprintln!("jar does not exist {jar:?}");
                continue;
            }
            let completed_number = completed_number.clone();
            let sender = sender.clone();

            let current_name = jar.display().to_string();
            handles.spawn(async move {
                match parser::loader::load_classes_jar(jar, SourceDestination::None, None).await {
                    Ok(classes) => {
                        let a = completed_number.fetch_add(1, Ordering::Relaxed);
                        let _ = sender.send(TaskProgress {
                            persentage: (100 * a) / tasks_number,
                            error: false,
                            message: current_name,
                        });
                        Some(classes)
                    }
                    Err(e) => {
                        eprintln!("Error loading graddle jar {e:?}");
                        None
                    }
                }
            });
        }
    }

    let done = handles.join_all().await;

    let gradle_class_folder = ClassFolder {
        classes: done.into_iter().flatten().flat_map(|i| i.classes).collect(),
    };
    if let Err(e) = parser::loader::save_class_folder(GRADLE_CFC, &gradle_class_folder) {
        eprintln!("Failed to save {GRADLE_CFC} because: {e:?}");
    };
    for class in gradle_class_folder.classes {
        class_map.insert(class.class_path.clone(), class);
    }
    Ok(())
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
