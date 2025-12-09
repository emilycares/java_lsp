#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::{num::TryFromIntError, str::Utf8Error};

use ast::types::{AstFile, AstRange, AstThing};
use lsp_types::{Location, Range, SymbolInformation, SymbolKind, Uri};

#[derive(Debug, PartialEq)]
pub enum PosionError {
    Utf8(Utf8Error),
    Lexer(ast::lexer::LexerError),
    Ast(ast::error::AstError),
    Int(TryFromIntError),
}

pub fn get_class_position_ast(
    ast: &AstFile,
    name: Option<&str>,
) -> Result<Vec<PositionSymbol>, PosionError> {
    let mut out = vec![];
    for th in &ast.things {
        get_class_position_ast_thing(th, name, &mut out)?;
    }
    Ok(out)
}
pub fn get_class_position_ast_thing(
    thing: &AstThing,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) -> Result<(), PosionError> {
    match &thing {
        AstThing::Class(ast_class) => {
            if let Some(name) = name
                && ast_class.name.value != name
            {
                return Ok(());
            }
            out.push(PositionSymbol::Range(ast_class.name.range));
        }
        AstThing::Record(ast_record) => {
            if let Some(name) = name
                && ast_record.name.value != name
            {
                return Ok(());
            }
            out.push(PositionSymbol::Range(ast_record.name.range));
        }
        AstThing::Interface(ast_interface) => {
            if let Some(name) = name
                && ast_interface.name.value != name
            {
                return Ok(());
            }
            out.push(PositionSymbol::Range(ast_interface.name.range));
        }
        AstThing::Enumeration(ast_enumeration) => {
            if let Some(name) = name
                && ast_enumeration.name.value != name
            {
                return Ok(());
            }
            out.push(PositionSymbol::Range(ast_enumeration.name.range));
        }
        AstThing::Annotation(ast_annotation) => {
            if let Some(name) = name
                && ast_annotation.name.value != name
            {
                return Ok(());
            }
            out.push(PositionSymbol::Range(ast_annotation.name.range));
        }
    }
    Ok(())
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
    let mut out = vec![];
    for th in &ast.things {
        get_method_position_ast_thing(th, name, &mut out)?;
    }
    Ok(out)
}
pub fn get_method_position_ast_thing(
    thing: &AstThing,
    name: &str,
    out: &mut Vec<PositionSymbol>,
) -> Result<(), PosionError> {
    match thing {
        AstThing::Class(ast_class) => out.extend(
            ast_class
                .block
                .methods
                .iter()
                .map(|i| &i.header.name)
                .filter(|i| i.value == name)
                .map(|i| PositionSymbol::Range(i.range)),
        ),
        AstThing::Record(ast_record) => out.extend(
            ast_record
                .block
                .methods
                .iter()
                .map(|i| &i.header.name)
                .filter(|i| i.value == name)
                .map(|i| PositionSymbol::Range(i.range)),
        ),
        AstThing::Interface(ast_interface) => out.extend(
            ast_interface
                .methods
                .iter()
                .map(|i| &i.header.name)
                .filter(|i| i.value == name)
                .map(|i| PositionSymbol::Range(i.range)),
        ),
        AstThing::Enumeration(ast_enumeration) => out.extend(
            ast_enumeration
                .methods
                .iter()
                .map(|i| &i.header.name)
                .filter(|i| i.value == name)
                .map(|i| PositionSymbol::Range(i.range)),
        ),
        AstThing::Annotation(_ast_annotation) => (),
    }
    Ok(())
}

pub fn get_field_positions(bytes: &[u8], name: &str) -> Result<Vec<PositionSymbol>, PosionError> {
    let str = str::from_utf8(bytes).map_err(PosionError::Utf8)?;
    let tokens = ast::lexer::lex(str).map_err(PosionError::Lexer)?;
    let ast = ast::parse_file(&tokens).map_err(PosionError::Ast)?;
    let mut out = vec![];
    for th in &ast.things {
        get_field_position_ast_thing(th, name, &mut out)?;
    }
    Ok(out)
}
pub fn get_field_position_ast_thing(
    thing: &AstThing,
    name: &str,
    out: &mut Vec<PositionSymbol>,
) -> Result<(), PosionError> {
    match thing {
        AstThing::Class(ast_class) => out.extend(
            ast_class
                .block
                .variables
                .iter()
                .filter(|i| i.name.value == name)
                .map(|i| PositionSymbol::Range(i.range)),
        ),
        AstThing::Record(ast_record) => out.extend(
            ast_record
                .block
                .variables
                .iter()
                .filter(|i| i.name.value == name)
                .map(|i| PositionSymbol::Range(i.range)),
        ),
        AstThing::Interface(ast_interface) => out.extend(
            ast_interface
                .constants
                .iter()
                .map(|i| &i.name)
                .filter(|i| i.value == name)
                .map(|i| PositionSymbol::Range(i.range)),
        ),
        AstThing::Enumeration(ast_enumeration) => out.extend(
            ast_enumeration
                .methods
                .iter()
                .map(|i| &i.header.name)
                .filter(|i| i.value == name)
                .map(|i| PositionSymbol::Range(i.range)),
        ),
        AstThing::Annotation(_ast_annotation) => (),
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum PositionSymbol {
    Range(AstRange),
    Symbol {
        range: AstRange,
        name: String,
        kind: String,
    },
}

impl PositionSymbol {
    #[must_use]
    pub const fn get_range(&self) -> &AstRange {
        match self {
            Self::Symbol {
                range,
                name: _,
                kind: _,
            }
            | Self::Range(range) => range,
        }
    }
}

pub const fn get_type_usage(
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

pub const fn get_method_usage(
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
    symbols: &[PositionSymbol],
    uri: &Uri,
) -> Vec<SymbolInformation> {
    symbols
        .iter()
        .filter_map(|r| match r {
            PositionSymbol::Range(_) => None,
            PositionSymbol::Symbol { range, name, kind } => {
                #[allow(deprecated)]
                Some(SymbolInformation {
                    name: name.clone(),
                    kind: match kind.as_str() {
                        "method_declaration" => SymbolKind::METHOD,
                        "class_declaration" => SymbolKind::CLASS,
                        // also "formal_parameter" | "variable_declarator"  => SymbolKind::FIELD,
                        _ => SymbolKind::FIELD,
                    },
                    tags: Some(vec![]),
                    deprecated: None,
                    location: Location {
                        uri: uri.clone(),
                        range: Range {
                            start: lsp_types::Position {
                                line: u32::try_from(range.start.line).ok()?,
                                character: u32::try_from(range.start.col).ok()?,
                            },
                            end: lsp_types::Position {
                                line: u32::try_from(range.end.line).ok()?,
                                character: u32::try_from(range.end.col).ok()?,
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
        PositionSymbol, get_class_position_ast, get_field_positions, get_method_positions,
        get_type_usage,
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
            Ok(vec![PositionSymbol::Range(AstRange {
                start: AstPoint { line: 3, col: 4 },
                end: AstPoint { line: 3, col: 19 },
            }),]),
            out
        );
    }

    #[test]
    fn class_pos_base() {
        let content = "
package ch.emilycares;
public class Test {}
";
        let tokens = ast::lexer::lex(content).unwrap();
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
