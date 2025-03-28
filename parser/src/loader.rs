use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, MAIN_SEPARATOR},
};

use crate::{
    class,
    dto::{self},
    java::{self, ParseJavaError},
};
use std::fmt::Debug;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub enum SourceDestination {
    Here(String),
    RelativeInFolder(String),
    None,
}

pub fn load_class_fs<T>(
    path: T,
    class_path: String,
    source: SourceDestination,
) -> Result<dto::Class, dto::ClassError>
where
    T: AsRef<Path> + Debug,
{
    let bytes = std::fs::read(path)?;
    class::load_class(&bytes, class_path, source)
}

pub fn load_java_fs<T>(path: T, source: SourceDestination) -> Result<dto::Class, ParseJavaError>
where
    T: AsRef<Path> + Debug,
{
    let bytes = std::fs::read(path).map_err(|e| ParseJavaError::Io(e))?;
    java::load_java(&bytes, source)
}

pub fn save_class_folder(
    prefix: &str,
    class_folder: &dto::ClassFolder,
) -> Result<(), dto::ClassError> {
    if class_folder.classes.is_empty() {
        return Ok(());
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(format!(".{}.cfc", prefix))?;
    let data = postcard::to_allocvec(class_folder)?;

    let _ = file.write_all(&data);
    Ok(())
}

pub fn load_class_folder(prefix: &str) -> Result<dto::ClassFolder, dto::ClassError> {
    let data = fs::read(Path::new(&format!(".{}.cfc", prefix)))?;
    let out = postcard::from_bytes(&data)?;

    Ok(out)
}

pub fn get_java_files_from_folder<P: AsRef<Path>>(path: P) -> Vec<String> {
    get_files(&path, ".java")
}

pub fn load_java_files(paths: Vec<String>) -> dto::ClassFolder {
    dto::ClassFolder {
        classes: paths
            .into_iter()
            .filter_map(|p| {
                match load_java_fs(p.as_str(), SourceDestination::Here(p.as_str().to_string())) {
                    Ok(c) => Some(c),
                    Err(e) => {
                        eprintln!("Unable to load java: {}: {:?}", p, e);
                        None
                    }
                }
            })
            .collect(),
    }
}

pub fn load_classes<P: AsRef<Path>>(path: P, source: SourceDestination) -> dto::ClassFolder {
    let Some(str_path) = &path.as_ref().to_str() else {
        eprintln!("load_classes failed could not make path into str");
        return dto::ClassFolder::default();
    };
    dto::ClassFolder {
        classes: get_files(&path, ".class")
            .into_iter()
            .filter(|p| !p.ends_with("module-info.class"))
            .filter_map(|p| {
                let class_path = &p.trim_start_matches(str_path);
                let class_path = class_path.trim_start_matches(MAIN_SEPARATOR);
                let class_path = class_path.trim_end_matches(".class");
                let class_path = class_path.replace(MAIN_SEPARATOR, ".");
                match load_class_fs(p.as_str(), class_path.to_string(), source.clone()) {
                    Ok(c) => Some(c),
                    Err(e) => {
                        eprintln!("Unable to load class: {}: {}", p, e);
                        None
                    }
                }
            })
            .collect(),
    }
}

fn get_files<P: AsRef<Path>>(dir: P, ending: &str) -> Vec<String> {
    WalkDir::new(dir)
        .into_iter()
        .filter(|a| {
            if let Err(e) = a {
                dbg!(e);
                return false;
            }
            true
        })
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
        .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
        .filter(|e| e.ends_with(ending))
        .collect::<Vec<_>>()
}
