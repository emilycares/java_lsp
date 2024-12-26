use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use crate::{
    class,
    dto::{self},
    java,
};
use std::fmt::Debug;
use walkdir::WalkDir;

pub fn load_class_fs<T>(path: T, class_path: String, source: String) -> Result<dto::Class, dto::ClassError>
where
    T: AsRef<Path> + Debug,
{
    let bytes = std::fs::read(path)?;
    class::load_class(&bytes, class_path, source)
}

pub fn load_java_fs<T>(path: T, class_path: String, source: String) -> Result<dto::Class, dto::ClassError>
where
    T: AsRef<Path> + Debug,
{
    let bytes = std::fs::read(path)?;
    java::load_java(&bytes, class_path, source)
}

pub fn save_class_folder(
    prefix: &str,
    class_folder: &dto::ClassFolder,
) -> Result<(), dto::ClassError> {
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

pub fn load_classes<P: AsRef<Path>>(path: P, source: String) -> dto::ClassFolder {
    let Some(str_path) = &path.as_ref().to_str() else {
        eprintln!("load_classes failed could not make path into str");
        return dto::ClassFolder::new();
    };
    dto::ClassFolder {
        classes: get_classes(&path)
            .into_iter()
            .filter_map(|p| {
                let class_path = &p.trim_start_matches(str_path);
                let class_path = class_path.trim_start_matches("/");
                let class_path = class_path.trim_end_matches(".class");
                let class_path = class_path.replace("/", ".");
                match load_class_fs(p.as_str(), class_path.to_string(), source.clone()) {
                    Ok(c) => Some(c),
                    Err(e) => {
                        dbg!("Unable to load class: {}: {}", p, e);
                        None
                    }
                }
            })
            .collect(),
    }
}

fn get_classes<P: AsRef<Path>>(dir: P) -> Vec<String> {
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
        .filter(|e| e.ends_with(".class"))
        .collect::<Vec<_>>()
}
