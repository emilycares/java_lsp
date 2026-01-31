#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
#[cfg(target_os = "windows")]
use std::sync::{Arc, mpsc};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{MAIN_SEPARATOR, Path, PathBuf},
};

use jwalk::WalkDir;
use my_string::MyString;
use parser::{
    SourceDestination,
    class::{self, ModuleInfo, load_class, load_module},
    dto::{Class, ClassError, ClassFolder},
    java::{self, ParseJavaError},
};
use rayon::iter::{ParallelBridge, ParallelIterator};
use rc_zip_tokio::{ReadZip, rc_zip::parse::EntryKind};
use std::fmt::Debug;
use tokio::fs::read;

pub const CFC_VERSION: usize = 0;

#[derive(Debug)]
pub enum LoaderError {
    IO(std::io::Error),
    Zip {
        e: rc_zip_tokio::rc_zip::error::Error,
        path: String,
    },
    InvalidCfcCache,
}

pub fn load_class_fs<T>(
    path: T,
    class_path: MyString,
    source: SourceDestination,
) -> Result<Class, ClassError>
where
    T: AsRef<Path> + Debug,
{
    let file = File::open(path).map_err(ClassError::IO)?;
    let mmap = unsafe { memmap2::Mmap::map(&file) }.map_err(ClassError::IO)?;
    class::load_class(&mmap[..], class_path, source)
}

pub fn load_java_fs<T>(path: T, source: SourceDestination) -> Result<Class, ParseJavaError>
where
    T: AsRef<Path> + Debug,
{
    let file = File::open(path).map_err(ParseJavaError::Io)?;
    let mmap = unsafe { memmap2::Mmap::map(&file) }.map_err(ParseJavaError::Io)?;
    #[cfg(unix)]
    mmap.advise(memmap2::Advice::Sequential)
        .map_err(ParseJavaError::Io)?;
    java::load_java(&mmap[..], source)
}

pub fn save_class_folder<P: AsRef<Path> + Debug>(
    path: P,
    class_folder: &ClassFolder,
) -> Result<(), LoaderError> {
    if class_folder.classes.is_empty() {
        return Ok(());
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(LoaderError::IO)?;
    let data = postcard::to_allocvec(class_folder).map_err(|_| LoaderError::InvalidCfcCache)?;

    let _ = file.write_all(&data);
    Ok(())
}

pub fn load_class_folder<P: AsRef<Path> + Debug>(path: P) -> Result<ClassFolder, LoaderError> {
    let file = File::open(&path).map_err(LoaderError::IO)?;
    let mmap = unsafe { memmap2::Mmap::map(&file) }.map_err(LoaderError::IO)?;
    if let Ok(o) = postcard::from_bytes::<ClassFolder>(&mmap[..]) {
        if o.version != CFC_VERSION {
            eprintln!("Detected old cfc cache: {path:?}");
            return Err(LoaderError::InvalidCfcCache);
        }
        Ok(o)
    } else {
        eprintln!("Detected invalid cfc cache: {path:?}");
        Err(LoaderError::InvalidCfcCache)
    }
}

pub fn get_java_files_from_folder<P: AsRef<Path>>(path: P) -> Vec<String> {
    get_files(&path, ".java")
}

#[cfg(not(target_os = "windows"))]
pub fn load_java_files(folder: PathBuf) -> Vec<Class> {
    WalkDir::new(folder)
        .into_iter()
        .par_bridge()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|i| i.eq_ignore_ascii_case("java"))
        })
        .filter_map(|e| {
            e.path()
                .to_str()
                .map(ToString::to_string)
                .map(|i| i.replace('\\', "/"))
        })
        .filter_map(
            |p| match load_java_fs(&p, SourceDestination::Here(p.clone())) {
                Ok(c) => Some(c),
                Err(e) => {
                    eprintln!("Unable to load java: {p}: {e:?}");
                    None
                }
            },
        )
        .collect::<Vec<_>>()
}
#[must_use]
#[cfg(target_os = "windows")]
pub fn load_java_files(dir: PathBuf) -> Vec<Class> {
    use std::time::Duration;

    let (tx, rx) = mpsc::channel();
    let _ = tx.send(dir);
    let tx = Arc::new(tx);
    let mut out = Vec::new();
    while let Ok(dir) = rx.recv_timeout(Duration::from_millis(3000)) {
        let tx = tx.clone();
        if let Ok(o) = visit_java_files(&dir, &tx, |p| {
            if let Some(s) = p.to_str() {
                return load_java_fs(p, SourceDestination::Here(s.to_owned())).ok();
            }
            None
        }) {
            out.extend(o);
        }
    }

    out
}
#[cfg(target_os = "windows")]
fn visit_java_files(
    dir: &PathBuf,
    tx: &Arc<mpsc::Sender<PathBuf>>,
    cb: impl Fn(&PathBuf) -> Option<Class>,
) -> Result<Vec<Class>, LoaderError> {
    let read_dir = std::fs::read_dir(dir)
        .map_err(LoaderError::IO)?
        .map(|res| res.map(|e| e.path()))
        .filter_map(Result::ok);
    let mut out: Vec<Class> = Vec::new();
    for entry in read_dir {
        if entry.is_dir() {
            let _ = tx.send(entry);
        } else if let Some(e) = entry.extension()
            && e == "java"
            && let Some(o) = cb(&entry)
        {
            out.push(o);
        }
    }
    Ok(out)
}

pub async fn load_classes_jar<P: AsRef<Path> + Debug + Clone>(
    path: P,
    source: SourceDestination,
) -> Result<ClassFolder, LoaderError> {
    let src_zip = format!("{path:?}");
    let buf = read(path).await.map_err(LoaderError::IO)?;

    base_load_classes_zip(src_zip, source, buf, None).await
}
pub async fn load_classes_jmod<P: AsRef<Path> + Debug>(
    path: P,
    source: SourceDestination,
) -> Result<ClassFolder, LoaderError> {
    let src_zip = format!("{path:?}");
    let mut buf = read(path).await.map_err(LoaderError::IO)?;
    buf.drain(0..4);

    base_load_classes_zip(src_zip, source, buf, Some("classes.")).await
}

async fn base_load_classes_zip(
    path: String,
    source: SourceDestination,
    buf: Vec<u8>,
    trim_prefix: Option<&str>,
) -> Result<ClassFolder, LoaderError> {
    let zip = buf.read_zip().await.map_err(|e| LoaderError::Zip {
        e,
        path: path.clone(),
    })?;
    let mut classes = vec![];

    // Prefix for module info
    let mut rules: Vec<(String, ModuleInfo)> = Vec::new();

    for entry in zip.entries() {
        if !matches!(entry.kind(), EntryKind::Directory)
            && let Some(file_name) = entry.sanitized_name()
            && file_name.ends_with("module-info.class")
        {
            let prefix = file_name.trim_end_matches("module-info.class");
            let buf = entry.bytes().await.map_err(LoaderError::IO)?;
            match load_module(buf.as_slice()) {
                Ok(c) => {
                    rules.push((prefix.to_string(), c));
                }
                Err(e) => {
                    eprintln!("Unable to load class: (in:{path}) {e:?}");
                }
            }
        }
    }
    'entries: for entry in zip.entries() {
        if matches!(entry.kind(), EntryKind::Directory) {
            continue;
        }
        let Some(file_name) = entry.sanitized_name() else {
            continue;
        };
        if !Path::new(file_name)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("class"))
        {
            continue;
        }
        if file_name.ends_with("module-info.class") {
            continue;
        }
        for r in &rules {
            let p = &file_name[8..];
            if file_name.starts_with(&r.0) && !r.1.exports.iter().any(|e| p.starts_with(e)) {
                continue 'entries;
            }
        }
        let class_path = file_name.trim_start_matches('/');
        let class_path = class_path.trim_end_matches(".class");
        let mut class_path = class_path.replace('/', ".");
        if let Some(trim_prefix) = trim_prefix {
            class_path = class_path.replace(trim_prefix, "");
        }

        let buf = entry.bytes().await.map_err(LoaderError::IO)?;

        match load_class(buf.as_slice(), class_path, source.clone()) {
            Ok(c) => classes.push(c),
            Err(e) => {
                eprintln!("Unable to load class: (in:{path}) {file_name} {e:?}");
            }
        }
    }

    Ok(ClassFolder {
        version: CFC_VERSION,
        classes,
    })
}

pub fn load_classes<P: AsRef<Path>>(path: P, source: &SourceDestination) -> ClassFolder {
    let Some(str_path) = &path.as_ref().to_str() else {
        eprintln!("load_classes failed could not make path into str");
        return ClassFolder::default();
    };
    ClassFolder {
        version: CFC_VERSION,
        classes: get_files(&path, ".class")
            .into_iter()
            .filter(|p| !p.ends_with("module-info.class"))
            .filter_map(|p| {
                let class_path = &p.trim_start_matches(str_path);
                let class_path = class_path.trim_start_matches(MAIN_SEPARATOR);
                let class_path = class_path.trim_end_matches(".class");
                let class_path = class_path.replace(MAIN_SEPARATOR, ".");
                match load_class_fs(p.as_str(), class_path, source.clone()) {
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
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
        .filter_map(|e| e.path().to_str().map(ToString::to_string))
        .filter(|e| e.ends_with(ending))
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use parser::dto::JType;

    #[test]
    fn ser() {
        let inp = JType::Void;
        let ser: Vec<u8> = postcard::to_allocvec(&inp).unwrap();
        let out: JType = postcard::from_bytes(ser.deref()).unwrap();

        assert_eq!(inp, out);
    }
}
