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

pub fn load_class_fs<T>(path: T, class_path: String) -> Result<dto::Class, dto::ClassError>
where
    T: AsRef<Path> + Debug,
{
    let bytes = std::fs::read(path)?;
    class::load_class(&bytes, class_path)
}

pub fn load_java_fs<T>(path: T, class_path: String) -> Result<dto::Class, dto::ClassError>
where
    T: AsRef<Path> + Debug,
{
    let bytes = std::fs::read(path)?;
    java::load_java(&bytes, class_path)
}

pub fn save_class_folder(prefix: &str, class_folder: &dto::ClassFolder) -> Result<(), dto::ClassError> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&format!(".{}.cfc", prefix))?;
    let data = postcard::to_allocvec(class_folder)?;

    let _ = file.write_all(&data);
    Ok(())
}

pub fn load_class_folder(prefix: &str) -> Result<dto::ClassFolder, dto::ClassError> {
    let data = fs::read(Path::new(&format!(".{}.cfc", prefix)))?;
    let out = postcard::from_bytes(&data)?;

    Ok(out)
}

pub fn load_classes(path: &str) -> dto::ClassFolder {
    dto::ClassFolder {
        classes: get_classes(path)
            .into_iter()
            .filter_map(|p| {
                let class_path = p.trim_start_matches(path);
                let class_path = class_path.trim_end_matches(".class");
                let class_path = class_path.replace("/", ".");
                dbg!(&class_path);
                load_class_fs(p.clone(), class_path.to_string()).ok()
            })
            .collect(),
    }
}

fn get_classes(dir: &str) -> Vec<String> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
        .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
        .filter(|e| e.ends_with(".class"))
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::load_class_fs;

    //#[test]
    //fn load_jdk() {
    //    dbg!(load_jdk_folder(
    //        "/home/emily/Documents/java/getting-started/jdk/classes/" //"D:\\rust\\jdk\\classes\\"
    //    )
    //    .len());
    //    assert!(false);
    //}

    #[test]
    fn fsbug() {
        let _ = load_class_fs(
            Path::new(
                "/home/emily/Documents/java/getting-started/jdk/classes/java/util/HashMap.class",
            ),
            "".to_string()
        );
    }
}
