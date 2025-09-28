use std::str::Utf8Error;

use ast::types::{AstFile, AstRange};
use lsp_types::{Location, Range, SymbolInformation, SymbolKind, Uri};

#[derive(Debug, PartialEq)]
pub enum PosionError {
    Utf8(Utf8Error),
    Lexer(ast::lexer::LexerError),
    Ast(ast::error::AstError),
}

pub fn get_class_position_ast(
    ast: &AstFile,
    name: Option<&str>,
) -> Result<Vec<PositionSymbol>, PosionError> {
    match &ast.thing {
        ast::types::AstThing::Class(ast_class) => {
            if let Some(name) = name
                && ast_class.name.value != name
            {
                return Ok(vec![]);
            }
            Ok(vec![PositionSymbol::Range(ast_class.name.range)])
        }
        ast::types::AstThing::Interface(ast_interface) => {
            if let Some(name) = name
                && ast_interface.name.value != name
            {
                return Ok(vec![]);
            }
            Ok(vec![PositionSymbol::Range(ast_interface.name.range)])
        }
        ast::types::AstThing::Enumeration(ast_enumeration) => {
            if let Some(name) = name
                && ast_enumeration.name.value != name
            {
                return Ok(vec![]);
            }
            Ok(vec![PositionSymbol::Range(ast_enumeration.name.range)])
        }
        ast::types::AstThing::Annotation(ast_annotation) => {
            if let Some(name) = name
                && ast_annotation.name.value != name
            {
                return Ok(vec![]);
            }
            Ok(vec![PositionSymbol::Range(ast_annotation.name.range)])
        }
    }
}

pub fn get_class_position(
    bytes: &[u8],
    name: Option<&str>,
) -> Result<Vec<PositionSymbol>, PosionError> {
    let str = str::from_utf8(bytes).map_err(PosionError::Utf8)?;
    let tokens = ast::lexer::lex(str).map_err(PosionError::Lexer)?;
    let ast = ast::parse_file(&tokens).map_err(PosionError::Ast)?;
    get_class_position_ast(&ast, name)
}
pub fn get_class_position_str(
    str: &str,
    name: Option<&str>,
) -> Result<Vec<PositionSymbol>, PosionError> {
    let tokens = ast::lexer::lex(str).map_err(PosionError::Lexer)?;
    let ast = ast::parse_file(&tokens).map_err(PosionError::Ast)?;
    get_class_position_ast(&ast, name)
}

pub fn get_method_positions(bytes: &[u8], name: &str) -> Result<Vec<PositionSymbol>, PosionError> {
    let str = str::from_utf8(bytes).map_err(PosionError::Utf8)?;
    let tokens = ast::lexer::lex(str).map_err(PosionError::Lexer)?;
    let ast = ast::parse_file(&tokens).map_err(PosionError::Ast)?;

    match &ast.thing {
        ast::types::AstThing::Class(ast_class) => Ok(ast_class
            .block
            .methods
            .iter()
            .map(|i| &i.header.name)
            .filter(|i| i.value == name)
            .map(|i| PositionSymbol::Range(i.range))
            .collect()),
        ast::types::AstThing::Interface(ast_interface) => Ok(ast_interface
            .methods
            .iter()
            .map(|i| &i.header.name)
            .filter(|i| i.value == name)
            .map(|i| PositionSymbol::Range(i.range))
            .collect()),
        ast::types::AstThing::Enumeration(ast_enumeration) => Ok(ast_enumeration
            .methods
            .iter()
            .map(|i| &i.header.name)
            .filter(|i| i.value == name)
            .map(|i| PositionSymbol::Range(i.range))
            .collect()),
        ast::types::AstThing::Annotation(_ast_annotation) => Ok(vec![]),
    }
}

pub fn get_field_positions(bytes: &[u8], name: &str) -> Result<Vec<PositionSymbol>, PosionError> {
    let str = str::from_utf8(bytes).map_err(PosionError::Utf8)?;
    let tokens = ast::lexer::lex(str).map_err(PosionError::Lexer)?;
    let ast = ast::parse_file(&tokens).map_err(PosionError::Ast)?;

    match &ast.thing {
        ast::types::AstThing::Class(ast_class) => Ok(ast_class
            .block
            .variables
            .iter()
            .map(|i| &i.name)
            .filter(|i| i.value == name)
            .map(|i| PositionSymbol::Range(i.range))
            .collect()),
        ast::types::AstThing::Interface(ast_interface) => Ok(ast_interface
            .constants
            .iter()
            .map(|i| &i.name)
            .filter(|i| i.value == name)
            .map(|i| PositionSymbol::Range(i.range))
            .collect()),
        ast::types::AstThing::Enumeration(ast_enumeration) => Ok(ast_enumeration
            .methods
            .iter()
            .map(|i| &i.header.name)
            .filter(|i| i.value == name)
            .map(|i| PositionSymbol::Range(i.range))
            .collect()),
        ast::types::AstThing::Annotation(_ast_annotation) => Ok(vec![]),
    }
}

#[derive(Debug, PartialEq)]
pub enum PositionSymbol {
    Range(AstRange),
    Symbol {
        range: AstRange,
        name: String,
        kind: String,
    },
}

impl PositionSymbol {
    pub fn get_range(&self) -> &AstRange {
        match self {
            PositionSymbol::Symbol {
                range,
                name: _,
                kind: _,
            } => range,
            PositionSymbol::Range(range) => range,
        }
    }
}

pub fn get_type_usage(
    _query_class_name: &str,
    _ast: &AstFile,
) -> Result<Vec<PositionSymbol>, PosionError> {
    Ok(vec![])
    // get_item_ranges(
    //     tree,
    //     bytes,
    //     "
    //     (type_identifier)@capture
    //     (field_access object: (identifier)@capture )
    //     (method_invocation object: (identifier)@capture )
    //     ",
    //     Some(query_class_name),
    // )
}

pub fn get_method_usage(
    _bytes: &[u8],
    _query_method_name: &str,
    _ast: &AstFile,
) -> Result<Vec<PositionSymbol>, PosionError> {
    Ok(vec![])
    // get_item_ranges(
    //     tree,
    //     bytes,
    //     "
    //     (method_invocation name: (identifier)@cature)
    //     ",
    //     Some(query_method_name),
    // )
}

pub fn symbols_to_document_symbols(
    symbols: Vec<PositionSymbol>,
    uri: Uri,
) -> Vec<SymbolInformation> {
    symbols
        .iter()
        .filter_map(|r| match r {
            PositionSymbol::Range(_) => None,
            PositionSymbol::Symbol { range, name, kind } =>
            {
                #[allow(deprecated)]
                Some(SymbolInformation {
                    name: name.to_string(),
                    kind: match kind.as_str() {
                        "formal_parameter" => SymbolKind::FIELD,
                        "variable_declarator" => SymbolKind::FIELD,
                        "method_declaration" => SymbolKind::METHOD,
                        "class_declaration" => SymbolKind::CLASS,
                        _ => SymbolKind::FIELD,
                    },
                    tags: Some(vec![]),
                    deprecated: None,
                    location: Location {
                        uri: uri.clone(),
                        range: Range {
                            start: lsp_types::Position {
                                line: range.start.line as u32,
                                character: range.start.col as u32,
                            },
                            end: lsp_types::Position {
                                line: range.end.line as u32,
                                character: range.end.col as u32,
                            },
                        },
                    },
                    container_name: None,
                })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use ast::types::{AstPoint, AstRange};
    use pretty_assertions::assert_eq;

    use crate::{
        PositionSymbol, get_class_position, get_class_position_ast, get_field_positions,
        get_method_positions, get_type_usage,
    };

    #[test]
    fn method_pos_base() {
        let content = b"
package ch.emilycares;
public class Test {
    public void hello() {
        if (a == b ) {
        }
        return;
    }
}
";
        let out = get_method_positions(content, "hello");
        assert_eq!(
            out,
            Ok(vec![PositionSymbol::Range(AstRange {
                start: AstPoint { line: 3, col: 16 },
                end: AstPoint { line: 3, col: 21 },
            }),])
        );
    }

    #[test]
    fn field_pos_base() {
        let content = b"
package ch.emilycares;
public class Test {
    public String a;
}
";
        let out = get_field_positions(content, "a");
        assert_eq!(
            out,
            Ok(vec![PositionSymbol::Range(AstRange {
                start: AstPoint { line: 3, col: 18 },
                end: AstPoint { line: 3, col: 19 },
            }),])
        );
    }

    #[test]
    fn class_pos_base() {
        let content = "
package ch.emilycares;
public class Test {}
";
        let tokens = ast::lexer::lex(&content).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();
        let out = get_class_position_ast(&ast, Some("Test"));
        assert_eq!(
            out,
            Ok(vec![PositionSymbol::Range(AstRange {
                start: AstPoint { line: 2, col: 13 },
                end: AstPoint { line: 2, col: 17 },
            }),])
        );
    }
    #[ignore = "todo"]
    #[test]
    fn type_usage_base() {
        let content = r#"
package ch.emilycares;
public class Test {
private StringBuilder sb = new StringBuilder();
}
"#;
        let tokens = ast::lexer::lex(content).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();
        let out = get_type_usage("StringBuilder", &ast);

        assert_eq!(out.unwrap().len(), 2);
    }
}
