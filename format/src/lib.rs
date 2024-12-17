use topiary_config::Configuration;
use topiary_core::{formatter, Language, Operation, TopiaryQuery};

pub enum Formatter {
    Topiary,
    None,
}

pub fn format(text: String, formater: Formatter) -> Option<String> {
    match formater {
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
    let language: Language = Language {
        name: "java".to_owned(),
        query: TopiaryQuery::new(&grammar, query).unwrap(),
        grammar,
        indent: None,
    };
    formatter(
        &mut input,
        &mut output,
        &language,
        Operation::Format {
            skip_idempotence: false,
            tolerate_parsing_errors: false,
        },
    )
    .unwrap();
    let formatted = String::from_utf8(output).expect("valid utf-8");
    return Some(formatted);
}
