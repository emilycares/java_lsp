use topiary_config::Configuration;
use topiary_core::{formatter, Language, Operation, TopiaryQuery};

pub enum Formatter {
    Topiary,
    None,
}

pub fn format(text: String, formatter: Formatter) -> Option<String> {
    match formatter {
        Formatter::Topiary => topiary(text),
        Formatter::None => todo!(),
    }
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
