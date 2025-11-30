//! Parsing functions for interface
use crate::{
    ExpressionOptions,
    error::{AstError, GetStartEnd, assert_token},
    lexer::{PositionToken, Token},
    parse_annotated_list, parse_array_type_on_name, parse_block, parse_expression, parse_extends,
    parse_identifier, parse_jtype, parse_method_header, parse_name, parse_permits, parse_thing,
    parse_type_parameters,
    types::{
        AstAnnotated, AstAvailability, AstInterface, AstInterfaceConstant, AstInterfaceMethod,
        AstInterfaceMethodDefault, AstJType, AstRange, AstThing, AstThingAttributes,
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
    let start = tokens.start(pos)?;
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
        let token = tokens.get(pos).ok_or_else(AstError::eof)?;
        match token.token {
            Token::Extends => {
                let (s, npos) = parse_extends(tokens, pos)?;
                extends = Some(s);
                pos = npos;
            }
            Token::Permits => {
                let (i, npos) = parse_permits(tokens, pos)?;
                permits = i;
                pos = npos;
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
        }
        match assert_token(tokens, pos, Token::Semicolon) {
            Ok(npos) => {
                pos = npos;
                continue;
            }
            Err(e) => {
                errors.push(("interface semicolon".into(), e));
            }
        }
        match parse_interface_constant(tokens, pos) {
            Ok((constant, npos)) => {
                constants.extend(constant);
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
    let end = tokens.end(pos)?;

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
) -> Result<(Vec<AstInterfaceConstant>, usize), AstError> {
    let start = tokens.start(pos)?;
    let (mut annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut avaliability = AstAvailability::empty();
    let mut pos = pos;
    loop {
        let t = tokens.get(pos).ok_or_else(AstError::eof)?;
        match t.token {
            Token::Public => avaliability |= AstAvailability::Public,
            Token::Private => avaliability |= AstAvailability::Private,
            Token::Protected => avaliability |= AstAvailability::Protected,
            Token::Static => avaliability |= AstAvailability::Static,
            Token::Final => avaliability |= AstAvailability::Final,
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
    let mut out = vec![];

    let (v, pos) = parse_interface_constant_base(
        tokens,
        start,
        &annotated,
        avaliability.clone(),
        &jtype,
        pos,
    )?;
    out.push(v);
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
        let (v, npos) = parse_interface_constant_base(
            tokens,
            start,
            &annotated,
            avaliability.clone(),
            &jtype,
            npos,
        )?;
        pos = npos;
        out.push(v);
    }

    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    Ok((out, pos))
}
fn parse_interface_constant_base(
    tokens: &[PositionToken],
    start: &PositionToken,
    annotated: &[AstAnnotated],
    avaliability: AstAvailability,
    jtype: &AstJType,
    pos: usize,
) -> Result<(AstInterfaceConstant, usize), AstError> {
    let mut jtype = jtype.clone();
    let (name, pos) = parse_name(tokens, pos)?;
    let pos = parse_array_type_on_name(tokens, pos, &mut jtype);
    if assert_token(tokens, pos, Token::Semicolon).is_ok() {
        let end = tokens.end(pos)?;
        return Ok((
            AstInterfaceConstant {
                range: AstRange::from_position_token(start, end),
                annotated: annotated.to_owned(),
                jtype,
                name,
                avaliability,
                expression: None,
            },
            pos,
        ));
    }
    let pos = assert_token(tokens, pos, Token::Equal)?;
    let (expression, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let end = tokens.end(pos)?;
    Ok((
        AstInterfaceConstant {
            range: AstRange::from_position_token(start, end),
            annotated: annotated.to_owned(),
            jtype,
            name,
            avaliability,
            expression: Some(expression),
        },
        pos,
    ))
}
/// `public static<A> A a(final A arg) {`
pub fn parse_interface_method(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstInterfaceMethod, usize), AstError> {
    let start = tokens.start(pos)?;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let (header, pos) = parse_method_header(tokens, pos, AstAvailability::Public)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.end(pos)?;
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
    let start = tokens.start(pos)?;
    let (mut annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut avaliability = AstAvailability::empty();
    let mut pos = pos;
    loop {
        let t = tokens.get(pos).ok_or_else(AstError::eof)?;
        match t.token {
            Token::Public => avaliability |= AstAvailability::Public,
            Token::Private => avaliability |= AstAvailability::Private,
            Token::Protected => avaliability |= AstAvailability::Protected,
            Token::Default => (),
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
    let (header, pos) = parse_method_header(tokens, pos, avaliability)?;
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.end(pos)?;
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
