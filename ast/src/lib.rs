use annotation::parse_annotation;
use class::parse_class;
use enumeration::parse_enumeration;
use error::{AstError, ExpectedToken, InvalidToken, assert_token};
use interface::parse_interface;
use lexer::{PositionToken, Token};
use smol_str::SmolStrBuilder;
use types::{
    AstAvailability, AstBlock, AstBlockAssign, AstBlockEntry, AstBlockExpression, AstBlockReturn,
    AstBlockVariable, AstBoolean, AstDouble, AstExpression, AstExtends, AstFile, AstIdentifier,
    AstImport, AstImportUnit, AstImports, AstJType, AstJTypeKind, AstMethodHeader,
    AstMethodParamerter, AstMethodParamerters, AstNumber, AstPoint, AstRange, AstSuperClass,
    AstThing, AstThrowsDeclaration, AstTypeParameters, AstValue, AstValueEquasion,
    AstValueEquasionOperator,
};

pub mod annotation;
pub mod class;
pub mod enumeration;
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
fn parse_imports(tokens: &[PositionToken], pos: usize) -> Result<(AstImports, usize), AstError> {
    let mut pos = pos;
    let mut imports = vec![];

    let start = tokens.get(pos).ok_or(AstError::eof())?;
    while let Ok((import, new_pos)) = parse_import(tokens, pos) {
        pos = new_pos;
        imports.push(import);
    }
    let end = tokens.get(pos).ok_or(AstError::eof())?;

    Ok((
        AstImports {
            range: AstRange {
                start: start.start_point(),
                end: end.end_point(),
            },
            imports,
        },
        pos,
    ))
}

///  import java.io.IOException;
fn parse_import(tokens: &[PositionToken], pos: usize) -> Result<(AstImport, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Import)?;
    let mut pos = pos;
    let mut stat = false;
    let mut prefix = false;
    if let Ok(npos) = assert_token(tokens, pos, Token::Static) {
        pos = npos;
        stat = true;
    }
    let (ident, pos) = parse_identifier(tokens, pos)?;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Star) {
        pos = npos;
        prefix = true;
    }
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstImport {
            range: AstRange::from_position_token(start, end),
            unit: match (stat, prefix) {
                (true, true) => AstImportUnit::StaticPrefix(ident),
                (true, false) => match ident.value.rsplit_once(".") {
                    Some((class, method)) => {
                        match method.chars().next().unwrap_or_default().is_lowercase() {
                            true => AstImportUnit::StaticClassMethod(
                                AstIdentifier {
                                    range: AstRange {
                                        start: ident.range.start.clone(),
                                        end: AstPoint {
                                            line: ident.range.start.line,
                                            col: ident.range.end.line - method.len(),
                                        },
                                    },
                                    value: class.into(),
                                },
                                AstIdentifier {
                                    range: AstRange {
                                        start: AstPoint {
                                            line: ident.range.start.line,
                                            col: ident.range.start.col - class.len(),
                                        },
                                        end: ident.range.end,
                                    },
                                    value: method.into(),
                                },
                            ),
                            false => AstImportUnit::StaticClass(ident),
                        }
                    }
                    None => AstImportUnit::StaticPrefix(ident),
                },
                (false, true) => AstImportUnit::Prefix(ident),
                (false, false) => AstImportUnit::Class(ident),
            },
        },
        pos,
    ))
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
            PositionToken {
                token: Token::Enum,
                line: _,
                col: _,
            } => parse_enumeration(tokens, pos + 1, avaliability),
            PositionToken {
                token: Token::At,
                line: _,
                col: _,
            } => parse_annotation(tokens, pos + 1, avaliability),
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
    match parse_value_equasion(tokens, pos) {
        Ok(v) => return Ok(v),
        Err(e) => errors.push(("value expression".to_string(), e)),
    }
    match parse_expression(tokens, pos) {
        Ok((expression, pos)) => return Ok((AstValue::Expression(expression), pos)),
        Err(e) => errors.push(("value expression".to_string(), e)),
    };
    match parse_value_new_class(tokens, pos) {
        Ok(v) => return Ok(v),
        Err(e) => errors.push(("value new class".to_string(), e)),
    };
    match parse_value_nuget(tokens, pos) {
        Ok((nuget, pos)) => return Ok((nuget, pos)),
        Err(e) => errors.push(("value nuget".to_string(), e)),
    }
    Err(AstError::AllChildrenFailed {
        parent: "value".to_string(),
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
            lhs: Box::new(lhs),
            operator,
            rhs: Box::new(rhs),
        }),
        pos,
    ))
}

fn parse_value_nuget(tokens: &[PositionToken], pos: usize) -> Result<(AstValue, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    match &start.token {
        Token::Identifier(name) => Ok((
            AstValue::Variable(AstIdentifier {
                range: AstRange {
                    start: start.start_point(),
                    end: start.end_point(),
                },
                value: name.clone(),
            }),
            pos + 1,
        )),
        Token::Number(num) => {
            if let Ok(pos) = assert_token(tokens, pos + 1, Token::Dot) {
                let current = tokens.get(pos).ok_or(AstError::eof())?;
                if let Token::Number(n) = current.token {
                    let value: f64 = format!("{num}.{n}")
                        .parse()
                        .map_err(|_| AstError::InvalidDouble(*num, n))?;
                    let pos = pos + 1;
                    let current = tokens.get(pos).ok_or(AstError::eof())?;
                    let pos = pos + 1;
                    match &current.token {
                        Token::Identifier(val) if val == "d" => {
                            return Ok((
                                AstValue::Double(AstDouble {
                                    range: AstRange::from_position_token(start, start),
                                    value,
                                }),
                                pos,
                            ));
                        }
                        Token::Identifier(val) if val == "f" => {
                            return Ok((
                                AstValue::Float(AstDouble {
                                    range: AstRange::from_position_token(start, start),
                                    value,
                                }),
                                pos,
                            ));
                        }
                        _ => return Err(AstError::InvalidNuget(InvalidToken::from(current))),
                    }
                }
            }
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::Identifier("l".into())) {
                return Ok((
                    AstValue::Number(AstNumber {
                        range: AstRange::from_position_token(start, start),
                        value: *num,
                    }),
                    npos,
                ));
            }
            Ok((
                AstValue::Number(AstNumber {
                    range: AstRange::from_position_token(start, start),
                    value: *num,
                }),
                pos + 1,
            ))
        }
        Token::DoubleQuote => parse_string_literal(tokens, pos),
        Token::SingleQuote => parse_char_literal(tokens, pos),
        Token::True => parse_boolean_literal(tokens, pos, true),
        Token::False => parse_boolean_literal(tokens, pos, false),
        _ => Err(AstError::InvalidNuget(InvalidToken::from(start))),
    }
}

fn parse_char_literal(tokens: &[PositionToken], pos: usize) -> Result<(AstValue, usize), AstError> {
    let pos = assert_token(tokens, pos, Token::SingleQuote)?;
    let (char, pos) = parse_name(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::SingleQuote)?;
    Ok((AstValue::CharLiteral(char), pos))
}

fn parse_boolean_literal(
    tokens: &[PositionToken],
    pos: usize,
    value: bool,
) -> Result<(AstValue, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstValue::BooleanLiteral(AstBoolean {
            range: AstRange::from_position_token(start, start),
            value,
        }),
        pos + 1,
    ))
}
fn parse_string_literal(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValue, usize), AstError> {
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
        AstValue::StringLiteral(AstIdentifier {
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
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParen) {
            pos = npos;
            break;
        }
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
fn parse_expression(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstExpression, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut ident = None;
    let mut values = None;
    let mut next = None;

    let mut pos = pos;
    let curretn = tokens.get(pos).ok_or(AstError::eof())?;
    match curretn.token {
        Token::Identifier(_) | Token::Class => {
            let (id, npos) = parse_expression_lhs(tokens, pos)?;
            ident = Some(id);
            pos = npos;
            if let Ok((exp, npos)) = parse_expression(tokens, pos) {
                pos = npos;
                if exp.has_content() {
                    next = Some(Box::new(exp));
                }
            }
        }
        Token::Dot => {
            let (exp, npos) = parse_expression(tokens, pos + 1)?;
            pos = npos;
            if exp.has_content() {
                next = Some(Box::new(exp));
            }
        }
        Token::LeftParen => {
            let (vals, npos) = parse_value_parameters(tokens, pos)?;
            values = Some(vals);
            pos = npos;
        }
        _ => return Err(AstError::InvalidExpression(InvalidToken::from(curretn))),
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstExpression {
            range: AstRange::from_position_token(start, end),
            ident,
            next,
            values,
        },
        pos,
    ))
}

fn parse_expression_lhs(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    match parse_name(tokens, pos) {
        Ok((ident, npos)) => Ok((ident, npos)),
        Err(AstError::InvalidName(InvalidToken {
            found: Token::Class,
            line: _,
            col: _,
        })) => {
            let start = tokens.get(pos).ok_or(AstError::eof())?;
            Ok((
                AstIdentifier {
                    range: AstRange::from_position_token(start, start),
                    value: "class".into(),
                },
                pos + 1,
            ))
        }
        Err(e) => Err(e),
    }
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
fn parse_block_expression(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockExpression, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (value, pos) = parse_expression(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;

    Ok((
        AstBlockExpression {
            range: AstRange::from_position_token(start, end),
            value,
        },
        pos,
    ))
}

fn parse_block_assign(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockAssign, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (key, pos) = parse_expression(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Equal)?;
    let (value, pos) = parse_value(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;

    Ok((
        AstBlockAssign {
            range: AstRange::from_position_token(start, end),
            key,
            value,
        },
        pos,
    ))
}

fn parse_method_header(
    tokens: &[PositionToken],
    pos: usize,
    default_availability: AstAvailability,
) -> Result<(AstMethodHeader, usize), AstError> {
    let mut pos = pos;
    let mut avaliability = default_availability;
    let mut stat = false;
    let mut type_parameters = None;
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    if let Ok((avav, npos)) = parse_avaliability(tokens, pos) {
        avaliability = avav;
        pos = npos;
    };

    if let Ok(npos) = assert_token(tokens, pos, Token::Static) {
        stat = true;
        pos = npos;
    }

    if let Ok((type_params, npos)) = parse_type_parameters(tokens, pos) {
        type_parameters = Some(type_params);
        pos = npos;
    };

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
            type_parameters,
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
                errors.push(("block variable".to_string(), e));
            }
        }
        match parse_block_return(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Return(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block return".to_string(), e));
            }
        }
        match parse_block_expression(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Expression(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block expression".to_string(), e));
            }
        }
        match parse_block_assign(tokens, pos) {
            Ok((nret, npos)) => {
                pos = npos;
                entries.push(AstBlockEntry::Assign(nret));
                continue;
            }
            Err(e) => {
                errors.push(("block expression".to_string(), e));
            }
        }
        return Err(AstError::AllChildrenFailed {
            parent: "block".to_string(),
            errors,
        });
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
    let mut pos = pos;
    let mut fin = false;
    if let Ok(npos) = assert_token(tokens, pos, Token::Final) {
        fin = true;
        pos = npos;
    }
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (name, pos) = parse_name(tokens, pos)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstMethodParamerter {
            range: AstRange::from_position_token(start, end),
            jtype,
            name,
            fin,
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

// Conatins Token::Identifier, Token::Dot
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
                        if let Ok(npos) = assert_token(tokens, pos, Token::QuestionMark) {
                            let wstart = tokens.get(pos).ok_or(AstError::eof())?;

                            if let Ok(npos) = assert_token(tokens, npos, Token::Implements) {
                                pos = npos;
                                let wend = tokens.get(pos).ok_or(AstError::eof())?;
                                args.push(AstJType {
                                    range: AstRange::from_position_token(wstart, wend),
                                    value: AstJTypeKind::Wildcard,
                                });
                            }
                            if let Ok(npos) = assert_token(tokens, npos, Token::Extends) {
                                pos = npos;
                                let wend = tokens.get(pos).ok_or(AstError::eof())?;
                                args.push(AstJType {
                                    range: AstRange::from_position_token(wstart, wend),
                                    value: AstJTypeKind::Wildcard,
                                });
                            }
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
                Token::LeftParenSquare => Ok((
                    AstJType {
                        range: AstRange::from_position_token(current, current),
                        value: AstJTypeKind::Array(Box::new(AstJType {
                            range: AstRange::from_position_token(current, current),
                            value: AstJTypeKind::Class(ident),
                        })),
                    },
                    out_pos + 2,
                )),
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
        _ => Ok((AstAvailability::Undefined, pos)),
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{error::PrintErr, lexer, parse_expression, parse_file};

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

    #[test]
    fn interface_base() {
        let content = include_str!("../../parser/test/InterfaceBase.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed.unwrap());
    }

    #[test]
    fn variants() {
        let content = include_str!("../../parser/test/Variants.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed.unwrap());
    }

    #[test]
    fn types() {
        let content = include_str!("../../parser/test/Types.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed.unwrap());
    }
    #[test]
    fn expression_base() {
        let content = "Logger.getLogger(Test.class)";
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_expression(&tokens, 0);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed.unwrap());
    }
    #[test]
    fn annotation() {
        let content = include_str!("../../parser/test/Annotation.java");
        let tokens = lexer::lex(content).unwrap();
        let parsed = parse_file(&tokens);
        parsed.print_err(content);
        insta::assert_debug_snapshot!(parsed.unwrap());
    }
}
