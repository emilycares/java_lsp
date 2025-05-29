use std::{path::PathBuf, process::Command};

use config::{CONFIG, FormatterConfig};

#[derive(Debug)]
pub enum FormatError {
    IO(std::io::Error),
}

pub fn format(path: PathBuf) -> Result<(), FormatError> {
    match CONFIG.formatter {
        FormatterConfig::Intelij => intelij(path),
        FormatterConfig::None => todo!(),
    }
}

pub enum Formatter {
    Intelij { path: PathBuf },
    None,
}

pub fn format_op(formatter: Formatter) -> Result<(), FormatError> {
    match formatter {
        Formatter::Intelij { path } => intelij(path),
        Formatter::None => Ok(()),
    }
}

fn intelij(path: PathBuf) -> Result<(), FormatError> {
    tokio::spawn(async move {
        match Command::new("idea-community")
            .arg("format")
            .arg(path)
            .output()
        {
            Ok(_r) => eprintln!("Intelij format ok"),
            Err(e) => eprintln!("Intelij format error: {e:?}"),
        }
    });

    Ok(())
}
