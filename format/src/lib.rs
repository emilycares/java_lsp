use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
    process::Command,
};

use common::config::{FormatterConfig, CONFIG};
use topiary_config::{
    error::{TopiaryConfigError, TopiaryConfigFetchingError},
    Configuration,
};
use topiary_core::{formatter, FormatterError, Language, Operation, TopiaryQuery};

#[derive(Debug)]
pub enum FormatError {
    IO(std::io::Error),
    TopiaryConfig(TopiaryConfigError),
    TopiaryFormatter(FormatterError),
    Grammar(TopiaryConfigFetchingError),
}

pub fn format(path: PathBuf) -> Result<(), FormatError> {
    match CONFIG.formatter {
        FormatterConfig::Topiary => topiary(path),
        FormatterConfig::Intelij => intelij(path),
        FormatterConfig::None => todo!(),
    }
}

pub enum Formatter {
    Topiary { path: PathBuf },
    Intelij { path: PathBuf },
    None,
}

pub fn format_op(formatter: Formatter) -> Result<(), FormatError> {
    match formatter {
        Formatter::Topiary { path } => topiary(path),
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
            Err(e) => eprintln!("Intelij format error: {:?}", e),
        }
    });

    Ok(())
}

fn topiary(path: PathBuf) -> Result<(), FormatError> {
    let config = Configuration::default();
    let java = config
        .get_language("java")
        .map_err(FormatError::TopiaryConfig)?;
    let query = topiary_queries::java();
    let grammar = java.grammar().map_err(FormatError::Grammar)?;
    let query = TopiaryQuery::new(&grammar, query).map_err(FormatError::TopiaryFormatter)?;
    let language: Language = Language {
        name: "java".to_owned(),
        query,
        grammar,
        indent: Some("    ".to_string()),
    };
    let f = File::open(path).map_err(FormatError::IO)?;
    let mut reader = BufReader::new(&f);
    let mut writer = BufWriter::new(&f);
    formatter(
        &mut reader,
        &mut writer,
        &language,
        Operation::Format {
            skip_idempotence: true,
            tolerate_parsing_errors: false,
        },
    )
    .map_err(FormatError::TopiaryFormatter)
}
