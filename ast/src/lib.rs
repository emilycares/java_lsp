use class::parse_class;
use error::{AstError, ExpectedToken, InvalidToken, assert_token};
use interface::parse_interface;
use lexer::{PositionToken, Token};
use smol_str::SmolStrBuilder;
use types::{
    AstAvailability, AstBlock, AstBlockEntry, AstBlockReturn, AstBlockVariable, AstExtends,
    AstFile, AstIdentifier, AstJType, AstJTypeKind, AstMethodHeader, AstMethodParamerter,
    AstMethodParamerters, AstNumber, AstRange, AstSuperClass, AstThing, AstThrowsDeclaration,
    AstTypeParameters, AstValue, AstValueEquasion, AstValueEquasionOperator, AstValueNuget,
};

pub mod class;
pub mod error;
pub mod interface;
pub mod lexer;
pub mod types;

pub fn parse_file(tokens: &[PositionToken]) -> Result<AstFile, AstError> {
    let (package_name, pos) = parse_package(tokens, 0)?;
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
            } => parse_interface(tokens, pos + 1, avaliability),
            found => Err(AstError::ExpectedToken(ExpectedToken::from(
                found,
                Token::Class,
            ))),
        },
        None => Err(AstError::eof()),
    }
}

fn parse_value(tokens: &[PositionToken], pos: usize) -> Result<(AstValue, usize), AstError> {
    let mut errors = vec![];
    match parse_value_new_class(tokens, pos) {
        Ok(v) => return Ok(v),
        Err(e) => errors.push(("value new class", e)),
    };
    match parse_value_equasion(tokens, pos) {
        Ok(v) => return Ok(v),
        Err(e) => errors.push(("value expression", e)),
    }
    match parse_value_nuget(tokens, pos) {
        Ok((nuget, pos)) => return Ok((AstValue::Nuget(nuget), pos)),
        Err(e) => errors.push(("value nuget", e)),
    }
    Err(AstError::AllChildrenFailed {
        parent: "value",
        errors,
    })
}
fn parse_value_equasion(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValue, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (lhs, pos) = parse_value_nuget(tokens, pos)?;
    let (operator, pos) = parse_value_operator(tokens, pos)?;
    let (rhs, pos) = parse_value_nuget(tokens, pos)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstValue::Equasion(AstValueEquasion {
            range: AstRange::from_position_token(start, end),
            lhs,
            operator,
            rhs,
        }),
        pos,
    ))
}

fn parse_value_nuget(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValueNuget, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    match &start.token {
        Token::Identifier(name) => Ok((
            AstValueNuget::Variable(AstIdentifier {
                range: AstRange {
                    start: start.start_point(),
                    end: start.end_point(),
                },
                value: name.clone(),
            }),
            pos + 1,
        )),
        Token::Number(num) => Ok((
            AstValueNuget::Number(AstNumber {
                range: AstRange::from_position_token(start, start),
                value: *num,
            }),
            pos + 1,
        )),
        Token::DoubleQuote => parse_string_literal(tokens, pos),
        _ => Err(AstError::InvalidNuget(InvalidToken::from(start))),
    }
}
fn parse_string_literal(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValueNuget, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::DoubleQuote)?;
    let mut value = SmolStrBuilder::new();
    let mut pos = pos;
    loop {
        let token = tokens.get(pos).ok_or(AstError::eof())?;
        match &token.token {
            Token::DoubleQuote => {
                let peek = tokens.get(pos - 1).ok_or(AstError::eof())?;
                if peek.token == Token::BackSlash {
                    value.push_str("\\\"");
                } else {
                    break;
                }
            }
            cot => {
                value.push_str(&cot.to_string());
            }
        }
        pos += 1;
    }
    let pos = assert_token(tokens, pos, Token::DoubleQuote)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstValueNuget::StringLiteral(AstIdentifier {
            range: AstRange::from_position_token(start, end),
            value: value.finish(),
        }),
        pos,
    ))
}

fn parse_value_operator(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValueEquasionOperator, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    match &start.token {
        Token::Plus => Ok((
            AstValueEquasionOperator::Plus(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Dash => Ok((
            AstValueEquasionOperator::Minus(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        _ => Err(AstError::InvalidNuget(InvalidToken::from(start))),
    }
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
            range: AstRange::from_position_token(start, end),
            jtype,
            parameters,
        }),
        pos,
    ))
}

fn parse_block_variable(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockVariable, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
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
        AstBlockVariable {
            name,
            jtype,
            value,
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}

fn parse_block_return(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockReturn, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Return)?;
    let (value, pos) = parse_value(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;

    Ok((
        AstBlockReturn {
            range: AstRange::from_position_token(start, end),
            value,
        },
        pos,
    ))
}

fn parse_method_header(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstMethodHeader, usize), AstError> {
    let mut pos = pos;
    let mut avaliability = AstAvailability::Protected;
    let mut stat = false;
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    if let Ok((avav, npos)) = parse_avaliability(tokens, pos) {
        avaliability = avav;
        pos = npos;
    };
    if let Ok(npos) = assert_token(tokens, pos, Token::Static) {
        stat = true;
        pos = npos;
    }
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let (parameters, pos) = parse_method_paramerters(tokens, pos)?;
    let mut pos = pos;
    let mut throws = None;
    if let Ok((nthrows, npos)) = parse_throws_declaration(tokens, pos) {
        throws = Some(nthrows);
        pos = npos;
    }
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstMethodHeader {
            range: AstRange::from_position_token(start, end),
            avaliability,
            name,
            jtype,
            parameters,
            stat,
            throws,
        },
        pos,
    ))
}
fn parse_throws_declaration(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstThrowsDeclaration, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Throws)?;
    let (parameters, pos) = parse_type_list(tokens, pos)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstThrowsDeclaration {
            range: AstRange::from_position_token(start, end),
            parameters,
        },
        pos,
    ))
}

pub fn parse_type_parameters(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstTypeParameters, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Lt)?;
    let mut pos = pos;
    let mut parameters = vec![];
    loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::Gt) {
            pos = npos;
            break;
        }
        let (name, npos) = parse_name(tokens, pos)?;
        pos = npos;
        parameters.push(name);
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Gt) {
            pos = npos;
            break;
        }
    }
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstTypeParameters {
            range: AstRange::from_position_token(start, end),
            parameters,
        },
        pos,
    ))
}

fn parse_extends(tokens: &[PositionToken], pos: usize) -> Result<(AstExtends, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Extends)?;
    let (parameters, pos) = parse_type_list(tokens, pos)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstExtends {
            range: AstRange::from_position_token(start, end),
            parameters,
        },
        pos,
    ))
}

fn parse_type_list(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstJType>, usize), AstError> {
    let mut pos = pos;
    let mut parameters = vec![];
    while let Ok((name, npos)) = parse_jtype(tokens, pos) {
        pos = npos;
        parameters.push(name);

        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }
    }
    Ok((parameters, pos))
}

fn parse_block(tokens: &[PositionToken], pos: usize) -> Result<(AstBlock, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut pos = pos;
    let mut entries = vec![];
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
        match parse_block_variable(tokens, pos) {
            Ok((variable, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Variable(variable));
                continue;
            }
            Err(e) => {
                errors.push(("block variable", e));
            }
        }
        match parse_block_return(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Return(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block return", e));
            }
        }
        pos += 1;
        // return Err(AstError::AllChildrenFailed {
        //     parent: "block",
        //     errors,
        // });
    }
    // let pos = assert_token(tokens, pos, Token::RightParenCurly)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof()).unwrap();
    Ok((
        AstBlock {
            range: AstRange::from_position_token(start, end),
            entries,
        },
        pos,
    ))
}

fn parse_method_paramerters(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstMethodParamerters, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let mut parameters = vec![];
    let mut pos = pos;
    let mut early_exit = false;
    'l: loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParen) {
            pos = npos;
            early_exit = true;
            break 'l;
        }
        match parse_method_paramerter(tokens, pos) {
            Ok((parameter, npos)) => {
                parameters.push(parameter);
                pos = npos;
            }
            Err(e) => return Err(e),
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
        } else {
            break 'l;
        }
    }
    if !early_exit {
        let npos = assert_token(tokens, pos, Token::RightParen)?;
        pos = npos;
    }
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstMethodParamerters {
            range: AstRange::from_position_token(start, end),
            parameters,
        },
        pos,
    ))
}
fn parse_method_paramerter(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstMethodParamerter, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstMethodParamerter {
            range: AstRange::from_position_token(start, end),
            jtype,
            name,
        },
        pos,
    ))
}

// Only one Token::Identifier content
fn parse_name(tokens: &[PositionToken], pos: usize) -> Result<(AstIdentifier, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let ident;
    let t = tokens.get(pos).ok_or(AstError::eof())?;
    match &t.token {
        Token::Identifier(id) => {
            ident = id;
            pos += 1;
        }
        _ => {
            let t = tokens.get(pos).ok_or(AstError::eof())?;
            return Err(AstError::InvalidName(InvalidToken::from(t)));
        }
    }
    if ident.is_empty() {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        return Err(AstError::IdentifierEmpty(InvalidToken::from(t)));
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstIdentifier {
            range: AstRange::from_position_token(start, end),
            value: ident.clone(),
        },
        pos,
    ))
}

// Conatins Token::Identifier, Token::Dot, Token::Star
fn parse_identifier(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut ident = SmolStrBuilder::new();
    let mut modded = false;
    loop {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match &t.token {
            Token::Identifier(id) => {
                modded = true;
                ident.push_str(id);
                pos += 1;
            }
            Token::Dot => {
                modded = true;
                ident.push('.');
                pos += 1;
            }
            Token::Star => {
                modded = true;
                ident.push('*');
                pos += 1;
            }
            _ => break,
        }
    }
    if !modded {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        return Err(AstError::IdentifierEmpty(InvalidToken::from(t)));
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstIdentifier {
            range: AstRange::from_position_token(start, end),
            value: ident.finish(),
        },
        pos,
    ))
}

fn parse_superclass(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSuperClass, usize), AstError> {
    let Ok(pos) = assert_token(tokens, pos, Token::Extends) else {
        return Ok((AstSuperClass::None, pos));
    };
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let sp = match jtype.value {
        AstJTypeKind::Class(c) | AstJTypeKind::Generic(c, _) => AstSuperClass::Name(c),
        _ => AstSuperClass::None,
    };

    Ok((sp, pos))
}

fn parse_jtype(tokens: &[PositionToken], pos: usize) -> Result<(AstJType, usize), AstError> {
    let out_pos = pos + 1;
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
            let token = tokens.get(pos + 1).ok_or(AstError::eof())?;
            let range = AstRange::from_position_token(current, current);
            let ident = AstIdentifier {
                value: ident.clone(),
                range: range.clone(),
            };
            match token.token {
                Token::Lt => {
                    let out_pos;
                    let mut args = vec![];
                    let mut pos = pos + 2;
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
        let parsed = parse_file(&tokens);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed.unwrap());
    }

    #[test]
    fn skip_comments() {
        let content = include_str!("../test/FullOffComments.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed.unwrap());
    }

    #[test]
    fn locale_variable_table() {
        let content = include_str!("../../parser/test/LocalVariableTable.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed.unwrap());
    }

    #[test]
    fn superee() {
        let content = include_str!("../../parser/test/Super.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed.unwrap());
    }

    #[test]
    fn constants() {
        let content = include_str!("../../parser/test/Constants.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed.unwrap());
    }

    #[test]
    fn super_interface() {
        let content = include_str!("../../parser/test/SuperInterface.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed.unwrap());
    }
}
