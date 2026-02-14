#![deny(clippy::redundant_clone)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]

use ast::types::{
    AstBlock, AstBlockEntry, AstClassBlock, AstExpressionKind, AstExpressionOrValue, AstFile,
    AstIdentifier, AstIf, AstIfContent, AstLambdaRhs, AstRange, AstThing,
};
use lsp_extra::to_lsp_range;
use lsp_types::{Location, SymbolInformation, SymbolKind, Uri};
use my_string::MyString;

#[derive(Debug, PartialEq, Eq)]
pub struct PositionSymbol {
    pub range: AstRange,
    pub name: MyString,
    pub kind: SymbolKind,
}

pub fn get_class_position_ast(ast: &AstFile, name: Option<&str>, out: &mut Vec<PositionSymbol>) {
    for thing in &ast.things {
        get_class_position_ast_thing(thing, name, out);
    }
}

fn get_class_position_ast_thing(
    thing: &AstThing,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) {
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
                get_class_position_ast_thing(inner, name, out);
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
                get_class_position_ast_thing(inner, name, out);
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
                get_class_position_ast_thing(inner, name, out);
            }
        }
        AstThing::Enumeration(ast_enumeration) => {
            if let Some(name) = name
                && ast_enumeration.name.value != name
            {
                return;
            }
            out.push(PositionSymbol {
                range: ast_enumeration.range,
                name: ast_enumeration.name.value.clone(),
                kind: SymbolKind::ENUM,
            });
            for inner in &ast_enumeration.inner {
                get_class_position_ast_thing(inner, name, out);
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
}

pub fn get_method_position_ast(file: &AstFile, name: Option<&str>, out: &mut Vec<PositionSymbol>) {
    for thing in &file.things {
        get_method_position_ast_thing(thing, name, out);
    }
}

pub fn get_method_position_ast_thing(
    thing: &AstThing,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) {
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
                get_method_position_ast_thing(inner, name, out);
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
                get_method_position_ast_thing(inner, name, out);
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
                get_method_position_ast_thing(inner, name, out);
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
                get_method_position_ast_thing(inner, name, out);
            }
        }
        AstThing::Annotation(_ast_annotation) => (),
    }
}

fn is_valid_name(name: Option<&str>, i: &AstIdentifier) -> bool {
    let Some(name) = name else {
        return true;
    };
    name == i.value
}

pub fn get_field_position_ast(file: &AstFile, name: Option<&str>, out: &mut Vec<PositionSymbol>) {
    for thing in &file.things {
        get_field_position_ast_thing(thing, name, out);
    }
}
pub fn get_field_position_ast_thing(
    thing: &AstThing,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) {
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
                get_field_position_ast_thing(inner, name, out);
            }
        }
        AstThing::Annotation(_ast_annotation) => (),
    }
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
    for e in &block.entries {
        get_field_position_block_entry(e, name, out);
    }
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
        AstBlockEntry::Return(r) => {
            get_field_position_expression_or_value(&r.expression, name, out);
        }
        AstBlockEntry::Expression(e) => {
            for e in &e.value {
                get_field_position_expression(e, name, out);
            }
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

fn get_field_position_expression_or_value(
    expression: &AstExpressionOrValue,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) {
    match expression {
        AstExpressionOrValue::Expression(ast_expression_kinds) => {
            for e in ast_expression_kinds {
                get_field_position_expression(e, name, out);
            }
        }
        AstExpressionOrValue::None | AstExpressionOrValue::Value(_) => (),
    }
}

fn get_field_position_expression(
    i: &AstExpressionKind,
    name: Option<&str>,
    out: &mut Vec<PositionSymbol>,
) {
    match i {
        AstExpressionKind::Recursive(r) => {
            if let Some(vals) = &r.values {
                for v in &vals.values {
                    for e in v {
                        get_field_position_expression(e, name, out);
                    }
                }
            }
        }
        AstExpressionKind::Lambda(ast_lambda) => {
            out.extend(ast_lambda.parameters.values.iter().map(|i| PositionSymbol {
                range: i.range,
                name: i.name.value.clone(),
                kind: SymbolKind::FIELD,
            }));
            match &ast_lambda.rhs {
                AstLambdaRhs::None => (),
                AstLambdaRhs::Block(ast_block) => get_field_position_block(ast_block, name, out),
                AstLambdaRhs::Expr(ast_expression_kinds) => {
                    for e in ast_expression_kinds {
                        get_field_position_expression(e, name, out);
                    }
                }
            }
        }
        AstExpressionKind::InlineSwitch(s) => {
            get_field_position_block(&s.block, name, out);
        }
        _ => (),
    }
}

pub const fn get_type_usage(_query_class_name: &str, _ast: &AstFile) {
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

pub const fn get_method_usage(_query_method_name: &str, _ast: &AstFile) {
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
                name: r.name.to_string(),
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
    use my_string::smol_str::ToSmolStr;
    use pretty_assertions::assert_eq;

    use crate::{
        PositionSymbol, get_class_position_ast, get_field_position_ast, get_method_position_ast,
        get_type_usage,
    };

    #[test]
    fn method_pos_base() {
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
        if (a == b ) {
        }
        return;
    }
}
";
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();
        let mut out = vec![];
        get_method_position_ast(&ast, Some("hello"), &mut out);
        assert_eq!(
            out,
            vec![PositionSymbol {
                range: AstRange {
                    start: AstPoint { line: 3, col: 4 },
                    end: AstPoint { line: 7, col: 5 },
                },
                name: "hello".to_smolstr(),
                kind: SymbolKind::METHOD,
            },]
        );
    }

    #[test]
    fn field_pos_base() {
        let content = "
package ch.emilycares;
public class Test {
    public String a;
}
";
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();
        let mut out = vec![];
        get_field_position_ast(&ast, Some("a"), &mut out);
        assert_eq!(
            vec![PositionSymbol {
                range: AstRange {
                    start: AstPoint { line: 3, col: 4 },
                    end: AstPoint { line: 3, col: 19 },
                },
                name: "a".to_smolstr(),
                kind: SymbolKind::FIELD,
            },],
            out
        );
    }

    #[test]
    fn field_pos_in_lambda() {
        let content = "
public class Test {
    public Uni<Response> test() {
        return Thing.dothing(t -> {
                    Definition q = new Definition();
                    
                    });
    }
}
        ";
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();
        let mut out = vec![];
        get_field_position_ast(&ast, None, &mut out);
        assert_eq!(
            vec![
                PositionSymbol {
                    range: AstRange {
                        start: AstPoint { line: 3, col: 29 },
                        end: AstPoint { line: 3, col: 30 },
                    },
                    name: "t".to_smolstr(),
                    kind: SymbolKind::FIELD,
                },
                PositionSymbol {
                    range: AstRange {
                        start: AstPoint { line: 4, col: 20 },
                        end: AstPoint { line: 4, col: 51 },
                    },
                    name: "q".to_smolstr(),
                    kind: SymbolKind::FIELD,
                },
            ],
            out
        );
    }

    #[test]
    fn class_pos_base() {
        let content = "
package ch.emilycares;
public class Test {}
";
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();
        let mut out = vec![];
        get_class_position_ast(&ast, Some("Test"), &mut out);
        assert_eq!(
            out,
            vec![PositionSymbol {
                range: AstRange {
                    start: AstPoint { line: 2, col: 0 },
                    end: AstPoint { line: 2, col: 20 },
                },
                name: "Test".to_smolstr(),
                kind: SymbolKind::CLASS,
            },]
        );
    }
    #[ignore = "todo"]
    #[test]
    fn type_usage_base() {
        let content = "
package ch.emilycares;
public class Test {
private StringBuilder sb = new StringBuilder();
}
";
        let tokens = ast::lexer::lex(content.as_bytes()).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();
        get_type_usage("StringBuilder", &ast);
        // assert here
    }
}
