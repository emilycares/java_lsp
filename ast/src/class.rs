use crate::{
    error::{AstError, assert_token},
    lexer::{PositionToken, Token},
    parse_avaliability, parse_block, parse_identifier, parse_jtype, parse_method_header,
    parse_method_paramerters, parse_name, parse_superclass, parse_value,
    types::{
        AstAvailability, AstClass, AstClassConstructor, AstClassMethod, AstClassVariable, AstRange,
        AstThing,
    },
};

pub fn parse_class(
    tokens: &[PositionToken],
    pos: usize,
    avaliability: AstAvailability,
) -> Result<(AstThing, usize), AstError> {
    let (name, pos) = parse_identifier(tokens, pos)?;
    let (superclass, pos) = parse_superclass(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut variables = vec![];
    let mut methods = vec![];
    let mut constructors = vec![];
    let mut pos = pos;
    let mut errors = vec![];
    loop {
        errors.clear();
        if tokens.get(pos).is_none() {
            break;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            pos = npos;
            break;
        };
        match parse_class_variable(tokens, pos) {
            Ok((variable, npos)) => {
                pos = npos;
                variables.push(variable);
                continue;
            }
            Err(e) => {
                errors.push(("class variable", e));
            }
        }
        match parse_class_constructor(tokens, pos) {
            Ok((constructor, npos)) => {
                pos = npos;
                constructors.push(constructor);
                continue;
            }
            Err(e) => {
                errors.push(("class constructor", e));
            }
        }
        match parse_class_method(tokens, pos) {
            Ok((method, npos)) => {
                pos = npos;
                methods.push(method);
                continue;
            }
            Err(e) => {
                errors.push(("class method", e));
            }
        }
        return Err(AstError::AllChildrenFailed {
            parent: "class",
            errors,
        });
    }

    Ok((
        AstThing::Class(AstClass {
            avaliability,
            name,
            superclass,
            variables,
            methods,
            constructors,
        }),
        pos,
    ))
}
fn parse_class_constructor(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstClassConstructor, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (avaliability, pos) = parse_avaliability(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let (parameters, pos) = parse_method_paramerters(tokens, pos)?;
    let (block, pos) = parse_block(tokens, pos)?;

    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstClassConstructor {
            avaliability,
            name,
            parameters,
            block,
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}
fn parse_class_variable(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstClassVariable, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (avaliability, pos) = parse_avaliability(tokens, pos)?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let mut value = None;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Equal) {
        let (avalue, npos) = parse_value(tokens, npos)?;
        pos = npos;
        value = Some(avalue);
    }
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;

    Ok((
        AstClassVariable {
            avaliability,
            name,
            jtype,
            value,
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}
fn parse_class_method(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstClassMethod, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (header, pos) = parse_method_header(tokens, pos)?;
    let (block, pos) = parse_block(tokens, pos)?;

    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstClassMethod {
            header,
            block,
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}
