use rc_zip_tokio::{ReadZip, rc_zip::parse::EntryKind};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum ZipUtilError {
    IO(std::io::Error),
    Zip(rc_zip_tokio::rc_zip::error::Error),
}

pub async fn extract_jar(jar: &PathBuf, source_dir: &Path) -> Result<(), ZipUtilError> {
    let dir = PathBuf::from(source_dir);
    let buf = tokio::fs::read(jar).await.map_err(ZipUtilError::IO)?;
    let reader = buf.read_zip().await.map_err(ZipUtilError::Zip)?;

    for entry in reader.entries() {
        let entry_name = match entry.sanitized_name() {
            Some(name) => name,
            None => continue,
        };

        match entry.kind() {
            EntryKind::Symlink => {
                #[cfg(windows)]
                {
                    let path = dir.join(entry_name);
                    std::fs::create_dir_all(
                        path.parent()
                            .expect("all full entry paths should have parent paths"),
                    )
                    .map_err(ZipUtilError::IO)?;

                    let mut entry_writer = File::create(path).map_err(ZipUtilError::IO)?;
                    let buf = entry.bytes().await.map_err(ZipUtilError::IO)?;

                    entry_writer
                        .write(buf.as_slice())
                        .map_err(ZipUtilError::IO)?;
                }

                #[cfg(not(windows))]
                {
                    use tokio::io::AsyncReadExt;

                    let path = dir.join(entry_name);
                    std::fs::create_dir_all(
                        path.parent()
                            .expect("all full entry paths should have parent paths"),
                    )
                    .map_err(ZipUtilError::IO)?;
                    if let Ok(metadata) = std::fs::symlink_metadata(&path) {
                        if metadata.is_file() {
                            std::fs::remove_file(&path).map_err(ZipUtilError::IO)?;
                        }
                    }

                    let mut src = String::new();
                    entry
                        .reader()
                        .read_to_string(&mut src)
                        .await
                        .map_err(ZipUtilError::IO)?;

                    // validate pointing path before creating a symbolic link
                    if src.contains("..") {
                        continue;
                    }
                    std::os::unix::fs::symlink(src, &path).map_err(ZipUtilError::IO)?;
                }
            }
            EntryKind::Directory => {
                let path = dir.join(entry_name);
                std::fs::create_dir_all(
                    path.parent()
                        .expect("all full entry paths should have parent paths"),
                )
                .map_err(ZipUtilError::IO)?;
            }
            EntryKind::File => {
                let path = dir.join(entry_name);
                std::fs::create_dir_all(
                    path.parent()
                        .expect("all full entry paths should have parent paths"),
                )
                .map_err(ZipUtilError::IO)?;
                let mut entry_writer = File::create(path).map_err(ZipUtilError::IO)?;
                let buf = entry.bytes().await.map_err(ZipUtilError::IO)?;

                entry_writer
                    .write(buf.as_slice())
                    .map_err(ZipUtilError::IO)?;
            }
        }
    }

    Ok(())
}
