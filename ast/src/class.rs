//! Parsing functions for class
use crate::{
    error::{AstError, assert_token},
    lexer::{PositionToken, Token},
    parse_annotated_list, parse_avaliability, parse_block, parse_constructor_header,
    parse_expression, parse_implements, parse_jtype, parse_method_header, parse_name,
    parse_superclass, parse_thing, parse_type_parameters,
    types::{
        AstAnnotated, AstAvailability, AstClass, AstClassBlock, AstClassConstructor,
        AstClassMethod, AstClassVariable, AstRange, AstStaticBlock, AstStaticFinal, AstSuperClass,
        AstThing,
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
    let (name, pos) = parse_name(tokens, pos)?;
    let mut type_parameters = None;
    let mut pos = pos;
    if let Ok((type_params, npos)) = parse_type_parameters(tokens, pos) {
        type_parameters = Some(type_params);
        pos = npos;
    };
    let mut superclass = AstSuperClass::None;
    let mut implements = vec![];
    loop {
        let token = tokens.get(pos).ok_or(AstError::eof())?;
        match token.token {
            Token::Extends => {
                let (s, npos) = parse_superclass(tokens, pos)?;
                superclass = s;
                pos = npos;
                continue;
            }
            Token::Implements => {
                let (i, npos) = parse_implements(tokens, pos)?;
                implements = i;
                pos = npos;
                continue;
            }
            _ => break,
        }
    }
    let (block, pos) = parse_class_block(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

    Ok((
        AstThing::Class(AstClass {
            range: AstRange::from_position_token(start, end),
            avaliability,
            annotated,
            name,
            type_parameters,
            superclass,
            implements,
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
    let mut static_blocks = vec![];
    let mut variables = vec![];
    let mut methods = vec![];
    let mut constructors = vec![];
    let mut inner = vec![];
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
        match parse_static_block(tokens, pos) {
            Ok((static_block, npos)) => {
                pos = npos;
                static_blocks.push(static_block);
                continue;
            }
            Err(e) => {
                errors.push(("static block".into(), e));
            }
        }
        match parse_thing(tokens, pos) {
            Ok((thing, npos)) => {
                pos = npos;
                inner.push(thing);
                continue;
            }
            Err(e) => {
                errors.push(("class thing".into(), e));
            }
        }
        return Err(AstError::AllChildrenFailed {
            parent: "class".into(),
            errors,
        });
    }
    Ok((
        AstClassBlock {
            static_blocks,
            variables,
            methods,
            constructors,
            inner,
        },
        pos,
    ))
}

fn parse_static_block(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstStaticBlock, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Static)?;
    let (block, pos) = parse_block(tokens, pos)?;

    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstStaticBlock {
            range: AstRange::from_position_token(start, end),
            block,
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
    let (header, pos) = parse_constructor_header(tokens, pos, AstAvailability::Public)?;
    let (block, pos) = parse_block(tokens, pos)?;

    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstClassConstructor {
            header,
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
    loop {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match t.token {
            Token::Static => static_final |= AstStaticFinal::Static,
            Token::Final => static_final |= AstStaticFinal::Final,
            Token::Volatile => static_final |= AstStaticFinal::Volatile,
            Token::Transient => static_final |= AstStaticFinal::Transient,
            _ => break,
        }
        pos += 1;
    }
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let mut names = vec![];
    let (name, pos) = parse_name(tokens, pos)?;
    names.push(name);
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
        let (name, npos) = parse_name(tokens, npos)?;
        names.push(name);
        pos = npos;
    }
    let mut expression = None;
    if let Ok(npos) = assert_token(tokens, pos, Token::Equal) {
        let (aexpr, npos) = parse_expression(tokens, npos)?;
        pos = npos;
        expression = Some(aexpr);
    }
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

    Ok((
        AstClassVariable {
            avaliability,
            annotated,
            names,
            static_final,
            jtype,
            expression,
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}
/// `public String getTag() { ... }`
/// `public String getTag();`
pub fn parse_class_method(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstClassMethod, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (header, pos) = parse_method_header(tokens, pos, AstAvailability::Protected)?;
    let mut block = None;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Semicolon) {
        pos = npos;
    } else {
        let (b, npos) = parse_block(tokens, pos)?;
        pos = npos;
        block = Some(b);
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstClassMethod {
            range: AstRange::from_position_token(start, end),
            header,
            block,
        },
        pos,
    ))
}
