//! Parsing functions for class
use crate::{
    error::{AstError, assert_token},
    lexer::{PositionToken, Token},
    parse_annotated_list, parse_avaliability, parse_block, parse_identifier, parse_jtype,
    parse_method_header, parse_method_paramerters, parse_name, parse_name_single,
    parse_recursive_expression, parse_superclass,
    types::{
        AstAnnotated, AstAvailability, AstClass, AstClassBlock, AstClassConstructor,
        AstClassMethod, AstClassVariable, AstRange, AstStaticFinal, AstThing,
    },
};

/// `Name { ... }`
pub fn parse_class(
    tokens: &[PositionToken],
    pos: usize,
    avaliability: AstAvailability,
    annotated: Vec<AstAnnotated>,
) -> Result<(AstThing, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (name, pos) = parse_identifier(tokens, pos)?;
    let (superclass, pos) = parse_superclass(tokens, pos)?;
    let (block, pos) = parse_class_block(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

    Ok((
        AstThing::Class(AstClass {
            range: AstRange::from_position_token(start, end),
            avaliability,
            annotated,
            name,
            superclass,
            block,
        }),
        pos,
    ))
}

/// `{ ... }`
pub fn parse_class_block(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstClassBlock, usize), AstError> {
    let pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut variables = vec![];
    let mut methods = vec![];
    let mut constructors = vec![];
    let mut pos = pos;
    let mut errors = vec![];
    loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            pos = npos;
            break;
        };
        errors.clear();
        match parse_class_variable(tokens, pos) {
            Ok((variable, npos)) => {
                pos = npos;
                variables.push(variable);
                continue;
            }
            Err(e) => {
                errors.push(("class variable".into(), e));
            }
        }
        match parse_class_constructor(tokens, pos) {
            Ok((constructor, npos)) => {
                pos = npos;
                constructors.push(constructor);
                continue;
            }
            Err(e) => {
                errors.push(("class constructor".into(), e));
            }
        }
        match parse_class_method(tokens, pos) {
            Ok((method, npos)) => {
                pos = npos;
                methods.push(method);
                continue;
            }
            Err(e) => {
                errors.push(("class method".into(), e));
            }
        }
        return Err(AstError::AllChildrenFailed {
            parent: "class".into(),
            errors,
        });
    }
    Ok((
        AstClassBlock {
            variables,
            methods,
            constructors,
        },
        pos,
    ))
}
/// `private Variants(String tag) { ... }`
pub fn parse_class_constructor(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstClassConstructor, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let (avaliability, pos) = parse_avaliability(tokens, pos)?;
    let (name, pos) = parse_name_single(tokens, pos)?;
    let (parameters, pos) = parse_method_paramerters(tokens, pos)?;
    let (block, pos) = parse_block(tokens, pos)?;

    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstClassConstructor {
            avaliability,
            annotated,
            name,
            parameters,
            block,
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}
/// `private final String tag;`
pub fn parse_class_variable(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstClassVariable, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let (avaliability, pos) = parse_avaliability(tokens, pos)?;
    let mut pos = pos;
    let mut static_final = AstStaticFinal::None;
    if let Ok(npos) = assert_token(tokens, pos, Token::Static) {
        pos = npos;
        static_final = AstStaticFinal::Static;
    }
    if let Ok(npos) = assert_token(tokens, pos, Token::Final) {
        pos = npos;
        if static_final == AstStaticFinal::Static {
            static_final = AstStaticFinal::StaticFinal;
        } else {
            static_final = AstStaticFinal::Final;
        }
    }
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let mut expression = None;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Equal) {
        let (aexpr, npos) = parse_recursive_expression(tokens, npos)?;
        pos = npos;
        expression = Some(aexpr);
    }
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

    Ok((
        AstClassVariable {
            avaliability,
            annotated,
            name,
            static_final,
            jtype,
            expression,
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}
/// `public String getTag() { ... }`
pub fn parse_class_method(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstClassMethod, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let (header, pos) = parse_method_header(tokens, pos, AstAvailability::Protected)?;
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstClassMethod {
            range: AstRange::from_position_token(start, end),
            annotated,
            header,
            block,
        },
        pos,
    ))
}
