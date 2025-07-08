use crate::{
    error::{AstError, assert_token},
    lexer::{PositionToken, Token},
    parse_avaliability, parse_extends, parse_identifier, parse_jtype, parse_method_header,
    parse_name, parse_type_parameters, parse_value,
    types::{
        AstAvailability, AstInterface, AstInterfaceConstant, AstInterfaceMethod, AstRange, AstThing,
    },
};

pub fn parse_interface(
    tokens: &[PositionToken],
    pos: usize,
    avaliability: AstAvailability,
) -> Result<(AstThing, usize), AstError> {
    let (name, pos) = parse_identifier(tokens, pos)?;
    let mut pos = pos;
    let mut type_parameters = None;
    let mut extends = None;
    if let Ok((tp, npos)) = parse_type_parameters(tokens, pos) {
        type_parameters = Some(tp);
        pos = npos;
    }
    if let Ok((tp, npos)) = parse_extends(tokens, pos) {
        extends = Some(tp);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut errors = vec![];
    let mut constants = vec![];
    let mut methods = vec![];
    let mut pos = pos;
    loop {
        errors.clear();
        if tokens.get(pos).is_none() {
            break;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            pos = npos;
            break;
        };
        match parse_interface_constant(tokens, pos) {
            Ok((constant, npos)) => {
                constants.push(constant);
                pos = npos;
                continue;
            }
            Err(e) => {
                errors.push(e);
            }
        }
        match parse_interface_method(tokens, pos) {
            Ok((method, npos)) => {
                methods.push(method);
                pos = npos;
                continue;
            }
            Err(e) => {
                errors.push(e);
            }
        }
        pos += 1;
    }

    Ok((
        AstThing::Interface(AstInterface {
            avaliability,
            name,
            type_parameters,
            extends,
            constants,
            methods,
        }),
        pos,
    ))
}

pub fn parse_interface_constant(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstInterfaceConstant, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (avaliability, pos) = parse_avaliability(tokens, pos)?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Equal)?;
    let (value, pos) = parse_value(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstInterfaceConstant {
            range: AstRange::from_position_token(start, end),
            avaliability,
            name,
            jtype,
            value,
        },
        pos,
    ))
}

pub fn parse_interface_method(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstInterfaceMethod, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (header, pos) = parse_method_header(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstInterfaceMethod {
            range: AstRange::from_position_token(start, end),
            header,
        },
        pos,
    ))
}
