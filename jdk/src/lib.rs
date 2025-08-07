use std::{
    env,
    fs::{self, remove_file},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use common::TaskProgress;
use dashmap::DashMap;
use futures::{AsyncBufReadExt, StreamExt};
use parser::{dto::ClassFolder, loader::SourceDestination};
use smol_str::SmolStr;
use tokio::{process::Command, task::JoinSet};

#[cfg(not(target_os = "windows"))]
const EXECUTABLE_JAVA: &str = "java";
#[cfg(target_os = "windows")]
const EXECUTABLE_JAVA: &str = "java.exe";

const JDK_CFC: &str = ".jdk.cfc";

#[derive(Debug)]
pub enum JdkError {
    NoSrcZip,
    Unzip(Option<String>),
    ParserLoader(parser::loader::ParserLoaderError),
    JavaVersionCommand(std::io::Error),
    WorkDir,
    JavaVersionNoLine,
    IO(std::io::Error),
    ZipUtil(zip_util::ZipUtilError),
}

pub async fn load_classes(
    class_map: &DashMap<SmolStr, parser::dto::Class>,
    sender: tokio::sync::watch::Sender<TaskProgress>,
) -> Result<(), JdkError> {
    let (java_path, op_dir) = get_work_dirs().await?;
    let cache_path = op_dir.join(JDK_CFC);

    if cache_path.exists() {
        if let Ok(classes) = parser::loader::load_class_folder(&cache_path) {
            for class in classes.classes {
                class_map.insert(class.class_path.clone(), class);
            }
            return Ok(());
        } else {
            remove_file(&cache_path).map_err(JdkError::IO)?
        }
    }
    // nix run github:nix-community/nix-index#nix-locate -- jmods/java.base.jmod
    // ``` bash
    // # jmod is in the jdk bin dir
    // jmod extract openjdk-22.0.2_windows-x64_bin/jdk-22.0.2/jmods/java.base.jmod
    // cd ..
    // mvn dependency:unpack
    // ```
    let class_folder = load_jdk(java_path, op_dir, sender).await?;
    if let Err(e) = parser::loader::save_class_folder(cache_path, &class_folder) {
        eprintln!("Failed to save {JDK_CFC} because: {e:?}");
    };
    for class in class_folder.classes {
        class_map.insert(class.class_path.clone(), class);
    }
    Ok(())
}

fn java_executable_location() -> Option<PathBuf> {
    if let Some(paths) = env::var_os("PATH") {
        return env::split_paths(&paths)
            .filter_map(|dir| {
                let full_path = dir.join(EXECUTABLE_JAVA);
                if full_path.is_file() {
                    Some(full_path)
                } else {
                    None
                }
            })
            .next();
    }
    None
}

/// Extracts java jdk from from the java executabel in path.
/// returns folder of output
async fn load_jdk(
    java_path: PathBuf,
    op_dir: PathBuf,
    sender: tokio::sync::watch::Sender<TaskProgress>,
) -> Result<ClassFolder, JdkError> {
    let mut jmod_executable = java_path.clone();
    jmod_executable.push("jmod");
    if cfg!(windows) {
        jmod_executable.set_extension("exe");
    }
    if jmod_executable.exists() {
        return load_jmods(java_path, op_dir, jmod_executable, sender).await;
    }
    eprintln!("There is no jmod in your jdk: {java_path:?}");
    load_old(java_path, op_dir).await
}

async fn load_old(mut path: PathBuf, op_dir: PathBuf) -> Result<ClassFolder, JdkError> {
    path.pop();
    let jre_lib = path.join("jre").join("lib");

    let source_dir = op_dir.join("src");
    let mut src_zip = path.join("src");
    src_zip.set_extension("zip");
    unzip_to_dir(&source_dir, &src_zip).await?;
    let mut rt_jar = jre_lib.join("rt");
    rt_jar.set_extension("jar");
    let mut classes = parser::loader::load_classes_jar(
        &rt_jar,
        SourceDestination::RelativeInFolder(
            source_dir
                .to_str()
                .expect("Should be represented as string")
                .into(),
        ),
        None,
    )
    .await
    .map_err(JdkError::ParserLoader)?;

    load_javafx(path, op_dir, jre_lib, &mut classes).await?;
    Ok(classes)
}

async fn load_javafx(
    path: PathBuf,
    op_dir: PathBuf,
    jre_lib: PathBuf,
    classes: &mut ClassFolder,
) -> Result<(), JdkError> {
    let source_dir_jfx = op_dir.join("src_jfx");
    let mut src_zip_jfx = path.join("javafx-src");
    src_zip_jfx.set_extension("zip");
    if src_zip_jfx.exists() {
        unzip_to_dir(&source_dir_jfx, &src_zip_jfx).await?;
        let mut jfxrt = jre_lib.join("ext").join("jfxrt");
        jfxrt.set_extension("jar");
        if jfxrt.exists() {
            let classes_jfx = parser::loader::load_classes_jar(
                jfxrt,
                SourceDestination::RelativeInFolder(
                    source_dir_jfx
                        .to_str()
                        .expect("Should be represented as string")
                        .into(),
                ),
                None,
            )
            .await
            .map_err(JdkError::ParserLoader)?;
            classes.append(classes_jfx);
        }
    }
    Ok(())
}

async fn load_jmods(
    mut path: PathBuf,
    op_dir: PathBuf,
    jmod_executable: PathBuf,
    sender: tokio::sync::watch::Sender<TaskProgress>,
) -> Result<ClassFolder, JdkError> {
    path.pop();

    let source_dir = op_dir.join("src");
    let mut src_zip = path.clone();
    src_zip = src_zip.join("lib").join("src");
    src_zip.set_extension("zip");
    unzip_to_dir(&source_dir, &src_zip).await?;

    let mut jmods = path.join("jmods");
    if !jmods.exists() {
        let lib_openjdk_jmods = path.join("lib").join("openjdk").join("jmods");
        if lib_openjdk_jmods.exists() {
            jmods = lib_openjdk_jmods;
        }
    }

    let jmods_dir = op_dir.join("jmods");
    let _ = fs::create_dir_all(&jmods_dir);

    let mut handles = JoinSet::<Option<ClassFolder>>::new();
    let source_dir = Arc::new(source_dir);
    let jmod_executable = Arc::new(jmod_executable);
    let jmods_dir = Arc::new(jmods_dir);
    let completed_number = Arc::new(AtomicUsize::new(0));
    let sender = Arc::new(sender);

    match fs::read_dir(jmods) {
        Err(e) => eprintln!("error reading dir {e:?}"),
        Ok(jmods) => {
            for (tasks_number, jmod) in jmods.enumerate() {
                let sender = sender.clone();
                let completed_number = completed_number.clone();
                let source_dir = source_dir.clone();
                let jmod_executable = jmod_executable.clone();
                let jmods_dir = jmods_dir.clone();
                if let Ok(jmod) = jmod {
                    if let Ok(ft) = jmod.file_type() {
                        if !ft.is_file() {
                            continue;
                        }
                    }
                    let jmod = jmod.path();
                    if let Some(jmod_name) = jmod
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.to_string())
                    {
                        let jmod_display = jmod_name.trim_end_matches(".jmod").to_owned();

                        handles.spawn(async move {
                            let jmod_dir = &jmods_dir.join(&jmod_display);
                            if !jmod_dir.exists() {
                                let _ = sender.send(TaskProgress {
                                    persentage: (100 * completed_number.load(Ordering::Relaxed))
                                        / tasks_number,
                                    error: false,
                                    message: format!("Extract classes of jmod: {}", jmod_display),
                                });
                                let _ = fs::create_dir_all(jmod_dir);
                                match Command::new(&*jmod_executable)
                                    .current_dir(jmod_dir)
                                    .arg("extract")
                                    .arg(&jmod)
                                    .output()
                                    .await
                                {
                                    Ok(_r) => {
                                        let _ = sender.send(TaskProgress {
                                            persentage: (100
                                                * completed_number.load(Ordering::Relaxed))
                                                / tasks_number,
                                            error: false,
                                            message: format!(
                                                "Extracted classes of jmod: {}",
                                                jmod_display
                                            ),
                                        });
                                    }
                                    Err(e) => eprintln!("Error with jmod extraction {e:?}"),
                                };
                            }
                            let classes_folder = jmod_dir.join("classes");
                            let relative_source = source_dir.join(&jmod_display);
                            let classes = parser::loader::load_classes(
                                &classes_folder,
                                SourceDestination::RelativeInFolder(
                                    relative_source
                                        .to_str()
                                        .expect("Should be represented as string")
                                        .into(),
                                ),
                            );
                            let a = completed_number.fetch_add(1, Ordering::Relaxed);
                            let _ = sender.send(TaskProgress {
                                persentage: (100 * a) / tasks_number,
                                error: false,
                                message: format!("Loaded classes of jmod: {}", jmod_display),
                            });
                            Some(classes)
                        });
                    }
                }
            }
        }
    }

    let done = handles.join_all().await;

    let class_folder = ClassFolder {
        classes: done.into_iter().flatten().flat_map(|i| i.classes).collect(),
    };

    Ok(class_folder)
}

async fn unzip_to_dir(dir: &Path, zip: &PathBuf) -> Result<(), JdkError> {
    if !zip.exists() {
        return Err(JdkError::Unzip(zip.to_str().map(|i| i.to_owned())));
    }
    if !dir.exists() {
        zip_util::extract_jar(zip, dir)
            .await
            .map_err(JdkError::ZipUtil)?;
    }
    Ok(())
}

/// Returns java path and opdir
async fn get_work_dirs() -> Result<(PathBuf, PathBuf), JdkError> {
    let mut java_path =
        java_executable_location().expect("There should be a java executabel in path");
    if java_path.is_symlink() {
        if let Ok(linked) = fs::read_link(&java_path) {
            java_path = linked;
        }
    }
    let version = get_java_version(&java_path).await?;
    java_path.pop();
    let mut java_folder = java_path.clone();
    java_folder.pop();
    if let Some(java_folder_name) = java_folder.file_name() {
        if let Some(java_folder_name) = java_folder_name.to_str() {
            let jdk_name = format!("{}_{}", java_folder_name, version);
            let op_dir = opdir(jdk_name);

            return Ok((java_path, op_dir));
        }
    }
    Err(JdkError::WorkDir)
}

async fn get_java_version(java_path: &PathBuf) -> Result<String, JdkError> {
    let command_output = Command::new(java_path)
        .arg("-version")
        .output()
        .await
        .map_err(JdkError::JavaVersionCommand)?;
    let mut lines = command_output.stderr.lines();
    let line = lines.next();
    let Some(line) = line.await.take() else {
        return Err(JdkError::JavaVersionNoLine);
    };
    let Ok(line) = line else {
        return Err(JdkError::JavaVersionNoLine);
    };
    Ok(line.replace('\"', "").replace(' ', "_"))
}

fn opdir(jdk_name: String) -> PathBuf {
    let mut op_dir = dirs::cache_dir().expect("There should be a cache dir");
    op_dir = op_dir.join("java_lsp").join("java");
    op_dir = op_dir.join(jdk_name);
    let _ = fs::create_dir_all(&op_dir);
    op_dir
}
