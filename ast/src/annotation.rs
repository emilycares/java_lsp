//! Parsing functions for defining annotation
use crate::{
    error::{AstError, GetStartEnd, assert_token},
    lexer::{PositionToken, Token},
    parse_jtype, parse_name, parse_name_single, parse_value,
    types::{
        AstAnnotated, AstAnnotation, AstAnnotationField, AstAvailability, AstRange, AstThing,
        AstThingAttributes,
    },
};

/// @Overwride
pub fn parse_annotation(
    tokens: &[PositionToken],
    pos: usize,
    avaliability: AstAvailability,
    attributes: AstThingAttributes,
    annotated: Vec<AstAnnotated>,
) -> Result<(AstThing, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Interface)?;
    let (name, pos) = parse_name_single(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut pos = pos;
    let mut errors = vec![];
    let mut fields = vec![];
    let mut start_pos;
    loop {
        start_pos = pos;
        errors.clear();
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            pos = npos;
            break;
        };
        match parse_annotation_field(tokens, pos) {
            Ok((field, npos)) => {
                fields.push(field);
                pos = npos;
                continue;
            }
            Err(e) => {
                errors.push(("interface_constant".into(), e));
            }
        }
        if pos == start_pos {
            eprintln!("No annotation field was parsed: {:?}", tokens.get(pos));
            break;
        }
        return Err(AstError::AllChildrenFailed {
            parent: "annotation".into(),
            errors,
        });
    }
    let end = tokens.end(pos)?;
    Ok((
        AstThing::Annotation(AstAnnotation {
            range: AstRange::from_position_token(start, end),
            avaliability,
            attributes,
            annotated,
            name,
            fields,
        }),
        pos,
    ))
}

/// String[] value();
pub fn parse_annotation_field(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstAnnotationField, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let mut value = None;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Default) {
        let (v, npos) = parse_value(tokens, npos)?;
        pos = npos;
        value = Some(v);
    }
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.end(pos)?;
    Ok((
        AstAnnotationField {
            range: AstRange::from_position_token(start, end),
            jtype,
            name,
            value,
        },
        pos,
    ))
}
