//! Parsing functions for interface
use crate::{
    ExpressionOptions,
    error::{AstError, assert_token},
    lexer::{PositionToken, Token},
    parse_annotated_list, parse_avaliability, parse_block, parse_expression, parse_extends,
    parse_identifier, parse_jtype, parse_method_header, parse_name, parse_permits, parse_thing,
    parse_type_parameters,
    types::{
        AstAnnotated, AstAvailability, AstInterface, AstInterfaceConstant, AstInterfaceMethod,
        AstInterfaceMethodDefault, AstRange, AstStaticFinal, AstThing, AstThingAttributes,
    },
};

/// `Named { ... }`
pub fn parse_interface(
    tokens: &[PositionToken],
    pos: usize,
    avaliability: AstAvailability,
    attributes: AstThingAttributes,
    annotated: Vec<AstAnnotated>,
) -> Result<(AstThing, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (name, pos) = parse_identifier(tokens, pos)?;
    let mut pos = pos;
    let mut type_parameters = None;
    let mut extends = None;
    if let Ok((tp, npos)) = parse_type_parameters(tokens, pos) {
        type_parameters = Some(tp);
        pos = npos;
    }
    let mut permits = vec![];
    loop {
        let token = tokens.get(pos).ok_or(AstError::eof())?;
        match token.token {
            Token::Extends => {
                let (s, npos) = parse_extends(tokens, pos)?;
                extends = Some(s);
                pos = npos;
                continue;
            }
            Token::Permits => {
                let (i, npos) = parse_permits(tokens, pos)?;
                permits = i;
                pos = npos;
                continue;
            }
            _ => break,
        }
    }
    let pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut errors = vec![];
    let mut constants = vec![];
    let mut methods = vec![];
    let mut default_methods = vec![];
    let mut inner = vec![];
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
                errors.push(("interface_constant".into(), e));
            }
        }
        match parse_interface_method(tokens, pos) {
            Ok((method, npos)) => {
                methods.push(method);
                pos = npos;
                continue;
            }
            Err(e) => {
                errors.push(("interface_method".into(), e));
            }
        }
        match parse_interface_method_impl(tokens, pos) {
            Ok((method, npos)) => {
                default_methods.push(method);
                pos = npos;
                continue;
            }
            Err(e) => {
                errors.push(("interface_method_impl".into(), e));
            }
        }
        match parse_thing(tokens, pos) {
            Ok((thing, npos)) => {
                pos = npos;
                inner.push(thing);
                continue;
            }
            Err(e) => {
                errors.push(("interface thing".into(), e));
            }
        }
        return Err(AstError::AllChildrenFailed {
            parent: "interface".into(),
            errors,
        });
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

    Ok((
        AstThing::Interface(AstInterface {
            range: AstRange::from_position_token(start, end),
            attributes,
            avaliability,
            annotated,
            name,
            type_parameters,
            extends,
            permits,
            constants,
            methods,
            default_methods,
            inner,
        }),
        pos,
    ))
}

/// `public String CONSTANT_A = "A";`
pub fn parse_interface_constant(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstInterfaceConstant, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut static_final = AstStaticFinal::None;
    let (avaliability, pos) = parse_avaliability(tokens, pos)?;
    let mut pos = pos;
    loop {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match t.token {
            Token::Static => static_final |= AstStaticFinal::Static,
            Token::Final => static_final |= AstStaticFinal::Final,
            _ => break,
        }
        pos += 1;
    }
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Equal)?;
    let (expression, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstInterfaceConstant {
            range: AstRange::from_position_token(start, end),
            annotated,
            static_final,
            avaliability,
            name,
            jtype,
            expression,
        },
        pos,
    ))
}

/// `public static<A> A a(final A arg) {`
pub fn parse_interface_method(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstInterfaceMethod, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let (header, pos) = parse_method_header(tokens, pos, AstAvailability::Public)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstInterfaceMethod {
            range: AstRange::from_position_token(start, end),
            annotated,
            header,
        },
        pos,
    ))
}

/// ` default Stream<E> stream() { ... }`
pub fn parse_interface_method_impl(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstInterfaceMethodDefault, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut availability = AstAvailability::Public;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Default) {
        pos = npos;
    }
    if let Ok((nava, npos)) = parse_avaliability(tokens, pos) {
        pos = npos;
        availability = nava;
    }
    if let Ok(npos) = assert_token(tokens, pos, Token::Default) {
        pos = npos;
    }
    let (header, pos) = parse_method_header(tokens, pos, availability)?;
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstInterfaceMethodDefault {
            range: AstRange::from_position_token(start, end),
            annotated,
            header,
            block,
        },
        pos,
    ))
}
