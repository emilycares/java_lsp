//! Parsing functions for enum
use my_string::NuVec;

use crate::{
    class::{
        parse_class_constructor, parse_class_method, parse_class_variable,
        parse_implemnets_extends_permits, parse_static_block,
    },
    error::{AstError, GetStartEnd, assert_semicolon, assert_token},
    lexer::{PositionToken, Token},
    parse_annotated_list, parse_expression_parameters, parse_identifier, parse_name, parse_thing,
    types::{
        AstAnnotated, AstAvailability, AstClassConstructor, AstClassMethod, AstClassVariable,
        AstEnumeration, AstEnumerationVariant, AstRange, AstStaticBlock, AstThing,
        AstThingAttributes,
    },
};

/// `AAA { ... }`
pub fn parse_enumeration(
    tokens: &[PositionToken],
    pos: usize,
    availability: AstAvailability,
    attributes: AstThingAttributes,
    annotated: Vec<AstAnnotated>,
    start: &PositionToken,
) -> Result<(AstThing, usize), AstError> {
    let (name, pos) = parse_identifier(tokens, pos)?;
    let (superclass, implements, permits, pos) = parse_implemnets_extends_permits(tokens, pos)?;
    let mut errors = [const { None }; 2];
    let pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut pos = pos;
    let mut variants = vec![];
    let mut methods = vec![];
    let mut variables = vec![];
    let mut constructors = vec![];
    let mut static_blocks = vec![];
    let mut inner = vec![];
    let mut end_reached = false;
    loop {
        errors.fill(None);
        match parse_enum_variant(tokens, pos) {
            Ok((variant, npos)) => {
                variants.push(variant);
                pos = npos;
            }
            Err(e) => {
                errors[0] = Some((NuVec::new_static(b"enum_variant"), e));
            }
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            pos = npos;
            end_reached = true;
            break;
        } else if let Ok(npos) = assert_token(tokens, pos, Token::Semicolon) {
            pos = npos;
            break;
        } else if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }
        match parse_enum_members(
            tokens,
            pos,
            true,
            &mut methods,
            &mut variables,
            &mut constructors,
            &mut static_blocks,
            &mut inner,
        ) {
            Ok(npos) => {
                if let Ok(npos) = assert_token(tokens, npos, Token::Semicolon) {
                    pos = npos;
                    break;
                } else if let Ok(npos) = assert_token(tokens, npos, Token::Comma) {
                    pos = npos;
                    continue;
                }
                pos = npos;
                continue;
            }
            Err(e) => errors[1] = Some((NuVec::new_static(b"enum_members"), e)),
        }
        return Err(AstError::AllChildrenFailed2 {
            parent: NuVec::new_static(b"enum_variant"),
            errors: Box::new(errors),
        });
    }
    if !end_reached {
        match parse_enum_members(
            tokens,
            pos,
            false,
            &mut methods,
            &mut variables,
            &mut constructors,
            &mut static_blocks,
            &mut inner,
        ) {
            Ok(npos) => {
                pos = npos;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    if !end_reached && let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
        pos = npos;
    }
    let pos = assert_semicolon(tokens, pos)?;
    let end = tokens.end(pos)?;
    Ok((
        AstThing::Enumeration(AstEnumeration {
            range: AstRange::from_position_token(start, end),
            availability,
            attributes,
            annotated,
            name,
            superclass,
            implements,
            permits,
            variants,
            methods,
            constructors,
            variables,
            static_blocks,
            inner,
        }),
        pos,
    ))
}
#[allow(clippy::too_many_arguments)]
fn parse_enum_members(
    tokens: &[PositionToken],
    pos: usize,
    braces: bool,
    methods: &mut Vec<AstClassMethod>,
    variables: &mut Vec<AstClassVariable>,
    constructors: &mut Vec<AstClassConstructor>,
    static_blocks: &mut Vec<AstStaticBlock>,
    inner: &mut Vec<AstThing>,
) -> Result<usize, AstError> {
    let mut pos = pos;
    if braces {
        let npos = assert_token(tokens, pos, Token::LeftParenCurly)?;
        pos = npos;
    }
    let mut errors = [const { None }; 5];
    loop {
        errors.fill(None);
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            if braces {
                pos = npos;
            }
            break;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Semicolon) {
            pos = npos;
        }
        match parse_class_method(tokens, pos) {
            Ok((method, npos)) => {
                methods.push(method);
                pos = npos;
                continue;
            }
            Err(e) => {
                errors[0] = Some((NuVec::new_static(b"enum_method"), e));
            }
        }
        match parse_class_constructor(tokens, pos) {
            Ok((constructor, npos)) => {
                constructors.push(constructor);
                pos = npos;
                continue;
            }
            Err(e) => {
                errors[1] = Some((NuVec::new_static(b"enum_constructor"), e));
            }
        }
        match parse_class_variable(tokens, pos) {
            Ok((vars, npos)) => {
                variables.extend(vars);
                pos = npos;
                continue;
            }
            Err(e) => {
                errors[2] = Some((NuVec::new_static(b"enum_variable"), e));
            }
        }
        match parse_static_block(tokens, pos) {
            Ok((static_block, npos)) => {
                pos = npos;
                static_blocks.push(static_block);
                continue;
            }
            Err(e) => {
                errors[3] = Some((NuVec::new_static(b"static block"), e));
            }
        }
        match parse_thing(tokens, pos) {
            Ok((thing, npos)) => {
                pos = npos;
                inner.push(thing);
                continue;
            }
            Err(e) => {
                errors[4] = Some((NuVec::new_static(b"thing"), e));
            }
        }
        return Err(AstError::AllChildrenFailed5 {
            parent: NuVec::new_static(b"enum"),
            errors: Box::new(errors),
        });
    }
    Ok(pos)
}
/// `A`
/// `A("a")`
pub fn parse_enum_variant(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstEnumerationVariant, usize), AstError> {
    let start = tokens.start(pos)?;
    let mut annotated = Vec::new();
    let pos = parse_annotated_list(tokens, pos, &mut annotated)?;
    let (name, mut pos) = parse_name(tokens, pos)?;
    let mut parameters = vec![];

    if assert_token(tokens, pos, Token::LeftParen).is_ok() {
        let (p, npos) = parse_expression_parameters(tokens, pos)?;
        parameters = p;
        pos = npos;
    }
    let end = tokens.end(pos)?;
    Ok((
        AstEnumerationVariant {
            range: AstRange::from_position_token(start, end),
            annotated,
            name,
            parameters,
        },
        pos,
    ))
}
