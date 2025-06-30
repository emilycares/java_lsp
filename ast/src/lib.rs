use error::{AstError, ExpectedToken, InvalidToken, assert_token};
use lexer::{PositionToken, Token};
use skip_helper::skip;
use types::{
    AstAvailability, AstClass, AstClassVariable, AstFile, AstIdentifier, AstInterface, AstJType,
    AstJTypeKind, AstRange, AstSuperClass, AstThing, AstValue,
};

pub mod error;
pub mod lexer;
pub mod skip_helper;
pub mod types;

pub fn parse_file(tokens: &[PositionToken], pos: usize) -> Result<AstFile, AstError> {
    let (package_name, pos) = parse_package(tokens, pos)?;
    let (imports, pos) = parse_imports(tokens, pos)?;
    let (thing, _pos) = parse_thing(tokens, pos)?;

    Ok(AstFile {
        package: package_name,
        imports,
        thing,
    })
}

/// package ch.emilycares;
fn parse_package(tokens: &[PositionToken], pos: usize) -> Result<(AstIdentifier, usize), AstError> {
    let pos = assert_token(tokens, pos, Token::Package)?;
    let (package_name, pos) = parse_identifier(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    Ok((package_name, pos))
}
///  import java.io.IOException;
///  import java.net.Socket;
fn parse_imports(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstIdentifier>, usize), AstError> {
    let mut pos = pos;
    let mut imports = vec![];

    while let Ok((import, new_pos)) = parse_import(tokens, pos) {
        pos = new_pos;
        imports.push(import);
    }

    Ok((imports, pos))
}

///  import java.io.IOException;
fn parse_import(tokens: &[PositionToken], pos: usize) -> Result<(AstIdentifier, usize), AstError> {
    let pos = assert_token(tokens, pos, Token::Import)?;
    let (ident, pos) = parse_identifier(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    Ok((ident, pos))
}

///  public class Everything { ...
///  public interface Constants { ...
fn parse_thing(tokens: &[PositionToken], pos: usize) -> Result<(AstThing, usize), AstError> {
    let (avaliability, pos) = parse_avaliability(tokens, pos)?;
    let pos = skip(tokens, pos)?;
    match tokens.get(pos) {
        Some(t) => match t {
            PositionToken {
                token: Token::Class,
                line: _,
                col: _,
            } => parse_class(tokens, pos + 1, avaliability),
            PositionToken {
                token: Token::Interface,
                line: _,
                col: _,
            } => Ok((AstThing::Interface(AstInterface { avaliability }), pos)),
            found => Err(AstError::ExpectedToken(ExpectedToken::from(
                found,
                Token::Class,
            ))),
        },
        None => Err(AstError::eof()),
    }
}

fn parse_class(
    tokens: &[PositionToken],
    pos: usize,
    avaliability: AstAvailability,
) -> Result<(AstThing, usize), AstError> {
    let (name, pos) = parse_identifier(tokens, pos)?;
    let (superclass, pos) = parse_superclass(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut variables = vec![];
    let mut pos = pos;
    loop {
        let Ok(npos) = skip(tokens, pos) else {
            break;
        };
        pos = npos;
        if tokens.get(pos).is_none() {
            break;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::LeftParenCurly) {
            pos = npos;
            break;
        };
        match parse_class_variable(tokens, pos) {
            Ok((variable, npos)) => {
                pos = npos;
                variables.push(variable);
            }
            Err(_e) => {
                // dbg!(e);
            }
        }
        pos += 1;
    }

    let class = AstClass {
        avaliability,
        name,
        superclass,
        variables,
    };
    Ok((AstThing::Class(class), pos))
}

fn parse_value(tokens: &[PositionToken], pos: usize) -> Result<(AstValue, usize), AstError> {
    parse_value_new_class(tokens, pos)
}
fn parse_value_parameters(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstValue>, usize), AstError> {
    let mut pos = assert_token(tokens, pos, Token::LeftParen)?;
    let mut out = vec![];
    loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParen) {
            pos = npos;
            break;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }
        let (value, npos) = parse_value(tokens, pos)?;
        pos = npos;
        out.push(value);
    }
    Ok((out, pos))
}
fn parse_value_new_class(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValue, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::New)?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (parameters, pos) = parse_value_parameters(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstValue::NewClass(types::AstValueNewClass {
            range: AstRange {
                start: start.start_point(),
                end: end.end_point(),
            },
            jtype,
            parameters,
        }),
        pos,
    ))
}

fn parse_class_variable(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstClassVariable, usize), AstError> {
    let pos = skip(tokens, pos)?;
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
    let pos = skip(tokens, pos)?;
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

fn parse_name(tokens: &[PositionToken], pos: usize) -> Result<(AstIdentifier, usize), AstError> {
    let pos = skip(tokens, pos)?;
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut ident = String::new();
    loop {
        pos = skip(tokens, pos)?;
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match &t.token {
            Token::Identifier(id) => {
                ident.push_str(id);
                pos += 1;
            }
            _ => break,
        }
    }
    if ident.is_empty() {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        return Err(AstError::IdentifierEmpty(InvalidToken::from(t)));
    }
    let pos = skip(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstIdentifier {
            range: AstRange::from_position_token(start, end),
            value: ident.to_string(),
        },
        pos,
    ))
}

fn parse_identifier(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    let pos = skip(tokens, pos)?;
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut ident = String::new();
    loop {
        pos = skip(tokens, pos)?;
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match &t.token {
            Token::Identifier(id) => {
                ident.push_str(id);
                pos += 1;
            }
            Token::Dot => {
                ident.push('.');
                pos += 1;
            }
            Token::Star => {
                ident.push('*');
                pos += 1;
            }
            _ => break,
        }
    }
    if ident.is_empty() {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        return Err(AstError::IdentifierEmpty(InvalidToken::from(t)));
    }
    let pos = skip(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstIdentifier {
            range: AstRange::from_position_token(start, end),
            value: ident.to_string(),
        },
        pos,
    ))
}

fn parse_superclass(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSuperClass, usize), AstError> {
    let pos = skip(tokens, pos)?;
    let Ok(pos) = assert_token(tokens, pos, Token::Extends) else {
        return Ok((AstSuperClass::None, pos));
    };
    // let (ident, pos) = parse_identifier(tokens, pos)?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let sp = match jtype.value {
        AstJTypeKind::Class(c) | AstJTypeKind::Generic(c, _) => AstSuperClass::Name(c),
        _ => AstSuperClass::None,
    };

    Ok((sp, pos))
}

fn parse_jtype(tokens: &[PositionToken], pos: usize) -> Result<(AstJType, usize), AstError> {
    let out_pos = pos + 1;
    let pos = skip(tokens, pos)?;
    let current = tokens.get(pos).ok_or(AstError::eof())?;
    match &current.token {
        Token::Int => Ok((
            AstJType {
                value: AstJTypeKind::Int,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Long => Ok((
            AstJType {
                value: AstJTypeKind::Long,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Short => Ok((
            AstJType {
                value: AstJTypeKind::Short,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Byte => Ok((
            AstJType {
                value: AstJTypeKind::Byte,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Char => Ok((
            AstJType {
                value: AstJTypeKind::Char,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Double => Ok((
            AstJType {
                value: AstJTypeKind::Double,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Float => Ok((
            AstJType {
                value: AstJTypeKind::Float,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Boolean => Ok((
            AstJType {
                value: AstJTypeKind::Boolean,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Void => Ok((
            AstJType {
                value: AstJTypeKind::Void,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::QuestionMark => Ok((
            AstJType {
                value: AstJTypeKind::Wildcard,
                range: AstRange::from_position_token(current, current),
            },
            out_pos,
        )),
        Token::Identifier(ident) => {
            let pos = skip(tokens, pos + 1)?;
            let peek = tokens.get(pos);
            let range = AstRange::from_position_token(current, current);
            let ident = AstIdentifier {
                value: ident.to_string(),
                range: range.clone(),
            };
            match peek {
                Some(PositionToken {
                    token,
                    line: _,
                    col: _,
                }) => match token {
                    Token::Lt => {
                        let out_pos;
                        let mut args = vec![];
                        let mut pos = pos + 1;
                        loop {
                            // If there are no type arguments
                            if let Ok(npos) = assert_token(tokens, pos, Token::Gt) {
                                out_pos = npos;
                                break;
                            }
                            let (jtype, npos) = parse_jtype(tokens, pos)?;
                            pos = npos;
                            args.push(jtype);
                            if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
                                pos = npos;
                                continue;
                            }
                            if let Ok(npos) = assert_token(tokens, pos, Token::Gt) {
                                out_pos = npos;
                                break;
                            }
                            pos += 1;
                        }
                        let end = tokens.get(pos).ok_or(AstError::eof())?;
                        Ok((
                            AstJType {
                                value: AstJTypeKind::Generic(ident, args),
                                range: AstRange::from_position_token(current, end),
                            },
                            out_pos,
                        ))
                    }
                    _ => Ok((
                        AstJType {
                            value: AstJTypeKind::Class(ident),
                            range,
                        },
                        out_pos,
                    )),
                },
                None => Err(AstError::eof()),
            }
        }
        found => {
            let point = current.start_point();
            Err(AstError::InvalidJtype(InvalidToken::from(&PositionToken {
                token: found.to_owned(),
                line: point.line,
                col: point.col,
            })))
        }
    }
}

fn parse_avaliability(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstAvailability, usize), AstError> {
    let pos = skip(tokens, pos)?;
    let Some(token) = tokens.get(pos) else {
        return Err(AstError::eof());
    };
    match token {
        PositionToken {
            token: Token::Public,
            line: _,
            col: _,
        } => Ok((AstAvailability::Public, pos + 1)),
        PositionToken {
            token: Token::Private,
            line: _,
            col: _,
        } => Ok((AstAvailability::Private, pos + 1)),
        PositionToken {
            token: Token::Protected,
            line: _,
            col: _,
        } => Ok((AstAvailability::Protected, pos + 1)),
        _ => Ok((AstAvailability::Private, pos)),
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{error::PrintErr, lexer, parse_file};

    #[test]
    fn everything() {
        let content = include_str!("../../parser/test/Everything.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens, 0);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed);
    }

    #[test]
    fn skip_comments() {
        let content = include_str!("../test/FullOffComments.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens, 0);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed);
    }

    #[test]
    fn locale_variable_table() {
        let content = include_str!("../../parser/test/LocalVariableTable.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens, 0);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed);
    }

    #[test]
    fn superee() {
        let content = include_str!("../../parser/test/Super.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens, 0);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed);
    }

    #[test]
    fn constants() {
        let content = include_str!("../../parser/test/Constants.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens, 0);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed);
    }
}
