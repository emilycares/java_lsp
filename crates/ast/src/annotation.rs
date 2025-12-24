//! Parsing functions for defining annotation
use crate::{
    ExpressionOptions,
    error::{AstError, GetStartEnd, assert_token},
    lexer::{PositionToken, Token},
    parse_annotated_list, parse_expression, parse_jtype, parse_name, parse_name_single,
    parse_thing,
    types::{
        AstAnnotated, AstAnnotation, AstAnnotationField, AstAvailability, AstRange, AstThing,
        AstThingAttributes,
    },
};

/// `@interface Overwrite`
pub fn parse_annotation(
    tokens: &[PositionToken],
    pos: usize,
    availability: AstAvailability,
    attributes: AstThingAttributes,
    annotated: Vec<AstAnnotated>,
    start: &PositionToken,
) -> Result<(AstThing, usize), AstError> {
    let (name, pos) = parse_name_single(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut pos = pos;
    let mut errors = vec![];
    let mut fields = vec![];
    let mut inner = vec![];
    loop {
        errors.clear();
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            pos = npos;
            break;
        }
        match parse_annotation_field(tokens, pos) {
            Ok((field, npos)) => {
                fields.push(field);
                pos = npos;
                continue;
            }
            Err(e) => {
                errors.push(("annotation field".into(), e));
            }
        }
        match parse_thing(tokens, pos) {
            Ok((thing, npos)) => {
                pos = npos;
                inner.push(thing);
                continue;
            }
            Err(e) => {
                errors.push(("thing".into(), e));
            }
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
            availability,
            attributes,
            annotated,
            name,
            fields,
            inner,
        }),
        pos,
    ))
}

/// `String[] value()`;
/// `String[] value() default 1`;
pub fn parse_annotation_field(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstAnnotationField, usize), AstError> {
    let start = tokens.get(pos).ok_or_else(AstError::eof)?;
    let mut availability = AstAvailability::empty();
    let (mut annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut pos = pos;
    loop {
        let t = tokens.get(pos).ok_or_else(AstError::eof)?;
        match t.token {
            Token::Public => availability |= AstAvailability::Public,
            Token::Private => availability |= AstAvailability::Private,
            Token::Protected => availability |= AstAvailability::Protected,
            Token::Static => availability |= AstAvailability::Static,
            Token::Final => availability |= AstAvailability::Final,
            Token::At => {
                let (annotated_after, npos) = parse_annotated_list(tokens, pos)?;
                pos = npos;
                annotated.extend(annotated_after);
                continue;
            }
            _ => break,
        }
        pos += 1;
    }
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, mut pos) = parse_name(tokens, pos)?;
    if let Ok(npos) = assert_token(tokens, pos, Token::LeftParen) {
        let npos = assert_token(tokens, npos, Token::RightParen)?;
        pos = npos;
    }
    let mut expression = None;
    if let Ok(npos) = assert_token(tokens, pos, Token::Default) {
        let (e, npos) = parse_expression(tokens, npos, &ExpressionOptions::None)?;
        pos = npos;
        expression = Some(e);
    } else if let Ok(npos) = assert_token(tokens, pos, Token::Equal) {
        let (e, npos) = parse_expression(tokens, npos, &ExpressionOptions::None)?;
        pos = npos;
        expression = Some(e);
    }
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.end(pos)?;
    Ok((
        AstAnnotationField {
            range: AstRange::from_position_token(start, end),
            availability,
            annotated,
            jtype,
            name,
            expression,
        },
        pos,
    ))
}
