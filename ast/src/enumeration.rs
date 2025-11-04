//! Parsing functions for enum
use crate::{
    class::{parse_class_constructor, parse_class_method, parse_class_variable},
    error::{AstError, assert_token},
    lexer::{PositionToken, Token},
    parse_expression_parameters, parse_identifier,
    types::{
        AstAnnotated, AstAvailability, AstEnumerationVariant, AstRange, AstThing,
        AstThingAttributes,
    },
};

/// `AAA { ... }`
pub fn parse_enumeration(
    tokens: &[PositionToken],
    pos: usize,
    avaliability: AstAvailability,
    attributes: AstThingAttributes,
    annotated: Vec<AstAnnotated>,
) -> Result<(AstThing, usize), AstError> {
    let (name, pos) = parse_identifier(tokens, pos)?;
    let mut errors = vec![];
    let pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut pos = pos;
    let mut variants = vec![];
    let mut methods = vec![];
    let mut variables = vec![];
    let mut constructors = vec![];
    let mut end_reached = false;
    loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::Semicolon) {
            pos = npos;
            break;
        };
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            pos = npos;
            end_reached = true;
            break;
        };
        if let Ok((variant, npos)) = parse_enum_variant(tokens, pos) {
            variants.push(variant);
            pos = npos;
            continue;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
        }
    }
    if !end_reached {
        loop {
            errors.clear();
            if tokens.get(pos).is_none() {
                break;
            }
            if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
                pos = npos;
                break;
            };
            match parse_class_method(tokens, pos) {
                Ok((method, npos)) => {
                    methods.push(method);
                    pos = npos;
                    continue;
                }
                Err(e) => {
                    errors.push(("enum_method".into(), e));
                }
            }
            match parse_class_constructor(tokens, pos) {
                Ok((constructor, npos)) => {
                    constructors.push(constructor);
                    pos = npos;
                    continue;
                }
                Err(e) => {
                    errors.push(("enum_constructor".into(), e));
                }
            }
            match parse_class_variable(tokens, pos) {
                Ok((variable, npos)) => {
                    variables.push(variable);
                    pos = npos;
                    continue;
                }
                Err(e) => {
                    errors.push(("enum_variable".into(), e));
                }
            }
            return Err(AstError::AllChildrenFailed {
                parent: "enum".into(),
                errors,
            });
        }
    }
    Ok((
        AstThing::Enumeration(crate::types::AstEnumeration {
            avaliability,
            attributes,
            annotated,
            name,
            variants,
            methods,
            constructors,
            variables,
        }),
        pos,
    ))
}
/// `A`
/// `A("a")`
pub fn parse_enum_variant(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstEnumerationVariant, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (name, mut pos) = parse_identifier(tokens, pos)?;
    let mut parameters = vec![];
    if let Ok((p, npos)) = parse_expression_parameters(tokens, pos) {
        parameters = p;
        pos = npos;
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstEnumerationVariant {
            range: AstRange::from_position_token(start, end),
            name,
            parameters,
        },
        pos,
    ))
}
