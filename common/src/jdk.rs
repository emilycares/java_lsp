use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use dashmap::DashMap;
use parser::loader::SourceDestination;

#[cfg(target_os = "linux")]
const EXECUTABLE_JAVA: &str = "java";
#[cfg(target_os = "windows")]
const EXECUTABLE_JAVA: &str = "java.exe";

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
fn extract_jmod() -> PathBuf {
    let mut path = java_executable_location().expect("There should be a java executabel in path");
    path.pop();

    let mut jmod_executable = path.clone();
    jmod_executable.push("jmod");
    if cfg!(windows) {
        jmod_executable.set_extension("exe");
    }

    path.pop();
    let binding = path.clone();
    let jdk_name = binding.file_name();
    path = path.join("jmods").join("java");
    path.set_extension("base.jmod");

    let mut op_dir = dirs::cache_dir().expect("There should be a cache dir");
    op_dir = op_dir.join("java_lsp").join("java");
    if let Some(java) = jdk_name {
        op_dir = op_dir.join(java);
    }
    let _ = fs::create_dir_all(&op_dir);
    match Command::new(jmod_executable)
        .current_dir(&op_dir)
        .arg("extract")
        .arg(&path)
        .output()
    {
        Ok(_r) => eprintln!("Extracted {:?} into {:?}", &path, &op_dir),
        Err(e) => eprintln!("Error with jmod extraction {:?}", e),
    }
    op_dir
}

pub fn load_classes(class_map: &DashMap<std::string::String, parser::dto::Class>) {
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
        // mkdir jdk
        // cd jdk
        // # jmod is in the jdk bin dir
        // jmod extract openjdk-22.0.2_windows-x64_bin/jdk-22.0.2/jmods/java.base.jmod
        // cd ..
        // mvn dependency:unpack
        // ```
        let mut extract_folder = extract_jmod();
        extract_folder = extract_folder.join("classes");
        let classes = parser::loader::load_classes(
            &extract_folder,
            SourceDestination::RelativeInFolder(
                extract_folder
                    .to_str()
                    .expect("Folder should be represented in string")
                    .to_string(),
            ),
        );
        if let Err(e) = parser::loader::save_class_folder("jdk", &classes) {
            eprintln!("Failed to save .jdk.cfc because: {e}");
        };
        for class in classes.classes {
            class_map.insert(class.class_path.clone(), class);
        }
    }
}
