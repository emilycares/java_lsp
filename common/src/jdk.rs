use std::{
    env,
    fs::{self},
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};

use dashmap::DashMap;
use parser::{dto::ClassFolder, loader::SourceDestination};
use tokio::{process::Command, sync::Mutex};

#[cfg(target_os = "linux")]
const EXECUTABLE_JAVA: &str = "java";
#[cfg(target_os = "windows")]
const EXECUTABLE_JAVA: &str = "java.exe";

#[derive(Debug)]
pub enum JdkError {
    NoSrcZip,
    Unzip(Option<String>),
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
async fn load_jdk() -> Result<ClassFolder, JdkError> {
    let mut path = java_executable_location().expect("There should be a java executabel in path");
    eprintln!("java executable location {:?}", &path);
    if path.is_symlink() {
        if let Ok(linked) = fs::read_link(&path) {
            eprintln!("sym: {:?}", linked);
            path = linked;
        }
    }
    path.pop();

    let mut jmod_executable = path.clone();
    jmod_executable.push("jmod");
    if cfg!(windows) {
        jmod_executable.set_extension("exe");
    }
    eprintln!("jmod_executable: {:?}", &jmod_executable);
    if jmod_executable.exists() {
        return load_jmods(path, jmod_executable).await;
    }
    eprintln!("There is no jmod in your jdk: {:?}", &path);
    load_old(path)
}

fn load_old(mut path: PathBuf) -> Result<ClassFolder, JdkError> {
    path.pop();
    let jdk_name = path.file_name();

    let op_dir = opdir(jdk_name);
    let source_dir = op_dir.join("src");
    let mut src_zip = path.join("src");
    src_zip.set_extension("zip");
    unzip_to_dir(&source_dir, &src_zip)?;

    let classes_dir = op_dir.join("classes");

    let mut rt_jar = path.join("jre").join("lib").join("rt");
    rt_jar.set_extension("jar");
    unzip_to_dir(&classes_dir, &rt_jar)?;
    let classes = parser::loader::load_classes(
        &classes_dir,
        SourceDestination::RelativeInFolder(
            source_dir
                .to_str()
                .expect("Should be represented as string")
                .to_owned(),
        ),
    );
    Ok(classes)
}

async fn load_jmods(mut path: PathBuf, jmod_executable: PathBuf) -> Result<ClassFolder, JdkError> {
    path.pop();
    let binding = path.clone();
    let jdk_name = binding.file_name();

    let op_dir = opdir(jdk_name);

    let source_dir = op_dir.join("src");
    eprintln!("Unpacking jdk source into: {:?}", &source_dir);
    let mut src_zip = path.clone();
    src_zip = src_zip.join("lib").join("src");
    src_zip.set_extension("zip");
    unzip_to_dir(&source_dir, &src_zip)?;

    let mut jmods = path.join("jmods");
    if !jmods.exists() {
        let lib_openjdk_jmods = path.join("lib").join("openjdk").join("jmods");
        if lib_openjdk_jmods.exists() {
            jmods = lib_openjdk_jmods;
        }
    }
    eprintln!("jmods folder: {:?}", &src_zip);

    let jmods_dir = op_dir.join("jmods");
    let _ = fs::create_dir_all(&jmods_dir);
    eprintln!("Unpacking jmod classes into: {:?}", &jmods_dir);

    let mut handles = Vec::new();
    let class_folder = Arc::new(Mutex::new(ClassFolder::default()));
    let source_dir = Arc::new(source_dir);
    let jmod_executable = Arc::new(jmod_executable);
    let jmods_dir = Arc::new(jmods_dir);

    eprintln!("jmods {:?}", &jmods);
    match fs::read_dir(jmods) {
        Err(e) => eprintln!("error reading dir {:?}", e),
        Ok(jmods) => {
            for jmod in jmods {
                let class_folder = class_folder.clone();
                let source_dir = source_dir.clone();
                let jmod_executable = jmod_executable.clone();
                let jmods_dir = jmods_dir.clone();
                if let Ok(jmod) = jmod {
                    eprintln!("jmod there");
                    if let Ok(ft) = jmod.file_type() {
                        if !ft.is_file() {
                            continue;
                        }
                    }
                    let jmod = jmod.path();
                    eprintln!("jmod isfile {:?}", &jmod);
                    if let Some(jmod_name) = jmod
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.to_string())
                    {
                        eprintln!("jmod found jmod_name: {}", &jmod_name);
                        let jmod_display = jmod_name.trim_end_matches(".jmod").to_owned();

                        handles.push(tokio::spawn(async move {
                            let jmod_dir = &jmods_dir.join(&jmod_display);
                            let _ = fs::create_dir_all(&jmod_dir);
                            match Command::new(&*jmod_executable)
                                .current_dir(&jmod_dir)
                                .arg("extract")
                                .arg(&jmod)
                                .output()
                                .await
                            {
                                Ok(_r) => {
                                    eprintln!("Extracted jdk jmod: {}", &jmod_display);
                                    let classes_folder = jmod_dir.join("classes");
                                    let relative_source = source_dir.join(jmod_display);
                                    let _ = fs::create_dir_all(&relative_source);
                                    let classes = parser::loader::load_classes(
                                        &classes_folder,
                                        SourceDestination::RelativeInFolder(
                                            relative_source
                                                .to_str()
                                                .expect("Should be represented as string")
                                                .to_owned(),
                                        ),
                                    );
                                    {
                                        let mut guard = class_folder.lock().await;
                                        guard.append(classes);
                                    }
                                }
                                Err(e) => eprintln!("Error with jmod extraction {:?}", e),
                            }
                        }));
                    }
                }
            }
        }
    }

    futures::future::join_all(handles).await;
    let class_folder = class_folder.clone();
    let guard = class_folder.lock().await;
    Ok(guard.clone())
}

fn unzip_to_dir(dir: &PathBuf, zip: &PathBuf) -> Result<(), JdkError> {
    if !zip.exists() {
        return Err(JdkError::Unzip(zip.to_str().map(|i| i.to_owned())));
    }
    if !dir.exists() {
        let _ = fs::create_dir_all(dir);
        if let Ok(data) = fs::read(zip) {
            let res = zip_extract::extract(Cursor::new(data), dir, false);
            if let Err(e) = res {
                eprintln!("Unable to unzip: {:?}, {e}", &zip);
            }
        }
    }
    Ok(())
}

fn opdir(jdk_name: Option<&std::ffi::OsStr>) -> PathBuf {
    let mut op_dir = dirs::cache_dir().expect("There should be a cache dir");
    op_dir = op_dir.join("java_lsp").join("java");
    if let Some(java) = jdk_name {
        op_dir = op_dir.join(java);
    }
    let _ = fs::create_dir_all(&op_dir);
    op_dir
}

pub async fn load_classes(
    class_map: &DashMap<std::string::String, parser::dto::Class>,
) -> Result<(), JdkError> {
    let path = Path::new(".jdk.cfc");
    if path.exists() {
        if let Ok(classes) = parser::loader::load_class_folder("jdk") {
            for class in classes.classes {
                class_map.insert(class.class_path.clone(), class);
            }
        }
    } else {
        // nix run github:nix-community/nix-index#nix-locate -- jmods/java.base.jmod
        // ``` bash
        // # jmod is in the jdk bin dir
        // jmod extract openjdk-22.0.2_windows-x64_bin/jdk-22.0.2/jmods/java.base.jmod
        // cd ..
        // mvn dependency:unpack
        // ```
        let class_folder = load_jdk().await?;
        if let Err(e) = parser::loader::save_class_folder("jdk", &class_folder) {
            eprintln!("Failed to save .jdk.cfc because: {e}");
        };
        for class in class_folder.classes {
            class_map.insert(class.class_path.clone(), class);
        }
    }
    Ok(())
}
