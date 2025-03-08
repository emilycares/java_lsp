use std::{path::PathBuf, process::Command};

use common::config::{FormatterConfig, CONFIG};
use topiary_config::Configuration;
use topiary_core::{formatter, Language, Operation, TopiaryQuery};

pub fn format(text: String, path: PathBuf) -> Option<String> {
    match CONFIG.formatter {
        FormatterConfig::Topiary => topiary(text),
        FormatterConfig::Intelij => intelij(path),
        FormatterConfig::None => None,
    }
}

pub enum Formatter {
    Topiary { text: String },
    Intelij { path: PathBuf },
    None,
}

pub fn format_op(formatter: Formatter) -> Option<String> {
    match formatter {
        Formatter::Topiary { text } => topiary(text),
        Formatter::Intelij { path } => intelij(path),
        Formatter::None => todo!(),
    }
}

fn intelij(path: PathBuf) -> Option<String> {
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

    None
}

fn topiary(text: String) -> Option<String> {
    let mut input = text.as_bytes();
    let mut output = Vec::new();
    let config = Configuration::default();
    let java = config.get_language("java").ok()?;
    let query = topiary_queries::java();
    let grammar = java.grammar().ok()?;
    let query = TopiaryQuery::new(&grammar, query);
    if let Err(e) = &query {
        eprintln!("Format query error: {:?}", e);
        return None;
    }
    let language: Language = Language {
        name: "java".to_owned(),
        query: query.ok()?,
        grammar,
        indent: Some("    ".to_string()),
    };
    let operation = formatter(
        &mut input,
        &mut output,
        &language,
        Operation::Format {
            skip_idempotence: true,
            tolerate_parsing_errors: false,
        },
    );
    if let Err(e) = &operation {
        eprintln!("Format operation error: {:?}", e);
        return None;
    }
    let formatted = String::from_utf8(output).expect("valid utf-8");
    Some(formatted)
}
