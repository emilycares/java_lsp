use std::{
    fs::{self, remove_file},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use common::TaskProgress;
use dashmap::DashMap;
use my_string::MyString;
use parser::{SourceDestination, dto::ClassFolder};
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
    NoGradlew(std::io::Error),
    GradlewNotExecutable,
}

#[cfg(not(target_os = "windows"))]
pub(crate) const PATH_GRADLE: &str = "./gradlew";
#[cfg(target_os = "windows")]
pub(crate) const PATH_GRADLE: &str = "./gradlew.bat";

const GRADLE_CFC: &str = ".gradle.cfc";

// https://github.com/gradle/gradle/issues/20460
// https://www.javathinking.com/blog/how-do-i-print-out-the-java-classpath-in-gradle/
// https://github.com/dansomething/gradle-classpath

pub async fn fetch_deps(
    class_map: &DashMap<MyString, parser::dto::Class>,
    build_gradle: PathBuf,
    sender: tokio::sync::watch::Sender<TaskProgress>,
) -> Result<(), GradleFetchError> {
    let path = Path::new(&GRADLE_CFC);
    if path.exists() {
        if let Ok(classes) = loader::load_class_folder(path) {
            for class in classes.classes {
                class_map.insert(class.class_path.clone(), class);
            }
            return Ok(());
        }
        remove_file(path).map_err(GradleFetchError::IO)?;
    }

    #[cfg(unix)]
    check_gradlew_executable_permission()?;

    let unpack_folder = copy_classpath(&build_gradle)?;
    let mut handles = JoinSet::<Option<ClassFolder>>::new();
    if let Ok(o) = fs::read_dir(unpack_folder) {
        let jars: Vec<PathBuf> = o
            .filter_map(Result::ok)
            .filter(|i| {
                if let Ok(ft) = i.file_type()
                    && ft.is_file()
                {
                    return true;
                }
                false
            })
            .filter(|i| i.file_name().to_string_lossy().ends_with(".jar"))
            .map(|i| i.path())
            .collect();
        let tasks_number = u32::try_from(jars.len() + 1).unwrap_or(1);
        let completed_number = Arc::new(AtomicU32::new(0));

        for jar in jars {
            if !jar.exists() {
                eprintln!("jar does not exist {}", jar.display());
                continue;
            }
            let completed_number = completed_number.clone();
            let sender = sender.clone();

            let current_name = jar.display().to_string();
            handles.spawn(async move {
                match loader::load_classes_jar(jar, SourceDestination::None).await {
                    Ok(classes) => {
                        let a = completed_number.fetch_add(1, Ordering::Relaxed);
                        let _ = sender.send(TaskProgress {
                            persentage: (100 * a) / (tasks_number + 1),
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
    if let Err(e) = loader::save_class_folder(GRADLE_CFC, &gradle_class_folder) {
        eprintln!("Failed to save {GRADLE_CFC} because: {e:?}");
    }
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
fn copy_classpath(build_gradle: &PathBuf) -> Result<PathBuf, GradleFetchError> {
    let script = if build_gradle.ends_with(".kts") {
        unimplemented!("No support yet for build.gradle.kts");
    } else {
        UNPACK_DEPENCENCIES_TASK_GROOVY
    };
    match fs::read_to_string(build_gradle) {
        Err(e) => Err(GradleFetchError::CouldNotReadBuildGradle(e)),
        Ok(gradle_content) => {
            if !gradle_content.contains(script) {
                write_build_gradle(build_gradle, format!("{gradle_content}\n{script}"))?;
            }
            let out = match Command::new(PATH_GRADLE).arg("classpath").output() {
                Err(e) => Err(GradleFetchError::GradlewExecFailed(e)),
                Ok(output) => match output.status.code() {
                    Some(1) => std::str::from_utf8(&output.stderr).map_or(
                        Err(GradleFetchError::StatusCodeErrMessageNotutf8),
                        |stderr| {
                            eprintln!("Got error from gradle: \n {stderr}");
                            Err(GradleFetchError::StatusCode)
                        },
                    ),
                    Some(_) => Ok(PathBuf::from("./build/classpath")),
                    None => todo!(),
                },
            };
            write_build_gradle(build_gradle, gradle_content)?;

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
        Ok(()) => Ok(()),
    }
}

/// When gradlew is not executable stop
#[cfg(unix)]
fn check_gradlew_executable_permission() -> Result<(), GradleFetchError> {
    use std::{fs::File, os::unix::fs::PermissionsExt};
    let f = File::open(PATH_GRADLE).map_err(GradleFetchError::NoGradlew)?;
    let meta = f.metadata().map_err(GradleFetchError::NoGradlew)?;
    let mode_exec = meta.permissions().mode() & 0o111 != 0;
    if !mode_exec {
        return Err(GradleFetchError::GradlewNotExecutable);
    }
    Ok(())
}
