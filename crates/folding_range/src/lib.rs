use std::num::TryFromIntError;

use ast::types::{
    AstBlock, AstBlockEntry, AstFile, AstFor, AstForContent, AstIf, AstIfContent, AstThing,
    AstTopLevel, AstWhile, AstWhileContent,
};
use lsp_types::{FoldingRange, FoldingRangeKind};

#[derive(Debug)]
pub enum FoldingRangeError {
    Int(TryFromIntError),
}

pub fn fold(ast: &AstFile, out: &mut Vec<FoldingRange>) -> Result<(), FoldingRangeError> {
    let mut first_import = true;
    let mut imports = FoldingRange {
        start_line: 0,
        start_character: None,
        end_line: 0,
        end_character: None,
        kind: Some(FoldingRangeKind::Imports),
        collapsed_text: None,
    };
    for t in &ast.top {
        match t {
            AstTopLevel::Module(_) | AstTopLevel::Package(_) => (),
            AstTopLevel::Import(ast_import) => {
                let line =
                    u32::try_from(ast_import.range.start.line).map_err(FoldingRangeError::Int)?;
                if first_import {
                    imports.start_line = line;
                    imports.end_line = line;
                    first_import = false;
                } else {
                    imports.end_line = line;
                }
            }
            AstTopLevel::Method(m) => {
                fold_o_block(m.block.as_ref(), out)?;
            }
            AstTopLevel::Thing(ast_thing) => fold_thing(ast_thing, out)?,
        }
    }
    out.push(imports);
    Ok(())
}

fn fold_thing(ast_thing: &AstThing, out: &mut Vec<FoldingRange>) -> Result<(), FoldingRangeError> {
    match ast_thing {
        AstThing::Class(ast_class) => {
            for m in &ast_class.block.constructors {
                fold_block(&m.block, out)?;
            }
            for m in &ast_class.block.methods {
                fold_o_block(m.block.as_ref(), out)?;
            }
            for t in &ast_class.block.inner {
                fold_thing(t, out)?;
            }
        }
        AstThing::Record(ast_record) => {
            for m in &ast_record.block.constructors {
                fold_block(&m.block, out)?;
            }
            for m in &ast_record.block.methods {
                fold_o_block(m.block.as_ref(), out)?;
            }
            for t in &ast_record.block.inner {
                fold_thing(t, out)?;
            }
        }
        AstThing::Interface(ast_interface) => {
            for m in &ast_interface.default_methods {
                fold_block(&m.block, out)?;
            }
            for t in &ast_interface.inner {
                fold_thing(t, out)?;
            }
        }
        AstThing::Enumeration(ast_enumeration) => {
            for m in &ast_enumeration.constructors {
                fold_block(&m.block, out)?;
            }
            for m in &ast_enumeration.methods {
                fold_o_block(m.block.as_ref(), out)?;
            }
            for t in &ast_enumeration.inner {
                fold_thing(t, out)?;
            }
        }
        AstThing::Annotation(_) => (),
    }
    Ok(())
}

fn fold_o_block(
    block: Option<&AstBlock>,
    out: &mut Vec<FoldingRange>,
) -> Result<(), FoldingRangeError> {
    if let Some(b) = &block {
        fold_block(b, out)?;
    }
    Ok(())
}

fn fold_block(block: &AstBlock, out: &mut Vec<FoldingRange>) -> Result<(), FoldingRangeError> {
    let start_line = u32::try_from(block.range.start.line).map_err(FoldingRangeError::Int)?;
    let start_character = u32::try_from(block.range.start.col).map_err(FoldingRangeError::Int)?;
    let end_line = u32::try_from(block.range.end.line).map_err(FoldingRangeError::Int)?;
    let end_character = u32::try_from(block.range.end.col).map_err(FoldingRangeError::Int)?;
    out.push(FoldingRange {
        start_line,
        start_character: Some(start_character),
        end_line,
        end_character: Some(end_character),
        kind: Some(FoldingRangeKind::Region),
        collapsed_text: None,
    });

    for e in &block.entries {
        fold_block_entry(e, out)?;
    }

    Ok(())
}

fn fold_block_entry(
    block_entry: &AstBlockEntry,
    out: &mut Vec<FoldingRange>,
) -> Result<(), FoldingRangeError> {
    match block_entry {
        AstBlockEntry::If(ast_if) => fold_if(ast_if, out)?,
        AstBlockEntry::While(ast_while) => fold_while(ast_while, out)?,
        AstBlockEntry::For(ast_for) => fold_for(ast_for, out)?,
        AstBlockEntry::ForEnhanced(ast_for_enhanced) => {
            fold_for_content(&ast_for_enhanced.content, out)?;
        }
        AstBlockEntry::Switch(ast_switch) => fold_block(&ast_switch.block, out)?,
        AstBlockEntry::TryCatch(ast_try_catch) => {
            fold_block(&ast_try_catch.block, out)?;
            fold_o_block(ast_try_catch.finally_block.as_ref(), out)?;
        }
        AstBlockEntry::SynchronizedBlock(ast_synchronized_block) => {
            fold_block(&ast_synchronized_block.block, out)?;
        }
        AstBlockEntry::Thing(ast_thing) => fold_thing(ast_thing, out)?,
        AstBlockEntry::InlineBlock(ast_inline_block) => fold_block(&ast_inline_block.block, out)?,
        AstBlockEntry::Expression(_)
        | AstBlockEntry::Return(_)
        | AstBlockEntry::Variable(_)
        | AstBlockEntry::Assign(_)
        | AstBlockEntry::Break(_)
        | AstBlockEntry::Continue(_)
        | AstBlockEntry::Throw(_)
        | AstBlockEntry::Yield(_)
        | AstBlockEntry::Semicolon(_)
        | AstBlockEntry::SwitchCase(_)
        | AstBlockEntry::SwitchDefault(_)
        | AstBlockEntry::SwitchCaseArrowValues(_)
        | AstBlockEntry::SwitchCaseArrowType(_)
        | AstBlockEntry::SwitchCaseArrowDefault(_)
        | AstBlockEntry::Assert(_) => (),
    }
    Ok(())
}

fn fold_if(i: &AstIf, out: &mut Vec<FoldingRange>) -> Result<(), FoldingRangeError> {
    match i {
        AstIf::If {
            content: AstIfContent::Block(b),
            ..
        }
        | AstIf::ElseIf {
            content: AstIfContent::Block(b),
            ..
        }
        | AstIf::Else {
            content: AstIfContent::Block(b),
            ..
        } => fold_block(b, out),
        _ => Ok(()),
    }
}

fn fold_for(f: &AstFor, out: &mut Vec<FoldingRange>) -> Result<(), FoldingRangeError> {
    fold_for_content(&f.content, out)
}

fn fold_for_content(
    fc: &AstForContent,
    out: &mut Vec<FoldingRange>,
) -> Result<(), FoldingRangeError> {
    match fc {
        AstForContent::Block(b) => fold_block(b, out),
        AstForContent::BlockEntry(e) => fold_block_entry(e, out),
        AstForContent::None => Ok(()),
    }
}

fn fold_while(f: &AstWhile, out: &mut Vec<FoldingRange>) -> Result<(), FoldingRangeError> {
    match &f.content {
        AstWhileContent::Block(b) => fold_block(b, out),
        AstWhileContent::BlockEntry(e) => fold_block_entry(e, out),
        AstWhileContent::None => Ok(()),
    }
}

#[cfg(test)]
pub mod tests {
    use expect_test::expect;

    #[test]
    fn base() {
        let tokens = ast::lexer::lex(include_bytes!("../../parser/test/Everything.java")).unwrap();
        let ast = ast::parse_file(&tokens).unwrap();
        let mut out = Vec::new();
        super::fold(&ast, &mut out).unwrap();
        let expected = expect![[r"
            [
                FoldingRange {
                    start_line: 5,
                    start_character: Some(
                        24,
                    ),
                    end_line: 6,
                    end_character: Some(
                        5,
                    ),
                    kind: Some(
                        Region,
                    ),
                    collapsed_text: None,
                },
                FoldingRange {
                    start_line: 10,
                    start_character: Some(
                        18,
                    ),
                    end_line: 11,
                    end_character: Some(
                        5,
                    ),
                    kind: Some(
                        Region,
                    ),
                    collapsed_text: None,
                },
                FoldingRange {
                    start_line: 13,
                    start_character: Some(
                        32,
                    ),
                    end_line: 14,
                    end_character: Some(
                        5,
                    ),
                    kind: Some(
                        Region,
                    ),
                    collapsed_text: None,
                },
                FoldingRange {
                    start_line: 16,
                    start_character: Some(
                        34,
                    ),
                    end_line: 17,
                    end_character: Some(
                        5,
                    ),
                    kind: Some(
                        Region,
                    ),
                    collapsed_text: None,
                },
                FoldingRange {
                    start_line: 19,
                    start_character: Some(
                        14,
                    ),
                    end_line: 21,
                    end_character: Some(
                        5,
                    ),
                    kind: Some(
                        Region,
                    ),
                    collapsed_text: None,
                },
                FoldingRange {
                    start_line: 29,
                    start_character: Some(
                        26,
                    ),
                    end_line: 31,
                    end_character: Some(
                        5,
                    ),
                    kind: Some(
                        Region,
                    ),
                    collapsed_text: None,
                },
                FoldingRange {
                    start_line: 33,
                    start_character: Some(
                        34,
                    ),
                    end_line: 35,
                    end_character: Some(
                        5,
                    ),
                    kind: Some(
                        Region,
                    ),
                    collapsed_text: None,
                },
                FoldingRange {
                    start_line: 0,
                    start_character: None,
                    end_line: 0,
                    end_character: None,
                    kind: Some(
                        Imports,
                    ),
                    collapsed_text: None,
                },
            ]
        "]];
        expected.assert_debug_eq(&out);
    }
}
