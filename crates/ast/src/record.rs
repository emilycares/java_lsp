//! Parsing functions for record
use crate::{
    class::parse_class_block,
    error::{AstError, GetStartEnd, assert_token},
    lexer::{PositionToken, Token},
    parse_annotated_list, parse_implements, parse_jtype, parse_name, parse_superclass,
    parse_type_parameters, parse_variadic,
    types::{
        AstAnnotated, AstAvailability, AstRange, AstRecord, AstRecordEntries, AstRecordEntry,
        AstThing, AstThingAttributes,
    },
};

/// `Name { ... }`
pub fn parse_record(
    tokens: &[PositionToken],
    pos: usize,
    avaliability: AstAvailability,
    attributes: AstThingAttributes,
    annotated: Vec<AstAnnotated>,
    start: &PositionToken,
) -> Result<(AstThing, usize), AstError> {
    let (name, mut pos) = parse_name(tokens, pos)?;
    let mut type_parameters = None;
    if let Ok((type_params, npos)) = parse_type_parameters(tokens, pos) {
        type_parameters = Some(type_params);
        pos = npos;
    }
    let (record_entries, pos) = parse_record_entires(tokens, pos)?;
    let (implements, pos) = parse_implements(tokens, pos)?;
    let (superclass, pos) = parse_superclass(tokens, pos)?;
    let (block, pos) = parse_class_block(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstThing::Record(AstRecord {
            range: AstRange::from_position_token(start, end),
            availability: avaliability,
            attributes,
            annotated,
            name,
            type_parameters,
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
    let start = tokens.start(pos)?;
    let mut pos = assert_token(tokens, pos, Token::LeftParen)?;
    let mut entries = vec![];
    while let Ok((entry, npos)) = parse_record_entry(tokens, pos) {
        pos = npos;
        entries.push(entry);

        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
        } else {
            break;
        }
    }
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let end = tokens.end(pos)?;
    Ok((
        AstRecordEntries {
            range: AstRange::from_position_token(start, end),
            entries,
        },
        pos,
    ))
}

fn parse_record_entry(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstRecordEntry, usize), AstError> {
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let (jtype, mut pos) = parse_jtype(tokens, pos)?;
    let mut variadic = false;
    if let Ok(npos) = parse_variadic(tokens, pos) {
        variadic = true;
        pos = npos;
    }
    let (name, pos) = parse_name(tokens, pos)?;
    Ok((
        AstRecordEntry {
            annotated,
            jtype,
            variadic,
            name,
        },
        pos,
    ))
}
