use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{MAIN_SEPARATOR, Path},
};

use crate::{
    class::{self, load_class},
    dto::{self, ClassError, ClassFolder},
    java::{self, ParseJavaError},
};
use async_zip::base::read::seek::ZipFileReader;
use std::fmt::Debug;
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};
use walkdir::WalkDir;

#[derive(Debug)]
pub enum ParserLoaderError {
    IO(std::io::Error),
    Zip(async_zip::error::ZipError),
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

pub fn load_java_files(paths: Vec<String>) -> dto::ClassFolder {
    dto::ClassFolder {
        classes: paths
            .into_iter()
            .filter_map(|p| {
                match load_java_fs(p.as_str(), SourceDestination::Here(p.as_str().to_string())) {
                    Ok(c) => Some(c),
                    Err(e) => {
                        eprintln!("Unable to load java: {p}: {e:?}");
                        None
                    }
                }
            })
            .collect(),
    }
}

pub async fn load_classes_jar<P: AsRef<Path>>(
    path: P,
    source: SourceDestination,
    skip_bytes_start: Option<usize>,
) -> Result<dto::ClassFolder, ParserLoaderError> {
    let file = File::open(path).await.map_err(ParserLoaderError::IO)?;
    let mut reader = BufReader::new(file);
    if let Some(k) = skip_bytes_start {
        let mut header = vec![0; k];
        reader
            .read_exact(&mut header)
            .await
            .map_err(ParserLoaderError::SkipBytesStart)?;
    }
    let mut zip = ZipFileReader::with_tokio(&mut reader)
        .await
        .map_err(ParserLoaderError::Zip)?;
    let mut classes = vec![];

    for index in 0..zip.file().entries().len() {
        let file = match zip.file().entries().get(index) {
            Some(f) => f,
            None => continue,
        };
        if file.dir().map_err(ParserLoaderError::Zip)? {
            continue;
        }
        let file_name = file.filename();
        let file_name = file_name.as_str().map_err(ParserLoaderError::Zip)?;
        let file_name = file_name.to_string();
        if !file_name.ends_with(".class") {
            continue;
        }
        if file_name.starts_with("module-info.class") {
            continue;
        }
        if file_name.starts_with("META-INF") {
            continue;
        }

        let class_path = file_name.trim_start_matches("/");
        let class_path = class_path.trim_end_matches(".class");
        let class_path = class_path.replace("/", ".");
        let mut entry_reader = zip
            .reader_with_entry(index)
            .await
            .map_err(ParserLoaderError::Zip)?;
        let mut buf = vec![];
        if let Err(e) = entry_reader.read_to_end_checked(&mut buf).await {
            eprintln!("Unable to read file in zip: {file_name} {e:?}");
            continue;
        }
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
        .filter(|a| a.is_ok())
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
        .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
        .filter(|e| e.ends_with(ending))
        .collect::<Vec<_>>()
}
