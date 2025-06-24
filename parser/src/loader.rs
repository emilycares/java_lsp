use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{MAIN_SEPARATOR, Path, PathBuf},
};

use crate::{
    class::{self, load_class},
    dto::{self, Class, ClassError, ClassFolder},
    java::{self, ParseJavaError},
};
use jwalk::WalkDir;
use rayon::iter::{ParallelBridge, ParallelIterator};
use rc_zip_tokio::{ReadZip, rc_zip::parse::EntryKind};
use std::fmt::Debug;
use tokio::fs::read;

#[derive(Debug)]
pub enum ParserLoaderError {
    IO(std::io::Error),
    Zip(rc_zip_tokio::rc_zip::error::Error),
    SkipBytesStart(std::io::Error),
}

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
    let bytes = std::fs::read(path).map_err(ClassError::IO)?;
    class::load_class(&bytes, class_path, source)
}

pub fn load_java_fs<T>(path: T, source: SourceDestination) -> Result<dto::Class, ParseJavaError>
where
    T: AsRef<Path> + Debug,
{
    let bytes = std::fs::read(path).map_err(ParseJavaError::Io)?;
    java::load_java(&bytes, source)
}

pub fn save_class_folder<P: AsRef<Path>>(
    path: P,
    class_folder: &dto::ClassFolder,
) -> Result<(), dto::ClassError> {
    if class_folder.classes.is_empty() {
        return Ok(());
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(ClassError::IO)?;
    let data = postcard::to_allocvec(class_folder).map_err(ClassError::Postcard)?;

    let _ = file.write_all(&data);
    Ok(())
}

pub fn load_class_folder<P: AsRef<Path>>(path: P) -> Result<dto::ClassFolder, dto::ClassError> {
    let data = fs::read(path).map_err(ClassError::IO)?;
    let out = postcard::from_bytes(&data).map_err(ClassError::Postcard)?;

    Ok(out)
}

pub fn get_java_files_from_folder<P: AsRef<Path>>(path: P) -> Vec<String> {
    get_files(&path, ".java")
}

pub async fn load_java_files(folder: PathBuf) -> Vec<Class> {
    WalkDir::new(folder)
        .into_iter()
        .par_bridge()
        .filter_map(|a| a.ok())
        .filter(|e| !e.file_type().is_dir())
        .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
        .filter(|e| e.ends_with(".java"))
        .filter_map(|p| {
            match load_java_fs(p.as_str(), SourceDestination::Here(p.as_str().to_string())) {
                Ok(c) => Some(c),
                Err(e) => {
                    eprintln!("Unable to load java: {p}: {e:?}");
                    None
                }
            }
        })
        .collect::<Vec<_>>()
}

pub async fn load_classes_jar<P: AsRef<Path>>(
    path: P,
    source: SourceDestination,
    _skip_bytes_start: Option<usize>,
) -> Result<dto::ClassFolder, ParserLoaderError> {
    let buf = read(path).await.map_err(ParserLoaderError::IO)?;
    let zip = buf.read_zip().await.map_err(ParserLoaderError::Zip)?;
    let mut classes = vec![];

    for entry in zip.entries() {
        if matches!(entry.kind(), EntryKind::Directory) {
            continue;
        }
        let Some(file_name) = entry.sanitized_name().map(|i| i.to_string()) else {
            continue;
        };
        if !file_name.ends_with(".class") {
            continue;
        }
        if file_name.starts_with("module-info.class") {
            continue;
        }

        let class_path = file_name.trim_start_matches("/");
        let class_path = class_path.trim_end_matches(".class");
        let class_path = class_path.replace("/", ".");

        let buf = entry.bytes().await.map_err(ParserLoaderError::IO)?;

        match load_class(buf.as_slice(), class_path.to_string(), source.clone()) {
            Ok(c) => classes.push(c),
            Err(e) => {
                eprintln!("Unable to load class: {file_name} {e:?}");
            }
        }
    }

    Ok(ClassFolder { classes })
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
                        eprintln!("Unable to load class: {p}: {e:?}");
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
        .par_bridge()
        .filter_map(|a| a.ok())
        .filter(|e| !e.file_type().is_dir())
        .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
        .filter(|e| e.ends_with(ending))
        .collect::<Vec<_>>()
}
