use common::deps::{deps_base, deps_get_source};
use common::{Dependency, TaskProgress, deps_dir};
use dto::{Class, ClassFolder, SourceDestination};
use maven::m2::{self, pom_m2, pom_sources_jar};
use maven::update::{CurlClient, pom_source_jar_url};
use my_string::MyString;
use my_string::smol_str::ToSmolStr;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::str::{Utf8Error, from_utf8};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
    sync::{Arc, RwLock},
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
            let value = p
                .trim_start_matches("JAVA_LSP_CLASSPATH:")
                .replace('\\', "/");
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
    MTwo(m2::MTwoError),
}

pub async fn index_project(
    class_map: Arc<RwLock<HashMap<MyString, Class, impl std::hash::BuildHasher + Send + Sync>>>,
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
    class_map: Arc<RwLock<HashMap<MyString, Class, impl std::hash::BuildHasher + Send + Sync>>>,
    sender: tokio::sync::watch::Sender<TaskProgress>,
    use_cache: bool,
    cache_path: PathBuf,
    executable_gradle: String,
) -> Result<(), GradleProjectError> {
    if use_cache
        && cache_path.exists()
        && let Ok(classes) = loader::load_class_folder(&cache_path)
    {
        if let Ok(mut cm) = class_map.write() {
            for class in classes.classes {
                cm.insert(class.class_path.clone(), class);
            }
        }
        return Ok(());
    }
    let mut handles = JoinSet::<Option<ClassFolder>>::new();
    let deps_path = deps_dir();
    let deps_path = deps_path.as_path();

    let inits = run_init_script(executable_gradle)?;
    let tasks_number = u32::try_from(inits.class_path.len()).unwrap_or(1);
    let completed_number = Arc::new(AtomicU32::new(0));
    let m2 = m2::get_maven_m2_folder().map_err(GradleProjectError::MTwo)?;
    let client = Arc::new(CurlClient::new());
    let repo = Arc::new(maven::repository::central());

    for p in inits.class_path {
        let sender = sender.clone();
        let completed_number = completed_number.clone();
        let source = get_dependency(&p).map_or(SourceDestination::None, |dep| {
            let deps_base = Arc::new(deps_base(&dep, deps_path));
            let deps_source = Arc::new(deps_get_source(&deps_base));
            let pom_mtwo = Arc::new(pom_m2(&dep, &m2));
            let source_file = Arc::new(pom_sources_jar(&dep, &pom_mtwo));
            let source_url = pom_source_jar_url(&dep, &repo.url);

            {
                let deps_source = deps_source.clone();
                let client = client.clone();
                let repo = repo.clone();
                handles.spawn(async move {
                    maven::update::fetch_extract_source(
                        source_file,
                        pom_mtwo,
                        deps_source,
                        deps_base,
                        client,
                        repo,
                        &source_url,
                    )
                    .await;
                    None
                });
            }
            deps_source
                .to_str()
                .map_or(SourceDestination::None, |source| {
                    SourceDestination::RelativeInFolder(source.to_smolstr())
                })
        });

        handles.spawn(async move {
            match loader::load_classes_jar(&p, source).await {
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
    };

    if let Err(e) = loader::save_class_folder(&cache_path, &class_folder) {
        eprintln!("Failed to save {} because: {e:?}", cache_path.display());
    }
    if let Ok(mut cm) = class_map.write() {
        for class in class_folder.classes {
            cm.insert(class.class_path.clone(), class);
        }
    }
    Ok(())
}

fn get_dependency(p: &str) -> Option<Dependency> {
    let (_, p) = p.split_once(".gradle/caches/modules-2/files-2.1/")?;
    let mut spl = p.splitn(4, '/');
    let group_id = spl.next()?;
    let artivact_id = spl.next()?;
    let version = spl.next()?;
    Some(Dependency {
        group_id: group_id.to_string(),
        artivact_id: artivact_id.to_string(),
        version: version.to_string(),
        version_suffix: None,
    })
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;

    #[test]
    fn dep_from_path() {
        let path = "/home/emily/.gradle/caches/modules-2/files-2.1/org.junit.jupiter/junit-jupiter-api/5.7.1/a7261dff44e64aea7f621842eac5977fd6d2412d/junit-jupiter-api-5.7.1.jar";
        let out = get_dependency(path);
        let expected = expect![[r#"
            Some(
                Dependency {
                    group_id: "org.junit.jupiter",
                    artivact_id: "junit-jupiter-api",
                    version: "5.7.1",
                    version_suffix: None,
                },
            )
        "#]];
        expected.assert_debug_eq(&out);
    }
}
