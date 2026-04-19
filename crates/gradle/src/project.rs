use common::TaskProgress;
use dto::{CFC_VERSION, Class, ClassFolder, SourceDestination};
use my_string::MyString;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::str::{Utf8Error, from_utf8};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::task::JoinSet;

#[must_use]
pub fn load_project_folders(project_dir: &Path) -> Vec<Class> {
    let mut out = vec![];

    out.extend(loader::load_java_files(project_dir.join("src/main/java")));
    out.extend(loader::load_java_files(project_dir.join("src/test/java")));

    out
}

#[must_use]
pub fn get_gradle_cache_path(project_dir: &Path, project_cache_dir: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    project_dir.hash(&mut hasher);
    let s = format!("{}.gradle.cfc", hasher.finish());
    project_cache_dir.join(s)
}

pub fn ensure_init_script() -> Result<PathBuf, GradleProjectError> {
    let mut file = common::cache_dir();
    file = file.join("gradle_init_script");
    file.set_extension("gradle");

    std::fs::write(
        &file,
        br#"
allprojects {
    tasks.register("printLspClasspath") {
        doLast {        
            project.sourceSets.each { ss ->
                ss.compileClasspath.each {
                    println("JAVA_LSP_CLASSPATH:" + it.absolutePath)
                }
                ss.runtimeClasspath.each {
                    println("JAVA_LSP_CLASSPATH:" + it.absolutePath)
                }
            }
        }
    }
}"#,
    )
    .map_err(GradleProjectError::IO)?;

    Ok(file)
}

#[derive(Debug)]
pub struct InitScriptOutput {
    class_path: HashSet<String>,
}

pub fn run_init_script(executable_gradle: String) -> Result<InitScriptOutput, GradleProjectError> {
    let script = ensure_init_script()?;
    let out = Command::new(executable_gradle)
        .arg("--init-script")
        .arg(script)
        .arg("printLspClasspath")
        .output()
        .map_err(GradleProjectError::IO)?;

    let content = from_utf8(&out.stdout).map_err(GradleProjectError::Utf8)?;
    let mut class_path = HashSet::new();
    for p in content.lines() {
        if p.starts_with("JAVA_LSP_CLASSPATH:") {
            let value = p.trim_start_matches("JAVA_LSP_CLASSPATH:").to_string();
            if std::path::Path::new(&value)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"))
            {
                class_path.insert(value);
            }
        }
    }
    Ok(InitScriptOutput { class_path })
}

#[derive(Debug)]
pub enum GradleProjectError {
    IO(std::io::Error),
    Utf8(Utf8Error),
}

pub async fn index_project(
    class_map: Arc<Mutex<HashMap<MyString, Class, impl std::hash::BuildHasher>>>,
    sender: tokio::sync::watch::Sender<TaskProgress>,
    use_cache: bool,
    cache_path: PathBuf,
    executable_gradle: String,
) {
    match index(class_map, sender, use_cache, cache_path, executable_gradle).await {
        Ok(()) => (),
        Err(e) => {
            eprintln!("Got error while loading gradle project: {e:?}");
        }
    }
}

async fn index(
    class_map: Arc<Mutex<HashMap<MyString, Class, impl std::hash::BuildHasher>>>,
    sender: tokio::sync::watch::Sender<TaskProgress>,
    use_cache: bool,
    cache_path: PathBuf,
    executable_gradle: String,
) -> Result<(), GradleProjectError> {
    if use_cache
        && cache_path.exists()
        && let Ok(classes) = loader::load_class_folder(&cache_path)
    {
        if let Ok(mut cm) = class_map.lock() {
            for class in classes.classes {
                cm.insert(class.class_path.clone(), class);
            }
        }
        return Ok(());
    }
    let mut handles = JoinSet::<Option<ClassFolder>>::new();

    let inits = run_init_script(executable_gradle)?;
    let tasks_number = u32::try_from(inits.class_path.len()).unwrap_or(1);
    let completed_number = Arc::new(AtomicU32::new(0));

    for p in inits.class_path {
        let sender = sender.clone();
        let completed_number = completed_number.clone();
        handles.spawn(async move {
            match loader::load_classes_jar(&p, SourceDestination::None).await {
                Ok(classes) => {
                    let a = completed_number.fetch_add(1, Ordering::Relaxed);
                    let message = p.clone();
                    let _ = sender.send(TaskProgress {
                        percentage: (100 * a) / tasks_number,
                        error: false,
                        message,
                    });
                    Some(classes)
                }
                Err(e) => {
                    eprintln!("Failed to load jar: {p}, {e:?}");
                    None
                }
            }
        });
    }

    let done = handles.join_all().await;

    let class_folder = ClassFolder {
        classes: done.into_iter().flatten().flat_map(|i| i.classes).collect(),
        version: CFC_VERSION,
    };

    if let Err(e) = loader::save_class_folder(&cache_path, &class_folder) {
        eprintln!("Failed to save {} because: {e:?}", cache_path.display());
    }
    if let Ok(mut cm) = class_map.lock() {
        for class in class_folder.classes {
            cm.insert(class.class_path.clone(), class);
        }
    }
    Ok(())
}
