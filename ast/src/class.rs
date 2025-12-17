//! Parsing functions for class
use crate::{
    ExpressionOptions,
    error::{AstError, GetStartEnd, assert_semicolon, assert_token},
    lexer::{PositionToken, Token},
    parse_annotated_list, parse_array_type_on_name, parse_block, parse_constructor_header,
    parse_expression, parse_implements, parse_jtype, parse_method_header, parse_name,
    parse_permits, parse_superclass, parse_thing, parse_type_parameters,
    types::{
        AstAnnotated, AstAvailability, AstClass, AstClassBlock, AstClassConstructor,
        AstClassMethod, AstClassVariable, AstJType, AstRange, AstStaticBlock, AstSuperClass,
        AstThing, AstThingAttributes, AstVolatileTransient,
    },
};

/// `Name { ... }`
pub fn parse_class(
    tokens: &[PositionToken],
    pos: usize,
    availability: AstAvailability,
    attributes: AstThingAttributes,
    annotated: Vec<AstAnnotated>,
    start: &PositionToken,
) -> Result<(AstThing, usize), AstError> {
    let (name, pos) = parse_name(tokens, pos)?;
    let mut pos = pos;
    let type_parameters = if let Ok((type_params, npos)) = parse_type_parameters(tokens, pos) {
        pos = npos;
        Some(type_params)
    } else {
        None
    };
    let (superclass, implements, permits, pos) = parse_implemnets_extends_permits(tokens, pos)?;
    let (block, pos) = parse_class_block(tokens, pos)?;
    let end = tokens.end(pos)?;

    Ok((
        AstThing::Class(AstClass {
            range: AstRange::from_position_token(start, end),
            availability,
            attributes,
            annotated,
            name,
            type_parameters,
            superclass,
            implements,
            permits,
            block,
        }),
        pos,
    ))
}

type ImplementsExtendsPermits = (Vec<AstSuperClass>, Vec<AstJType>, Vec<AstJType>, usize);

/// `implements Option`
pub fn parse_implemnets_extends_permits(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<ImplementsExtendsPermits, AstError> {
    let mut superclass = vec![];
    let mut implements = vec![];
    let mut permits = vec![];
    let mut pos = pos;
    loop {
        let token = tokens.get(pos).ok_or_else(AstError::eof)?;
        match token.token {
            Token::Extends => {
                let (s, npos) = parse_superclass(tokens, pos)?;
                superclass.extend(s);
                pos = npos;
            }
            Token::Implements => {
                let (i, npos) = parse_implements(tokens, pos)?;
                implements = i;
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
    Ok((superclass, implements, permits, pos))
}

/// `{ ... }`
pub fn parse_class_block(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstClassBlock, usize), AstError> {
    let pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut static_blocks = vec![];
    let mut blocks = vec![];
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
        }
        errors.clear();
        let current = tokens.start(pos)?;
        match &current.token {
            Token::Semicolon => {
                pos += 1;
                continue;
            }
            Token::Static => match parse_static_block(tokens, pos) {
                Ok((static_block, npos)) => {
                    pos = npos;
                    static_blocks.push(static_block);
                    continue;
                }
                Err(e) => {
                    errors.push(("static block".into(), e));
                }
            },
            Token::LeftParenCurly => match parse_block(tokens, pos) {
                Ok((block, npos)) => {
                    pos = npos;
                    blocks.push(block);
                    continue;
                }
                Err(e) => {
                    errors.push(("block".into(), e));
                }
            },
            _ => (),
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
        match parse_class_variable(tokens, pos) {
            Ok((vars, npos)) => {
                pos = npos;
                variables.extend(vars);
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
            static_blocks,
            inner,
            blocks,
        },
        pos,
    ))
}

/// `static { ... }`
pub fn parse_static_block(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstStaticBlock, usize), AstError> {
    let start = tokens.start(pos)?;
    let pos = assert_token(tokens, pos, Token::Static)?;
    let (block, pos) = parse_block(tokens, pos)?;

    let end = tokens.end(pos)?;
    Ok((
        AstStaticBlock {
            range: AstRange::from_position_token(start, end),
            block,
        },
        pos,
    ))
}
/// `private Variants { ... }`
/// `private Variants(String tag) { ... }`
pub fn parse_class_constructor(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstClassConstructor, usize), AstError> {
    let start = tokens.start(pos)?;
    let (header, pos) = parse_constructor_header(tokens, pos, AstAvailability::Public)?;
    let (block, pos) = parse_block(tokens, pos)?;

    let end = tokens.end(pos)?;
    let pos = assert_semicolon(tokens, pos)?;
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
) -> Result<(Vec<AstClassVariable>, usize), AstError> {
    let start = tokens.start(pos)?;
    let (mut annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut availability = AstAvailability::empty();
    let mut pos = pos;
    let mut volatile_transient = AstVolatileTransient::empty();
    loop {
        let t = tokens.get(pos).ok_or_else(AstError::eof)?;
        match t.token {
            Token::Public => availability |= AstAvailability::Public,
            Token::Private => availability |= AstAvailability::Private,
            Token::Protected => availability |= AstAvailability::Protected,
            Token::Static => availability |= AstAvailability::Static,
            Token::Final => availability |= AstAvailability::Final,
            Token::Volatile => volatile_transient |= AstVolatileTransient::Volatile,
            Token::Transient => volatile_transient |= AstVolatileTransient::Transient,
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
    let mut out = vec![];
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (v, pos) = parse_class_variable_base(
        tokens,
        start,
        &annotated,
        availability.clone(),
        volatile_transient.clone(),
        &jtype,
        pos,
    )?;
    out.push(v);
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
        let (v, npos) = parse_class_variable_base(
            tokens,
            start,
            &annotated,
            availability.clone(),
            volatile_transient.clone(),
            &jtype,
            npos,
        )?;
        pos = npos;
        out.push(v);
    }

    let pos = assert_token(tokens, pos, Token::Semicolon)?;

    Ok((out, pos))
}
fn parse_class_variable_base(
    tokens: &[PositionToken],
    start: &PositionToken,
    annotated: &[AstAnnotated],
    availability: AstAvailability,
    volatile_transient: AstVolatileTransient,
    jtype: &AstJType,
    pos: usize,
) -> Result<(AstClassVariable, usize), AstError> {
    let mut jtype = jtype.clone();
    let (name, pos) = parse_name(tokens, pos)?;
    let mut pos = parse_array_type_on_name(tokens, pos, &mut jtype);
    let expression = if let Ok(npos) = assert_token(tokens, pos, Token::Equal) {
        let (aexpression, npos) = parse_expression(tokens, npos, &ExpressionOptions::None)?;
        pos = npos;
        Some(aexpression)
    } else {
        None
    };
    let end = tokens.end(pos)?;
    Ok((
        AstClassVariable {
            range: AstRange::from_position_token(start, end),
            annotated: annotated.to_owned(),
            jtype,
            name,
            expression,
            availability,
            volatile_transient,
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
    let start = tokens.start(pos)?;
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
    let end = tokens.end(pos)?;
    Ok((
        AstClassMethod {
            range: AstRange::from_position_token(start, end),
            header,
            block,
        },
        pos,
    ))
}
