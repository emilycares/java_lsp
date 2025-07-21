use crate::{
    error::{AstError, assert_token},
    lexer::{PositionToken, Token},
    parse_identifier, parse_jtype, parse_name, parse_value,
    types::{AstAnnotation, AstAnnotationField, AstAvailability, AstRange, AstThing},
};

pub fn parse_annotation(
    tokens: &[PositionToken],
    pos: usize,
    avaliability: AstAvailability,
) -> Result<(AstThing, usize), AstError> {
    let pos = assert_token(tokens, pos, Token::Interface)?;
    let (name, pos) = parse_identifier(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut pos = pos;
    let mut errors = vec![];
    let mut fields = vec![];
    loop {
        errors.clear();
        if tokens.get(pos).is_none() {
            break;
        }
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
        return Err(AstError::AllChildrenFailed {
            parent: "annotation".into(),
            errors,
        });
    }
    Ok((
        AstThing::Annotation(AstAnnotation {
            avaliability,
            name,
            fields,
        }),
        pos,
    ))
}

pub fn parse_annotation_field(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstAnnotationField, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let pos = assert_token(tokens, pos, Token::Default)?;
    let (value, pos) = parse_value(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
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
