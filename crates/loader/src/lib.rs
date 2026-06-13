#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::collections::VecDeque;
use std::fs::read;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use class::{ModuleInfo, load_class, load_module};
use dto::{Class, ClassFolder, ClassParserError, SourceDestination};
pub use dto_rw::DtoRwError;
use my_string::smol_str::SmolStr;
#[cfg(windows)]
use my_string::smol_str::StrExt;
use my_string::{MyString, smol_str::ToSmolStr};
use parser::java::{self, ParseJavaError};
use rc_zip_tokio::{ReadZip, rc_zip::parse::EntryKind};
use std::fmt::Debug;

pub const DEBUGGING: bool = false;

#[derive(Debug)]
pub enum LoaderError {
    IO(std::io::Error),
    Zip {
        e: rc_zip_tokio::rc_zip::error::Error,
        path: String,
    },
    EmptyClassFolder,
    Module(ClassParserError),
    ClassParser(ClassParserError),
    ParseJava(ParseJavaError),
    DtoRw(DtoRwError),
}

pub fn load_java_fs<T>(path: T, source: SourceDestination) -> Result<Class, LoaderError>
where
    T: AsRef<Path> + Debug,
{
    let buf = read(path).map_err(LoaderError::IO)?;
    java::load_java(&buf, source).map_err(LoaderError::ParseJava)
}

pub fn load_class_fs<T>(
    path: T,
    source: SourceDestination,
    class_path: MyString,
    filter: bool,
) -> Result<Class, LoaderError>
where
    T: AsRef<Path> + Debug,
{
    let buf = read(path).map_err(LoaderError::IO)?;
    class::load_class(&buf, class_path, source, filter).map_err(LoaderError::ClassParser)
}

pub fn save_class_folder<P: AsRef<Path> + Debug>(
    path: P,
    class_folder: &ClassFolder,
) -> Result<(), LoaderError> {
    if class_folder.classes.is_empty() {
        return Err(LoaderError::EmptyClassFolder);
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(LoaderError::IO)?;
    let data = dto_rw::write(class_folder);

    file.write_all(&data).map_err(LoaderError::IO)?;
    file.flush().map_err(LoaderError::IO)?;
    Ok(())
}

pub fn load_class_folder<P: AsRef<Path> + Debug>(path: P) -> Result<ClassFolder, LoaderError> {
    if DEBUGGING {
        return Err(LoaderError::DtoRw(DtoRwError::InvalidCfcCache));
    }
    let file = File::open(&path).map_err(LoaderError::IO)?;
    let mmap = unsafe { memmap2::Mmap::map(&file) }.map_err(LoaderError::IO)?;
    dto_rw::parse(&mmap[..]).map_err(LoaderError::DtoRw)
}

#[must_use]
pub fn load_java_files(dir: PathBuf) -> Vec<Class> {
    let mut dirs = VecDeque::new();
    dirs.push_back(dir);
    let mut out = Vec::new();
    while let Some(dir) = dirs.pop_front() {
        if let Ok(o) = visit_java_files(&dir, &mut dirs, |p| {
            if let Some(s) = p.to_str() {
                return load_java_fs(p, SourceDestination::Here(s.to_smolstr())).ok();
            }
            None
        }) {
            out.extend(o);
        }
    }
    out
}
fn visit_java_files(
    dir: &PathBuf,
    dirs: &mut VecDeque<PathBuf>,
    cb: impl Fn(&PathBuf) -> Option<Class>,
) -> Result<Vec<Class>, LoaderError> {
    let read_dir = std::fs::read_dir(dir)
        .map_err(LoaderError::IO)?
        .map(|res| res.map(|e| e.path()))
        .filter_map(Result::ok);
    let mut out: Vec<Class> = Vec::new();
    for entry in read_dir {
        if entry.is_dir() {
            dirs.push_back(entry);
        } else if let Some(e) = entry.extension()
            && e == "java"
            && let Some(o) = cb(&entry)
        {
            out.push(o);
        }
    }
    Ok(out)
}

fn visit_class_files(
    dir: &PathBuf,
    dirs: &mut VecDeque<PathBuf>,
) -> Result<Vec<SmolStr>, LoaderError> {
    let read_dir = std::fs::read_dir(dir)
        .map_err(LoaderError::IO)?
        .map(|res| res.map(|e| e.path()))
        .filter_map(Result::ok);
    let mut out: Vec<SmolStr> = Vec::new();
    for entry in read_dir {
        if entry.is_dir() {
            dirs.push_back(entry);
        } else if let Some(e) = entry.extension()
            && e == "class"
            && let Some(p) = entry.to_str()
        {
            #[cfg(windows)]
            let p = p.replace_smolstr("\\", "/");
            #[cfg(not(windows))]
            let p = p.to_smolstr();
            out.push(p);
        }
    }
    Ok(out)
}

pub fn load_class_files(
    folder: &Path,
    trim_prefix: usize,
    filter: bool,
    source: &str,
) -> Result<Vec<Class>, LoaderError> {
    use my_string::smol_str::ToSmolStr;
    let mut dirs = VecDeque::new();
    dirs.push_back(folder.to_path_buf());

    let Some(root_prefix) = folder.to_str() else {
        return Ok(vec![]);
    };

    #[cfg(windows)]
    let root_prefix = root_prefix.replace('\\', "/");
    #[cfg(windows)]
    let root_prefix = root_prefix.as_str();

    let mut files = Vec::new();
    while let Some(dir) = dirs.pop_front() {
        if let Ok(o) = visit_class_files(&dir, &mut dirs) {
            files.extend(o);
        }
    }

    // Prefix for module info
    let mut rules: Vec<(String, ModuleInfo)> = Vec::new();

    for p in files.iter().filter(|i| i.ends_with("module-info.class")) {
        let prefix = p
            .trim_start_matches(root_prefix)
            .trim_start_matches('/')
            .trim_end_matches("module-info.class");
        if let Ok(file) = File::open(p).map_err(LoaderError::IO)
            && let Ok(mmap) = unsafe { memmap2::Mmap::map(&file) }
        {
            #[cfg(unix)]
            let _ = mmap.advise(memmap2::Advice::Sequential);
            match load_module(&mmap) {
                Ok(c) => {
                    rules.push((prefix.to_string(), c));
                }
                Err(e) => {
                    return Err(LoaderError::Module(e));
                }
            }
        }
    }

    let mut out = Vec::new();

    'outer: for p in files.iter().filter(|i| !i.ends_with("module-info.class")) {
        use my_string::smol_str::{StrExt, format_smolstr};

        let prefix = p.trim_start_matches(root_prefix).trim_start_matches('/');

        for r in &rules {
            if prefix.starts_with(&r.0) && !r.1.exports.iter().any(|e| p.contains(e.as_str())) {
                continue 'outer;
            }
        }
        let mut class_path = prefix.trim_end_matches(".class").replace_smolstr("/", ".");
        if trim_prefix > 1 {
            let spl = class_path.splitn(trim_prefix, '.');
            if let Some(p) = spl.last() {
                class_path = p.to_smolstr();
            }
        }
        let sr = p
            .trim_start_matches(root_prefix)
            .replacen_smolstr(".class", ".java", 1);

        let smol_str = format_smolstr!("{source}{sr}");
        match load_class_fs(
            p,
            SourceDestination::Here(smol_str.clone()),
            class_path,
            filter,
        ) {
            Ok(c) => {
                out.push(c);
            }
            Err(LoaderError::ClassParser(
                ClassParserError::NotAClass | ClassParserError::Ignoring,
            )) => (),
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(out)
}
pub async fn load_classes_jar<P: AsRef<Path> + Debug + Clone>(
    path: P,
    source: SourceDestination,
) -> Result<ClassFolder, LoaderError> {
    let src_zip = format!("{path:?}");
    let buf = read(path).map_err(LoaderError::IO)?;

    base_load_classes_zip(src_zip, source, buf, None).await
}
pub async fn load_classes_jmod<P: AsRef<Path> + Debug>(
    path: P,
    source: SourceDestination,
) -> Result<ClassFolder, LoaderError> {
    let src_zip = format!("{path:?}");
    let mut buf = read(path).map_err(LoaderError::IO)?;
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
                    return Err(LoaderError::Module(e));
                }
            }
        }
    }
    let trim_prefix_path = trim_prefix.map(|i| i.replace('.', "/"));
    'entries: for entry in zip.entries() {
        if matches!(entry.kind(), EntryKind::Directory) {
            continue;
        }
        let Some(file_name) = entry.sanitized_name() else {
            continue;
        };
        let ext = Path::new(file_name).extension();
        if ext.is_some_and(|e| e.eq_ignore_ascii_case("jar")) {
            let buf = entry.bytes().await.map_err(LoaderError::IO)?;
            let o = Box::pin(base_load_classes_zip(
                file_name.to_string(),
                SourceDestination::None,
                buf,
                None,
            ))
            .await?;
            classes.extend(o.classes);
            continue;
        }
        if !ext.is_some_and(|e| e.eq_ignore_ascii_case("class")) {
            continue;
        }
        if file_name.ends_with("module-info.class") {
            continue;
        }
        for r in &rules {
            let p = trim_prefix_path
                .as_ref()
                .map_or(file_name, |prefix| file_name.trim_start_matches(prefix));
            if file_name.starts_with(&r.0) && !r.1.exports.iter().any(|e| p.starts_with(e.as_str()))
            {
                continue 'entries;
            }
        }
        let class_path = file_name.trim_start_matches('/');
        let class_path = class_path.trim_end_matches(".class");
        let mut class_path = class_path.replace('/', ".").to_smolstr();
        if let Some(trim_prefix) = trim_prefix {
            class_path = class_path.replace(trim_prefix, "").to_smolstr();
        }

        let buf = entry.bytes().await.map_err(LoaderError::IO)?;

        match load_class(buf.as_slice(), class_path.clone(), source.clone(), true) {
            Ok(c) => classes.push(c),
            Err(ClassParserError::Ignoring | ClassParserError::NotAClass) => (),
            Err(e) => {
                return Err(LoaderError::ClassParser(e));
            }
        }
    }

    Ok(ClassFolder { classes })
}

#[cfg(test)]
mod tests {
    use crate::DEBUGGING;

    #[test]
    fn not_debugging() {
        const {
            assert!(!DEBUGGING);
        }
    }
}
