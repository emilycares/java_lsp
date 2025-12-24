#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use std::{num::TryFromIntError, str::Utf8Error};

use ast::types::{
    AstBlock, AstBlockEntry, AstClassBlock, AstFile, AstIdentifier, AstIf, AstIfContent, AstRange,
    AstThing,
};
use lsp_extra::to_lsp_range;
use lsp_types::{Location, SymbolInformation, SymbolKind, Uri};

#[derive(Debug, PartialEq)]
pub enum PositionError {
    Utf8(Utf8Error),
    Lexer(ast::lexer::LexerError),
    Ast(ast::error::AstError),
    Int(TryFromIntError),
}

pub fn get_class_position_ast(
    ast: &AstFile,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) -> Result<(), PositionError> {
    for thing in &ast.things {
        get_class_position_ast_thing(thing, name, out)?;
    }
    Ok(())
}

fn get_class_position_ast_thing(
    thing: &AstThing,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) -> Result<(), PositionError> {
    let kind = SymbolKind::CLASS;
    match &thing {
        AstThing::Class(ast_class) => {
            if let Some(name) = name
                && ast_class.name.value == name
            {
                out.push(PositionSymbol {
                    range: ast_class.range,
                    name: ast_class.name.value.clone(),
                    kind,
                });
            }
            for inner in &ast_class.block.inner {
                get_class_position_ast_thing(inner, name, out)?;
            }
        }
        AstThing::Record(ast_record) => {
            if let Some(name) = name
                && ast_record.name.value == name
            {
                out.push(PositionSymbol {
                    range: ast_record.range,
                    name: ast_record.name.value.clone(),
                    kind,
                });
            }
            for inner in &ast_record.block.inner {
                get_class_position_ast_thing(inner, name, out)?;
            }
        }
        AstThing::Interface(ast_interface) => {
            if let Some(name) = name
                && ast_interface.name.value == name
            {
                out.push(PositionSymbol {
                    range: ast_interface.range,
                    name: ast_interface.name.value.clone(),
                    kind: SymbolKind::INTERFACE,
                });
            }
            for inner in &ast_interface.inner {
                get_class_position_ast_thing(inner, name, out)?;
            }
        }
        AstThing::Enumeration(ast_enumeration) => {
            if let Some(name) = name
                && ast_enumeration.name.value != name
            {
                return Ok(());
            }
            out.push(PositionSymbol {
                range: ast_enumeration.range,
                name: ast_enumeration.name.value.clone(),
                kind: SymbolKind::ENUM,
            });
            for inner in &ast_enumeration.inner {
                get_class_position_ast_thing(inner, name, out)?;
            }
        }
        AstThing::Annotation(ast_annotation) => {
            if let Some(name) = name
                && ast_annotation.name.value == name
            {
                out.push(PositionSymbol {
                    range: ast_annotation.range,
                    name: ast_annotation.name.value.clone(),
                    kind,
                });
            }
        }
    }
    Ok(())
}

pub fn get_class_position(
    bytes: &[u8],
    name: Option<&str>,
) -> Result<Vec<PositionSymbol>, PositionError> {
    let str = str::from_utf8(bytes).map_err(PositionError::Utf8)?;
    let tokens = ast::lexer::lex(str).map_err(PositionError::Lexer)?;
    let ast = ast::parse_file(&tokens).map_err(PositionError::Ast)?;
    let mut out = vec![];
    get_class_position_ast(&ast, name, &mut out)?;
    Ok(out)
}
pub fn get_class_position_str(
    str: &str,
    name: Option<&str>,
) -> Result<Vec<PositionSymbol>, PositionError> {
    let tokens = ast::lexer::lex(str).map_err(PositionError::Lexer)?;
    let ast = ast::parse_file(&tokens).map_err(PositionError::Ast)?;
    let mut out = vec![];
    get_class_position_ast(&ast, name, &mut out)?;
    Ok(out)
}

pub fn get_method_positions(
    bytes: &[u8],
    name: Option<&str>,
) -> Result<Vec<PositionSymbol>, PositionError> {
    let str = str::from_utf8(bytes).map_err(PositionError::Utf8)?;
    let tokens = ast::lexer::lex(str).map_err(PositionError::Lexer)?;
    let ast = ast::parse_file(&tokens).map_err(PositionError::Ast)?;
    let mut out = vec![];
    get_method_position_ast(&ast, name, &mut out)?;
    Ok(out)
}
pub fn get_method_position_ast(
    file: &AstFile,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) -> Result<(), PositionError> {
    for thing in &file.things {
        get_method_position_ast_thing(thing, name, out)?;
    }
    Ok(())
}

pub fn get_method_position_ast_thing(
    thing: &AstThing,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) -> Result<(), PositionError> {
    match thing {
        AstThing::Class(ast_class) => {
            out.extend(
                ast_class
                    .block
                    .methods
                    .iter()
                    .filter(|i| is_valid_name(name, &i.header.name))
                    .map(|i| PositionSymbol {
                        range: i.range,
                        name: i.header.name.value.clone(),
                        kind: SymbolKind::METHOD,
                    }),
            );
            for inner in &ast_class.block.inner {
                get_method_position_ast_thing(inner, name, out)?;
            }
        }
        AstThing::Record(ast_record) => {
            out.extend(
                ast_record
                    .block
                    .methods
                    .iter()
                    .filter(|i| is_valid_name(name, &i.header.name))
                    .map(|i| PositionSymbol {
                        range: i.range,
                        name: i.header.name.value.clone(),
                        kind: SymbolKind::METHOD,
                    }),
            );
            for inner in &ast_record.block.inner {
                get_method_position_ast_thing(inner, name, out)?;
            }
        }
        AstThing::Interface(ast_interface) => {
            out.extend(
                ast_interface
                    .methods
                    .iter()
                    .filter(|i| is_valid_name(name, &i.header.name))
                    .map(|i| PositionSymbol {
                        range: i.range,
                        name: i.header.name.value.clone(),
                        kind: SymbolKind::METHOD,
                    }),
            );
            out.extend(
                ast_interface
                    .default_methods
                    .iter()
                    .filter(|i| is_valid_name(name, &i.header.name))
                    .map(|i| PositionSymbol {
                        range: i.range,
                        name: i.header.name.value.clone(),
                        kind: SymbolKind::METHOD,
                    }),
            );
            for inner in &ast_interface.inner {
                get_method_position_ast_thing(inner, name, out)?;
            }
        }
        AstThing::Enumeration(ast_enumeration) => {
            out.extend(
                ast_enumeration
                    .methods
                    .iter()
                    .filter(|i| is_valid_name(name, &i.header.name))
                    .map(|i| PositionSymbol {
                        range: i.range,
                        name: i.header.name.value.clone(),
                        kind: SymbolKind::METHOD,
                    }),
            );
            for inner in &ast_enumeration.inner {
                get_method_position_ast_thing(inner, name, out)?;
            }
        }
        AstThing::Annotation(_ast_annotation) => (),
    }
    Ok(())
}

fn is_valid_name(name: Option<&str>, i: &AstIdentifier) -> bool {
    let Some(name) = name else {
        return true;
    };
    name == i.value
}

pub fn get_field_positions(
    bytes: &[u8],
    name: Option<&str>,
) -> Result<Vec<PositionSymbol>, PositionError> {
    let str = str::from_utf8(bytes).map_err(PositionError::Utf8)?;
    let tokens = ast::lexer::lex(str).map_err(PositionError::Lexer)?;
    let ast = ast::parse_file(&tokens).map_err(PositionError::Ast)?;
    let mut out = vec![];
    get_field_position_ast(&ast, name, &mut out)?;
    Ok(out)
}
pub fn get_field_position_ast(
    file: &AstFile,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) -> Result<(), PositionError> {
    for thing in &file.things {
        get_field_position_ast_thing(thing, name, out)?;
    }
    Ok(())
}
pub fn get_field_position_ast_thing(
    thing: &AstThing,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) -> Result<(), PositionError> {
    match thing {
        AstThing::Class(ast_class) => {
            get_field_position_class_block(&ast_class.block, name, out);
        }
        AstThing::Record(ast_record) => {
            get_field_position_class_block(&ast_record.block, name, out);
        }
        AstThing::Interface(ast_interface) => {
            out.extend(
                ast_interface
                    .constants
                    .iter()
                    .filter(|i| is_valid_name(name, &i.name))
                    .map(|i| PositionSymbol {
                        range: i.range,
                        name: i.name.value.clone(),
                        kind: SymbolKind::FIELD,
                    }),
            );
            ast_interface
                .default_methods
                .iter()
                .filter(|i| is_valid_name(name, &i.header.name))
                .map(|i| &i.block)
                .for_each(|i| {
                    get_field_position_block(i, name, out);
                });
        }
        AstThing::Enumeration(ast_enumeration) => {
            out.extend(
                ast_enumeration
                    .variants
                    .iter()
                    .filter(|i| is_valid_name(name, &i.name))
                    .map(|i| PositionSymbol {
                        range: i.range,
                        name: i.name.value.clone(),
                        kind: SymbolKind::ENUM_MEMBER,
                    }),
            );
            out.extend(
                ast_enumeration
                    .variables
                    .iter()
                    .filter(|i| is_valid_name(name, &i.name))
                    .map(|i| PositionSymbol {
                        range: i.range,
                        name: i.name.value.clone(),
                        kind: SymbolKind::FIELD,
                    }),
            );
            ast_enumeration
                .methods
                .iter()
                .filter_map(|i| i.block.as_ref())
                .for_each(|i| {
                    get_field_position_block(i, name, out);
                });
            for inner in &ast_enumeration.inner {
                get_field_position_ast_thing(inner, name, out)?;
            }
        }
        AstThing::Annotation(_ast_annotation) => (),
    }
    Ok(())
}

fn get_field_position_class_block(
    cblock: &AstClassBlock,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) {
    out.extend(
        cblock
            .variables
            .iter()
            .filter(|i| is_valid_name(name, &i.name))
            .map(|i| PositionSymbol {
                range: i.range,
                name: i.name.value.clone(),
                kind: SymbolKind::FIELD,
            }),
    );
    cblock
        .methods
        .iter()
        .filter_map(|i| i.block.as_ref())
        .for_each(|i| {
            get_field_position_block(i, name, out);
        });
}

fn get_field_position_block(block: &AstBlock, name: Option<&str>, out: &mut Vec<PositionSymbol>) {
    block
        .entries
        .iter()
        .for_each(|i| get_field_position_block_entry(i, name, out));
}

fn get_field_position_block_entry(
    entry: &AstBlockEntry,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) {
    match entry {
        AstBlockEntry::Variable(ast_block_variables) => {
            out.extend(
                ast_block_variables
                    .iter()
                    .filter(|i| is_valid_name(name, &i.name))
                    .map(|i| PositionSymbol {
                        range: i.range,
                        name: i.name.value.clone(),
                        kind: SymbolKind::FIELD,
                    }),
            );
        }
        AstBlockEntry::If(
            AstIf::If {
                range: _,
                control: _,
                control_range: _,
                content,
            }
            | AstIf::ElseIf {
                range: _,
                control: _,
                control_range: _,
                content,
            }
            | AstIf::Else { range: _, content },
        ) => match content {
            AstIfContent::Block(ast_block) => get_field_position_block(ast_block, name, out),
            AstIfContent::BlockEntry(ast_block_entry) => {
                get_field_position_block_entry(ast_block_entry, name, out);
            }
        },
        _ => (),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PositionSymbol {
    pub range: AstRange,
    pub name: String,
    pub kind: SymbolKind,
}

pub const fn get_type_usage(
    _query_class_name: &str,
    _ast: &AstFile,
) -> Result<Vec<PositionSymbol>, PositionError> {
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
) -> Result<Vec<PositionSymbol>, PositionError> {
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
        .filter_map(|r| {
            let Ok(range) = to_lsp_range(&r.range) else {
                return None;
            };
            #[allow(deprecated)]
            Some(SymbolInformation {
                name: r.name.clone(),
                kind: r.kind,
                tags: Some(vec![]),
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
                    range,
                },
                container_name: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use ast::types::{AstPoint, AstRange};
    use lsp_types::SymbolKind;
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
        let out = get_method_positions(content, Some("hello"));
        assert_eq!(
            out,
            Ok(vec![PositionSymbol {
                range: AstRange {
                    start: AstPoint { line: 3, col: 4 },
                    end: AstPoint { line: 7, col: 5 },
                },
                name: "hello".to_string(),
                kind: SymbolKind::METHOD,
            },])
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
        let out = get_field_positions(content, Some("a"));
        assert_eq!(
            Ok(vec![PositionSymbol {
                range: AstRange {
                    start: AstPoint { line: 3, col: 4 },
                    end: AstPoint { line: 3, col: 19 },
                },
                name: "a".to_string(),
                kind: SymbolKind::FIELD,
            },]),
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
        let mut out = vec![];
        get_class_position_ast(&ast, Some("Test"), &mut out).unwrap();
        assert_eq!(
            out,
            vec![PositionSymbol {
                range: AstRange {
                    start: AstPoint { line: 2, col: 0 },
                    end: AstPoint { line: 2, col: 20 },
                },
                name: "Test".to_string(),
                kind: SymbolKind::CLASS,
            },]
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
