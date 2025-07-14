//! Parsing functions for record
use crate::{
    class::parse_class_block,
    error::{AstError, assert_token},
    lexer::{PositionToken, Token},
    parse_implements, parse_jtype, parse_name, parse_superclass,
    types::{
        AstAnnotated, AstAvailability, AstRange, AstRecord, AstRecordEntries, AstRecordEntry,
        AstThing,
    },
};

/// `Name { ... }`
pub fn parse_record(
    tokens: &[PositionToken],
    pos: usize,
    avaliability: AstAvailability,
    annotated: Vec<AstAnnotated>,
) -> Result<(AstThing, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (name, pos) = parse_name(tokens, pos)?;
    let (record_entries, pos) = parse_record_entires(tokens, pos)?;
    let (implements, pos) = parse_implements(tokens, pos)?;
    let (superclass, pos) = parse_superclass(tokens, pos)?;
    let (block, pos) = parse_class_block(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstThing::Record(AstRecord {
            range: AstRange::from_position_token(start, end),
            avaliability,
            annotated,
            name,
            record_entries,
            superclass,
            implements,
            block,
        }),
        pos,
    ))
}
/// `(String a, short b)`
pub fn parse_record_entires(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstRecordEntries, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = assert_token(tokens, pos, Token::LeftParen)?;
    let mut entries = vec![];
    while let Ok((jtype, npos)) = parse_jtype(tokens, pos) {
        pos = npos;

        if let Ok((name, npos)) = parse_name(tokens, pos) {
            pos = npos;
            entries.push(AstRecordEntry { jtype, name });
            if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
                pos = npos;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstRecordEntries {
            range: AstRange::from_position_token(start, end),
            entries,
        },
        pos,
    ))
}
