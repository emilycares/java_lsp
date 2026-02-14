#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::implicit_hasher)]
use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs::{self},
    path::{Path, PathBuf},
    str::{Utf8Error, from_utf8},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
};

use common::TaskProgress;
use loader::{CFC_VERSION, load_class_files};
use my_string::MyString;
use parser::{
    SourceDestination,
    dto::{Class, ClassFolder},
};
use tokio::{process::Command, task::JoinSet};

#[cfg(not(target_os = "windows"))]
const EXECUTABLE_JAVA: &str = "java";
#[cfg(target_os = "windows")]
const EXECUTABLE_JAVA: &str = "java.exe";

const JDK_CFC: &str = "jdk.cfc";

#[derive(Debug)]
pub enum JdkError {
    NoSrcZip,
    Unzip(Option<String>),
    ParserLoader(loader::LoaderError),
    JavaVersionCommand(std::io::Error),
    JimageExtractCommand(std::io::Error),
    JimageGotError(String),
    WorkDir,
    JavaVersionNoLine,
    IO(std::io::Error),
    ZipUtil(zip_util::ZipUtilError),
    JavaNotInPath,
    Utf8(Utf8Error),
    Str,
}

pub async fn load_classes(
    class_map: Arc<Mutex<HashMap<MyString, Class>>>,
    sender: tokio::sync::watch::Sender<TaskProgress>,
    path: &OsString,
) -> Result<(), JdkError> {
    let (java_path, op_dir) = get_work_dirs(path).await?;
    let cache_path = op_dir.join(JDK_CFC);

    if cache_path.exists()
        && let Ok(classes) = loader::load_class_folder(&cache_path)
    {
        if let Ok(mut cm) = class_map.lock() {
            for class in classes.classes {
                cm.insert(class.class_path.clone(), class);
            }
        }
        return Ok(());
    }
    let class_folder = load_jdk(&java_path, &op_dir, false, sender).await?;
    if let Err(e) = loader::save_class_folder(cache_path, &class_folder) {
        eprintln!("Failed to save {JDK_CFC} because: {e:?}");
    }
    if let Ok(mut cm) = class_map.lock() {
        for class in class_folder.classes {
            cm.insert(class.class_path.clone(), class);
        }
    }
    Ok(())
}

fn java_executable_location(path: &OsString) -> Option<PathBuf> {
    env::split_paths(path).find_map(|dir| {
        let full_path = dir.join(EXECUTABLE_JAVA);
        if full_path.is_file() {
            Some(full_path)
        } else {
            None
        }
    })
}

/// Extracts java jdk from from the java executable in path.
/// returns folder of output
pub async fn load_jdk(
    java_path: &Path,
    op_dir: &Path,
    ignore_jmod: bool,
    sender: tokio::sync::watch::Sender<TaskProgress>,
) -> Result<ClassFolder, JdkError> {
    extract_source_zip(java_path, op_dir).await?;
    if !ignore_jmod {
        let jmods_dir = get_jmods_dir(java_path);
        if jmods_dir.exists() {
            let mut jmod_executable = java_path.to_path_buf();
            jmod_executable.push("jmod");
            if cfg!(windows) {
                jmod_executable.set_extension("exe");
            }
            if jmod_executable.exists() {
                return load_jmods(jmods_dir, op_dir, sender).await;
            }
        }
    }
    let modules_file = get_modules_file(java_path);
    if modules_file.exists() {
        let mut jimage_executable = java_path.to_path_buf();
        jimage_executable.push("jimage");
        if cfg!(windows) {
            jimage_executable.set_extension("exe");
        }
        if jimage_executable.exists() {
            return load_modules(jimage_executable, modules_file, op_dir, sender).await;
        }
    }
    eprintln!("There is no jmod in your jdk: {}", java_path.display());
    load_old(java_path, op_dir).await
}

async fn load_modules(
    jimage_executable: PathBuf,
    modules_file: PathBuf,
    op_dir: &Path,
    _sender: tokio::sync::watch::Sender<TaskProgress>,
) -> Result<ClassFolder, JdkError> {
    let source_dir = op_dir.join("src");
    let source_dir = source_dir.to_str().ok_or(JdkError::Str)?;

    let out = op_dir.join("modules");
    if !fs::exists(&out).unwrap_or(false) {
        let _ = fs::create_dir_all(&out);
        let out_param = format!("--dir={}", out.display());
        // jimage extract --dir=/tmp/moduels/ lib/modules
        let output = Command::new(jimage_executable)
            .arg("extract")
            .arg(out_param)
            .arg(modules_file)
            .output()
            .await
            .map_err(JdkError::JimageExtractCommand)?;
        if !output.status.success() {
            let err = from_utf8(&output.stderr).map_err(JdkError::Utf8)?;
            return Err(JdkError::JimageGotError(err.to_owned()));
        }
    }

    let classes = load_class_files(&out, 3, true, source_dir);

    Ok(ClassFolder {
        version: CFC_VERSION,
        classes,
    })
}

fn get_modules_file(java_path: &Path) -> PathBuf {
    let mut java_path = java_path.to_path_buf();
    java_path.pop();
    let mut modules_file: PathBuf = java_path.join("lib").join("modules");
    if !modules_file.exists() {
        let lib_openjdk_lib_modules = java_path
            .join("lib")
            .join("openjdk")
            .join("lib")
            .join("modules");
        if lib_openjdk_lib_modules.exists() {
            modules_file = lib_openjdk_lib_modules;
        }
    }
    modules_file
}

async fn load_old(java_path: &Path, op_dir: &Path) -> Result<ClassFolder, JdkError> {
    let mut java_path = java_path.to_path_buf();
    java_path.pop();
    let jre_lib = java_path.join("jre").join("lib");

    let source_dir = op_dir.join("src");
    let mut rt_jar = jre_lib.join("rt");
    rt_jar.set_extension("jar");
    let mut classes = loader::load_classes_jar(
        &rt_jar,
        SourceDestination::RelativeInFolder(
            source_dir
                .to_str()
                .expect("Should be represented as string")
                .into(),
        ),
    )
    .await
    .map_err(JdkError::ParserLoader)?;

    load_javafx(java_path, op_dir, jre_lib, &mut classes).await?;
    Ok(classes)
}

async fn load_javafx(
    path: PathBuf,
    op_dir: &Path,
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
            let classes_jfx = loader::load_classes_jar(
                jfxrt,
                SourceDestination::RelativeInFolder(
                    source_dir_jfx
                        .to_str()
                        .expect("Should be represented as string")
                        .into(),
                ),
            )
            .await
            .map_err(JdkError::ParserLoader)?;
            classes.append(classes_jfx);
        }
    }
    Ok(())
}

async fn load_jmods(
    jmods: PathBuf,
    op_dir: &Path,
    sender: tokio::sync::watch::Sender<TaskProgress>,
) -> Result<ClassFolder, JdkError> {
    let source_dir = op_dir.join("src");
    let mut handles = JoinSet::<Result<ClassFolder, JdkError>>::new();
    let source_dir = Arc::new(source_dir);
    let completed_number = Arc::new(AtomicU32::new(0));
    let sender = Arc::new(sender);

    match fs::read_dir(&jmods) {
        Err(e) => eprintln!("error reading dir: {:?} {e:?}", &jmods.to_str()),
        Ok(jmods) => {
            let mut tasks_number: u32 = 1;
            for jmod in jmods {
                let sender = sender.clone();
                let completed_number = completed_number.clone();
                let source_dir = source_dir.clone();
                if let Ok(jmod) = jmod {
                    if let Ok(ft) = jmod.file_type()
                        && !ft.is_file()
                    {
                        continue;
                    }
                    let jmod = jmod.path();
                    if let Some(jmod_name) = jmod.file_name().and_then(|n| n.to_str()) {
                        let jmod_display = jmod_name.trim_end_matches(".jmod").to_owned();

                        handles.spawn(async move {
                            let relative_source = source_dir.join(&jmod_display);
                            let classes = loader::load_classes_jmod(
                                jmod,
                                SourceDestination::RelativeInFolder(
                                    relative_source
                                        .to_str()
                                        .expect("Should be represented as string")
                                        .into(),
                                ),
                            )
                            .await
                            .map_err(JdkError::ParserLoader);
                            let a = completed_number.fetch_add(1, Ordering::Relaxed);
                            let _ = sender.send(TaskProgress {
                                percentage: (100 * a) / tasks_number,
                                error: false,
                                message: format!("Loaded classes of jmod: {jmod_display}"),
                            });

                            classes
                        });
                    }
                }
                tasks_number += 1;
            }
        }
    }

    let done = handles.join_all().await;
    let mut classes = vec![];

    for r in done {
        match r {
            Ok(c) => classes.extend(c.classes),
            Err(e) => return Err(e),
        }
    }

    Ok(ClassFolder {
        classes,
        version: CFC_VERSION,
    })
}

fn get_jmods_dir(path: &Path) -> PathBuf {
    let mut path = path.to_path_buf();
    path.pop();
    let mut jmods = path.join("jmods");
    if !jmods.exists() {
        let lib_openjdk_jmods = path.join("lib").join("openjdk").join("jmods");
        if lib_openjdk_jmods.exists() {
            jmods = lib_openjdk_jmods;
        }
    }
    jmods
}

async fn extract_source_zip(path: &Path, op_dir: &Path) -> Result<(), JdkError> {
    let mut src_zip = path.to_path_buf();
    src_zip.pop();
    src_zip = src_zip.join("lib").join("src");
    src_zip.set_extension("zip");
    match std::fs::canonicalize(src_zip) {
        Ok(src_zip) => {
            let src_dir = op_dir.join("src");
            unzip_to_dir(&src_dir, &src_zip).await?;
        }
        Err(e) => {
            eprintln!("Unable to extract jdk src.zip, (Broken link?) {e:?}");
        }
    }

    Ok(())
}

async fn unzip_to_dir(dir: &Path, zip: &PathBuf) -> Result<(), JdkError> {
    if !std::fs::exists(zip).unwrap_or(false) {
        return Err(JdkError::Unzip(zip.to_str().map(ToOwned::to_owned)));
    }
    if !dir.exists() {
        zip_util::extract_jar(zip, dir)
            .await
            .map_err(JdkError::ZipUtil)?;
    }
    Ok(())
}

/// Returns java path and opdir
pub async fn get_work_dirs(path: &OsString) -> Result<(PathBuf, PathBuf), JdkError> {
    let java_path = java_executable_location(path).ok_or(JdkError::JavaNotInPath)?;
    let mut java_path = std::fs::canonicalize(java_path).map_err(JdkError::IO)?;
    let version = get_java_version(&java_path).await?;
    java_path.pop();
    let mut java_folder = java_path.clone();
    java_folder.pop();
    if let Some(java_folder_name) = java_folder.file_name()
        && let Some(java_folder_name) = java_folder_name.to_str()
    {
        let jdk_name = format!("{java_folder_name}_{version}");
        let op_dir = opdir(&jdk_name);
        return Ok((java_path, op_dir));
    }
    Err(JdkError::WorkDir)
}

async fn get_java_version(java_path: &PathBuf) -> Result<String, JdkError> {
    let command_output = Command::new(java_path)
        .arg("-version")
        .output()
        .await
        .map_err(JdkError::JavaVersionCommand)?;
    let stderr = std::str::from_utf8(&command_output.stderr).map_err(JdkError::Utf8)?;
    let mut lines = stderr.lines();
    let Some(line) = lines.next() else {
        return Err(JdkError::JavaVersionNoLine);
    };
    Ok(line.replace('\"', "").replace(' ', "_"))
}

fn opdir(jdk_name: &str) -> PathBuf {
    let mut op_dir = dirs::cache_dir().expect("There should be a cache dir");
    op_dir = op_dir.join("java_lsp").join("java");
    op_dir = op_dir.join(jdk_name);
    let _ = fs::create_dir_all(&op_dir);
    op_dir
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn load_jdk_modules_integration() {
        let Some(path) = std::env::var_os("PATH") else {
            return;
        };
        let (java_path, op_dir) = get_work_dirs(&path).await.unwrap();
        let (sender, _) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress::default());
        let out = load_jdk(&java_path, &op_dir, true, sender).await.unwrap();

        let string = out.classes.iter().find(|i| i.name == "String");
        assert!(string.is_some());
        assert_eq!(string.unwrap().class_path, "java.lang.String");
        let source = &string.unwrap().source;
        assert!(source.ends_with("src/java.base/java/lang/String.java"));
        assert!(fs::exists(source).unwrap());
    }

    #[tokio::test]
    async fn load_jdk_jmod_integration() {
        let Some(path) = std::env::var_os("PATH") else {
            return;
        };
        let (java_path, op_dir) = get_work_dirs(&path).await.unwrap();
        let (sender, _) = tokio::sync::watch::channel::<TaskProgress>(TaskProgress::default());
        let out = load_jdk(&java_path, &op_dir, false, sender).await.unwrap();

        let string = out.classes.iter().find(|i| i.name == "String");
        assert!(string.is_some());
        assert_eq!(string.unwrap().class_path, "java.lang.String");
        let source = &string.unwrap().source;
        assert!(
            source
                .replace("\\", "/")
                .ends_with("src/java.base/java/lang/String.java")
        );
        assert!(fs::exists(source).unwrap());
    }
}
