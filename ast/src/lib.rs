#![deny(missing_docs)]
//! A java ast
use annotation::parse_annotation;
use class::parse_class;
use enumeration::parse_enumeration;
use error::{AstError, ExpectedToken, InvalidToken, assert_semicolon, assert_token};
use interface::parse_interface;
use lexer::{PositionToken, Token};
use smol_str::SmolStrBuilder;
use types::{
    AstAnnotated, AstAvailability, AstBlock, AstBlockAssign, AstBlockBreak, AstBlockContinue,
    AstBlockEntry, AstBlockExpression, AstBlockReturn, AstBlockVariable, AstBlockVariableMutliType,
    AstBoolean, AstCastedExpression, AstDouble, AstExpression, AstExpressionIdentifier,
    AstExpressionOperator, AstExtends, AstFile, AstFor, AstForEnhanced, AstIdentifier, AstIf,
    AstIfContent, AstImport, AstImportUnit, AstImports, AstInt, AstJType, AstJTypeKind, AstLambda,
    AstLambdaParameters, AstMethodHeader, AstMethodParamerter, AstMethodParamerters, AstNewClass,
    AstPoint, AstRange, AstRecursiveExpression, AstSuperClass, AstSwitch, AstSwitchCase,
    AstSwitchDefault, AstThing, AstThrow, AstThrowsDeclaration, AstTryCatch, AstTryCatchCase,
    AstTypeParameters, AstValue, AstValueNuget, AstValues, AstWhile,
};

use crate::{
    class::parse_class_block,
    error::assert_semicolon_options,
    record::parse_record,
    types::{
        AstAnnotatedParameter, AstBlockYield, AstClassAccess, AstConstructorHeader,
        AstExpressionOrValue, AstForContent, AstGenerics, AstLambdaRhs, AstNewRhs,
        AstSwitchCaseArrow, AstSwitchCaseArrowContent, AstSwitchCaseArrowDefault,
        AstSynchronizedBlock, AstThingAttributes, AstTypeParameter, AstWhileContent,
    },
};

pub mod annotation;
pub mod class;
pub mod enumeration;
pub mod error;
pub mod interface;
pub mod lexer;
pub mod range;
pub mod record;
pub mod types;

///` package ch.emilycares; import .... public class ...`
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

///` package ch.emilycares;`
fn parse_package(tokens: &[PositionToken], pos: usize) -> Result<(AstIdentifier, usize), AstError> {
    let pos = assert_token(tokens, pos, Token::Package)?;
    let (package_name, pos) = parse_name_dot_logical(tokens, pos)?;
    let pos = assert_semicolon(tokens, pos)?;
    Ok((package_name, pos))
}
///`  import java.io.IOException;`
///`  import java.net.Socket;`
fn parse_imports(tokens: &[PositionToken], pos: usize) -> Result<(AstImports, usize), AstError> {
    let mut pos = pos;
    let mut imports = vec![];

    let start = tokens.get(pos).ok_or(AstError::eof())?;
    while let Ok((import, new_pos)) = parse_import(tokens, pos) {
        pos = new_pos;
        imports.push(import);
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

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

///`  import java.io.IOException;`
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
    let (ident, pos) = parse_name_dot(tokens, pos)?;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Star) {
        pos = npos;
        prefix = true;
    }
    let pos = assert_semicolon(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
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
                                        start: ident.range.start,
                                        end: AstPoint {
                                            line: ident.range.start.line,
                                            col: ident.range.end.col - method.len(),
                                        },
                                    },
                                    value: class.into(),
                                },
                                AstIdentifier {
                                    range: AstRange {
                                        start: AstPoint {
                                            line: ident.range.start.line,
                                            col: ident.range.start.col + class.len(),
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

///`  public class Everything { ...`
///`  public interface Constants { ...`
pub fn parse_thing(tokens: &[PositionToken], pos: usize) -> Result<(AstThing, usize), AstError> {
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let (mut avaliability, mut pos) = parse_avaliability(tokens, pos)?;
    let mut attributes = AstThingAttributes::empty();
    loop {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match t.token {
            Token::Static => avaliability |= AstAvailability::Static,
            Token::Final => avaliability |= AstAvailability::Final,
            Token::Abstract => avaliability |= AstAvailability::Abstract,
            Token::Sealed => attributes |= AstThingAttributes::Sealed,
            Token::Non => {
                if let Ok(npos) = assert_token(tokens, pos + 1, Token::Dash)
                    && let Ok(npos) = assert_token(tokens, npos, Token::Sealed)
                {
                    attributes |= AstThingAttributes::NonSealed;
                    pos = npos;
                    continue;
                }
            }
            _ => break,
        }
        pos += 1;
    }
    match tokens.get(pos) {
        Some(t) => {
            let pos = pos + 1;
            match t.token {
                Token::Class => parse_class(tokens, pos, avaliability, attributes, annotated),
                Token::Record => parse_record(tokens, pos, avaliability, attributes, annotated),
                Token::Interface => {
                    parse_interface(tokens, pos, avaliability, attributes, annotated)
                }
                Token::Enum => parse_enumeration(tokens, pos, avaliability, attributes, annotated),
                Token::At => parse_annotation(tokens, pos, avaliability, attributes, annotated),
                _ => Err(AstError::ExpectedToken(ExpectedToken::from(
                    t,
                    pos,
                    Token::Class,
                ))),
            }
        }
        None => Err(AstError::eof()),
    }
}

fn parse_value(tokens: &[PositionToken], pos: usize) -> Result<(AstValue, usize), AstError> {
    let mut errors = vec![];
    match parse_boolean_literal(tokens, pos) {
        Ok((nuget, pos)) => return Ok((nuget, pos)),
        Err(e) => errors.push(("value boolean".into(), e)),
    }
    match parse_value_nuget(tokens, pos) {
        Ok((nuget, pos)) => return Ok((nuget, pos)),
        Err(e) => errors.push(("value nuget".into(), e)),
    }
    Err(AstError::AllChildrenFailed {
        parent: "value".into(),
        errors,
    })
}
fn parse_annotated_list(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstAnnotated>, usize), AstError> {
    let mut out = vec![];
    let mut pos = pos;
    while let Ok((a, npos)) = parse_annotated(tokens, pos) {
        out.push(a);
        pos = npos;
    }
    Ok((out, pos))
}
fn parse_annotated(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstAnnotated, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::At)?;
    let (name, pos) = parse_name_dot_logical(tokens, pos)?;
    let mut parameters = vec![];
    let mut pos = pos;
    if let Ok((params, npos)) = parse_annotated_parameters(tokens, pos) {
        parameters = params;
        pos = npos;
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstAnnotated {
            range: AstRange::from_position_token(start, end),
            name,
            parameters,
        },
        pos,
    ))
}
///` (a) -> a.doThing()`
///` a -> a.doThing()`
///` a -> {a.doThing();}`
pub fn parse_lambda(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstLambda, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let parameters;
    if let Ok((lparams, npos)) = parse_lambda_parameters(tokens, pos) {
        parameters = lparams;
        pos = npos;
    } else {
        let (n, npos) = parse_name(tokens, pos)?;
        parameters = AstLambdaParameters {
            range: n.range,
            values: vec![n],
        };
        pos = npos;
    }
    let mut pos = assert_token(tokens, pos, Token::Arrow)?;
    let mut rhs = AstLambdaRhs::None;
    if let Ok((block, npos)) = parse_block(tokens, pos) {
        pos = npos;
        rhs = AstLambdaRhs::Block(block);
    } else if let Ok((expr, npos)) = parse_expression(tokens, pos, expression_options) {
        pos = npos;
        rhs = AstLambdaRhs::Expr(Box::new(expr));
    }

    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstLambda {
            range: AstRange::from_position_token(start, end),
            parameters,
            rhs,
        },
        pos,
    ))
}
/// `(a) -> a.length`
///  ^^^
pub fn parse_lambda_parameters(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstLambdaParameters, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let mut pos = pos;
    let mut values = vec![];

    while let Ok((n, npos)) = parse_name(tokens, pos) {
        values.push(n);

        pos = npos;
        if let Ok(npos) = assert_token(tokens, npos, Token::Comma) {
            pos = npos;
        } else {
            break;
        }
    }

    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstLambdaParameters {
            range: AstRange::from_position_token(start, end),
            values,
        },
        pos,
    ))
}

/// `{ "", "" }`
fn parse_array(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstValues, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = assert_token(tokens, pos, Token::LeftParenCurly)?;
    let mut values = vec![];
    loop {
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            pos = npos;
            break;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }
        let (value, npos) = parse_expression(tokens, pos, expression_options)?;
        pos = npos;
        values.push(value);
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenCurly) {
            pos = npos;
            break;
        }
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstValues {
            range: AstRange::from_position_token(start, end),
            values,
        },
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
                if let Token::Number(n) = &current.token {
                    let value: f64 = format!("{num}.{n}")
                        .parse()
                        .map_err(|_| AstError::InvalidDouble(num.clone(), n.clone()))?;
                    let pos = pos + 1;
                    let current = tokens.get(pos).ok_or(AstError::eof())?;
                    match &current.token {
                        Token::Identifier(val) if val == "d" => {
                            return Ok((
                                AstValue::Nuget(AstValueNuget::Double(AstDouble {
                                    range: AstRange::from_position_token(start, start),
                                    value,
                                })),
                                pos + 1,
                            ));
                        }
                        Token::Identifier(val) if val == "f" => {
                            return Ok((
                                AstValue::Nuget(AstValueNuget::Float(AstDouble {
                                    range: AstRange::from_position_token(start, start),
                                    value,
                                })),
                                pos + 1,
                            ));
                        }
                        _ => {
                            return Ok((
                                AstValue::Nuget(AstValueNuget::Float(AstDouble {
                                    range: AstRange::from_position_token(start, start),
                                    value,
                                })),
                                pos,
                            ));
                        }
                    }
                }
            }
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::Identifier("l".into())) {
                return Ok((
                    AstValue::Nuget(AstValueNuget::Int(AstInt {
                        range: AstRange::from_position_token(start, start),
                        value: num.clone(),
                    })),
                    npos,
                ));
            }
            Ok((
                AstValue::Nuget(AstValueNuget::Int(AstInt {
                    range: AstRange::from_position_token(start, start),
                    value: num.clone(),
                })),
                pos + 1,
            ))
        }
        Token::StringLiteral(_) => {
            parse_string_literal(tokens, pos).map(|i| (AstValue::Nuget(i.0), i.1))
        }
        Token::CharLiteral(_) => {
            parse_char_literal(tokens, pos).map(|i| (AstValue::Nuget(i.0), i.1))
        }
        Token::True => parse_boolean_literal_input(tokens, pos, true),
        Token::False => parse_boolean_literal_input(tokens, pos, false),
        _ => Err(AstError::InvalidNuget(InvalidToken::from(start, pos))),
    }
}

fn parse_boolean_literal_input(
    tokens: &[PositionToken],
    pos: usize,
    value: bool,
) -> Result<(AstValue, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    Ok((
        AstValue::Nuget(AstValueNuget::BooleanLiteral(AstBoolean {
            range: AstRange::from_position_token(start, start),
            value,
        })),
        pos + 1,
    ))
}

fn parse_boolean_literal(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValue, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let value = match start.token {
        Token::True => true,
        Token::False => false,
        _ => return Err(AstError::InvalidBoolean(InvalidToken::from(start, pos))),
    };
    Ok((
        AstValue::Nuget(AstValueNuget::BooleanLiteral(AstBoolean {
            range: AstRange::from_position_token(start, start),
            value,
        })),
        pos + 1,
    ))
}
/// `"some string"`
pub fn parse_string_literal(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValueNuget, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    match &start.token {
        Token::StringLiteral(str) => {
            let end = tokens.get(pos.saturating_sub(1)).ok_or(AstError::eof())?;
            Ok((
                AstValueNuget::StringLiteral(AstIdentifier {
                    range: AstRange::from_position_token(start, end),
                    value: str.clone(),
                }),
                pos + 1,
            ))
        }
        _ => Err(AstError::InvalidString(InvalidToken::from(start, pos))),
    }
}
/// `'\r`
pub fn parse_char_literal(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstValueNuget, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    match &start.token {
        Token::CharLiteral(str) => {
            let end = tokens.get(pos.saturating_sub(1)).ok_or(AstError::eof())?;
            Ok((
                AstValueNuget::CharLiteral(AstIdentifier {
                    range: AstRange::from_position_token(start, end),
                    value: str.clone(),
                }),
                pos + 1,
            ))
        }
        _ => Err(AstError::InvalidString(InvalidToken::from(start, pos))),
    }
}

fn parse_value_operator_options(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstExpressionOperator, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    match &start.token {
        Token::Plus => {
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::Plus) {
                let end = tokens.get(npos).ok_or(AstError::eof())?;
                Ok((
                    AstExpressionOperator::PlusPlus(AstRange::from_position_token(start, end)),
                    npos,
                ))
            } else {
                Ok((
                    AstExpressionOperator::Plus(AstRange::from_position_token(start, start)),
                    pos + 1,
                ))
            }
        }
        Token::Dash => {
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::Dash) {
                let end = tokens.get(npos).ok_or(AstError::eof())?;
                Ok((
                    AstExpressionOperator::MinusMinus(AstRange::from_position_token(start, end)),
                    npos,
                ))
            } else {
                Ok((
                    AstExpressionOperator::Minus(AstRange::from_position_token(start, start)),
                    pos + 1,
                ))
            }
        }
        Token::Ampersand => {
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::Ampersand) {
                let end = tokens.get(npos).ok_or(AstError::eof())?;
                Ok((
                    AstExpressionOperator::AmpersandAmpersand(AstRange::from_position_token(
                        start, end,
                    )),
                    npos,
                ))
            } else {
                Ok((
                    AstExpressionOperator::Ampersand(AstRange::from_position_token(start, start)),
                    pos + 1,
                ))
            }
        }
        Token::VerticalBar => {
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::VerticalBar) {
                let end = tokens.get(npos).ok_or(AstError::eof())?;
                Ok((
                    AstExpressionOperator::VerticalBarVerticalBar(AstRange::from_position_token(
                        start, end,
                    )),
                    npos,
                ))
            } else {
                Ok((
                    AstExpressionOperator::VerticalBar(AstRange::from_position_token(start, start)),
                    pos + 1,
                ))
            }
        }
        Token::QuestionMark => Ok((
            AstExpressionOperator::QuestionMark(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Colon => {
            if let Ok(npos) = assert_token(tokens, pos + 1, Token::Colon) {
                let end = tokens.get(npos).ok_or(AstError::eof())?;
                return Ok((
                    AstExpressionOperator::ColonColon(AstRange::from_position_token(start, end)),
                    npos,
                ));
            } else if expression_options != &ExpressionOptions::NoInlineIf {
                return Ok((
                    AstExpressionOperator::Colon(AstRange::from_position_token(start, start)),
                    pos + 1,
                ));
            }
            Err(AstError::InvalidNuget(InvalidToken::from(start, pos)))
        }
        Token::ExclamationMark if expression_options != &ExpressionOptions::NoInlineIf => Ok((
            AstExpressionOperator::ExclemationMark(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Dot => Ok((
            AstExpressionOperator::Dot(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Star => Ok((
            AstExpressionOperator::Multiply(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Slash => Ok((
            AstExpressionOperator::Devide(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Percent => Ok((
            AstExpressionOperator::Modulo(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::EqualDouble => Ok((
            AstExpressionOperator::Equal(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Equal => Ok((
            AstExpressionOperator::Assign(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Ne => Ok((
            AstExpressionOperator::NotEqual(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Gt => Ok((
            AstExpressionOperator::Gt(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Ge => Ok((
            AstExpressionOperator::Ge(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Lt => Ok((
            AstExpressionOperator::Lt(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Le => Ok((
            AstExpressionOperator::Le(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Tilde => Ok((
            AstExpressionOperator::Tilde(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        Token::Caret => Ok((
            AstExpressionOperator::Caret(AstRange::from_position_token(start, start)),
            pos + 1,
        )),
        _ => Err(AstError::InvalidNuget(InvalidToken::from(start, pos))),
    }
}

fn parse_annotated_parameters(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstAnnotatedParameter>, usize), AstError> {
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

        let start_named = tokens.get(pos).ok_or(AstError::eof())?;
        if let Ok((name, npos)) = parse_name(tokens, pos)
            && let Ok(npos) = assert_token(tokens, npos, Token::Equal)
        {
            let (expression, npos) = parse_expression(tokens, npos, &ExpressionOptions::None)?;
            pos = npos;
            let end_named = tokens.get(pos).ok_or(AstError::eof())?;
            out.push(AstAnnotatedParameter::NamedExpression {
                range: AstRange::from_position_token(start_named, end_named),
                name,
                expression,
            });
            continue;
        }
        let (expression, npos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
        pos = npos;
        out.push(AstAnnotatedParameter::Expression(expression));
    }
    Ok((out, pos))
}
fn parse_expression_parameters(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstExpression>, usize), AstError> {
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

        let (expression, npos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
        pos = npos;
        out.push(expression);
    }
    Ok((out, pos))
}
fn parse_array_parameters(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(Vec<Vec<AstExpression>>, usize), AstError> {
    let mut out = vec![];
    let mut current = vec![];
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::LeftParenSquare) {
        pos = npos;

        if let Ok((expression, npos)) = parse_expression(tokens, pos, expression_options) {
            pos = npos;
            current.push(expression);
        }
        while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            let (expression, npos) = parse_expression(tokens, pos, expression_options)?;
            pos = npos;
            current.push(expression);
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::RightParenSquare) {
            pos = npos;
            out.push(current);
            current = vec![];
            continue;
        }
    }
    Ok((out, pos))
}

/// `new String()`
pub fn parse_new_class(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstNewClass, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::New)?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let mut pos = pos;
    let mut rhs = AstNewRhs::None;
    let mut errors = vec![];
    match parse_array_parameters(tokens, pos, expression_options) {
        Ok((array_parameters, npos)) => {
            pos = npos;
            if !array_parameters.is_empty() {
                rhs = AstNewRhs::ArrayParameters(array_parameters);
            }
        }
        Err(e) => errors.push(("array_parameters".into(), e)),
    }
    match parse_expression_parameters(tokens, pos) {
        Ok((nrhs, npos)) => {
            pos = npos;
            rhs = AstNewRhs::Parameters(nrhs);
        }
        Err(e) => errors.push(("expression_parameters".into(), e)),
    }
    if let AstJTypeKind::Array(_) = jtype.value {
        match parse_array(tokens, pos, expression_options) {
            Ok((nrhs, npos)) => {
                pos = npos;
                rhs = AstNewRhs::Array(nrhs);
            }
            Err(e) => errors.push(("array".into(), e)),
        }
    } else {
        match parse_class_block(tokens, pos) {
            Ok((b, npos)) => {
                pos = npos;
                if let AstNewRhs::Parameters(p) = rhs {
                    rhs = AstNewRhs::ParametersAndBlock(p, b);
                } else {
                    rhs = AstNewRhs::Block(b);
                }
            }
            Err(e) => errors.push(("array".into(), e)),
        }
    }
    if matches!(rhs, AstNewRhs::None) {
        return Err(AstError::AllChildrenFailed {
            parent: "new_class".into(),
            errors,
        });
    }
    let mut next = None;
    if let Ok((e, npos)) = parse_expression(tokens, pos, expression_options) {
        next = Some(Box::new(e));
        pos = npos;
    }

    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstNewClass {
            range: AstRange::from_position_token(start, end),
            jtype,
            rhs: Box::new(rhs),
            next,
        },
        pos,
    ))
}
/// `byte.class`
/// `String.class`
pub fn parse_class_access(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstClassAccess, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Dot)?;
    let mut pos = assert_token(tokens, pos, Token::Class)?;
    let mut next = None;
    if let Ok((e, npos)) = parse_expression(tokens, pos, expression_options) {
        next = Some(Box::new(e));
        pos = npos;
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstClassAccess {
            range: AstRange::from_position_token(start, end),
            jtype,
            next,
        },
        pos,
    ))
}
/// `<int>`
pub fn parse_generics(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstGenerics, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Lt)?;
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let mut pos = assert_token(tokens, pos, Token::Gt)?;
    let mut next = None;
    if let Ok((e, npos)) = parse_expression(tokens, pos, expression_options) {
        next = Some(Box::new(e));
        pos = npos;
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstGenerics {
            range: AstRange::from_position_token(start, end),
            jtype,
            next,
        },
        pos,
    ))
}
/// Options for expression parsing
#[derive(Debug, PartialEq)]
pub enum ExpressionOptions {
    /// Default expression
    None,
    /// Don't parse '<exp> ? <expr> : expr'
    /// QuestionMark and Colon will not be parsed as operators
    NoInlineIf,
    /// Don't parse labdas
    NoLambda,
}
/// `a.a()`
/// `(byte)'\r'
pub fn parse_expression(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstExpression, usize), AstError> {
    let (e, pos) = parse_expression_inner(tokens, pos, expression_options)?;

    if !e.has_content() {
        let token = tokens.get(pos).ok_or(AstError::eof())?;
        return Err(AstError::EmptyExpression(InvalidToken::from(token, pos)));
    }

    Ok((e, pos))
}
fn parse_expression_inner(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstExpression, usize), AstError> {
    let mut errors = vec![];
    match parse_array(tokens, pos, expression_options) {
        Ok((v, pos)) => return Ok((AstExpression::Array(v), pos)),
        Err(e) => errors.push(("array".into(), e)),
    }
    if expression_options != &ExpressionOptions::NoLambda {
        match parse_lambda(tokens, pos, expression_options) {
            Ok((lambda, pos)) => {
                return Ok((AstExpression::Lambda(lambda), pos));
            }
            Err(e) => errors.push(("lambda".into(), e)),
        }
    }
    match parse_switch(tokens, pos, expression_options) {
        Ok((casted, pos)) => {
            return Ok((AstExpression::InlineSwitch(casted), pos));
        }
        Err(e) => errors.push(("inline switch".into(), e)),
    }
    match parse_casted_expression(tokens, pos, expression_options) {
        Ok((casted, pos)) => {
            return Ok((AstExpression::Casted(casted), pos));
        }
        Err(e) => errors.push(("casted".into(), e)),
    }
    match parse_jtype_expression(tokens, pos, expression_options) {
        Ok((casted, pos)) => {
            return Ok((AstExpression::JType(casted), pos));
        }
        Err(e) => errors.push(("casted".into(), e)),
    }
    match parse_new_class(tokens, pos, expression_options) {
        Ok((new, pos)) => return Ok((AstExpression::NewClass(new), pos)),
        Err(e) => errors.push(("new class".into(), e)),
    };
    match parse_class_access(tokens, pos, expression_options) {
        Ok((a, pos)) => return Ok((AstExpression::ClassAccess(a), pos)),
        Err(e) => errors.push(("new class".into(), e)),
    };
    match parse_generics(tokens, pos, expression_options) {
        Ok((a, pos)) => return Ok((AstExpression::Generics(a), pos)),
        Err(e) => errors.push(("new class".into(), e)),
    };
    match parse_recursive_expression(tokens, pos, expression_options) {
        Ok((recursive, pos)) => {
            return Ok((AstExpression::Recursive(recursive), pos));
        }
        Err(e) => errors.push(("recursive".into(), e)),
    }
    Err(AstError::AllChildrenFailed {
        parent: "expression".into(),
        errors,
    })
}
fn parse_casted_expression(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstCastedExpression, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (cast, pos) = parse_jtype(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let (expression, pos) = parse_expression(tokens, pos, expression_options)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstCastedExpression {
            range: AstRange::from_position_token(start, end),
            cast,
            expression: Box::new(expression),
        },
        pos,
    ))
}
fn parse_jtype_expression(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstCastedExpression, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (cast, pos) = parse_jtype(tokens, pos)?;
    let (expression, pos) = parse_expression(tokens, pos, expression_options)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstCastedExpression {
            range: AstRange::from_position_token(start, end),
            cast,
            expression: Box::new(expression),
        },
        pos,
    ))
}
/// `a.b.c("a".length)`
pub fn parse_recursive_expression(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(Box<AstRecursiveExpression>, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut out = Box::new(AstRecursiveExpression {
        range: AstRange::from_position_token(start, start),
        ident: None,
        next: None,
        values: None,
        operator: AstExpressionOperator::None,
        instance_of: None,
    });
    let mut pos = pos;
    match start.token {
        Token::Semicolon => {
            return Err(AstError::InvalidExpression(InvalidToken::from(start, pos)));
        }
        Token::Identifier(_) | Token::Class | Token::This | Token::New => {
            let (id, npos) = parse_expression_lhs(tokens, pos)?;
            pos = npos;
            out.ident = Some(AstExpressionIdentifier::Identifier(id));
            if let Ok((exp, npos)) = parse_expression(tokens, pos, expression_options) {
                pos = npos;
                if exp.has_content() {
                    out.next = Some(Box::new(exp));
                }
            }
        }
        Token::LeftParenSquare => {
            pos += 1;
            let (array_access_expr, npos) = parse_expression(tokens, pos, expression_options)?;
            let npos = assert_token(tokens, npos, Token::RightParenSquare)?;
            out.ident = Some(AstExpressionIdentifier::ArrayAccess(Box::new(
                array_access_expr,
            )));
            pos = npos;
            if let Ok((exp, npos)) = parse_expression(tokens, pos, expression_options) {
                pos = npos;
                if exp.has_content() {
                    out.next = Some(Box::new(exp));
                }
            }
        }
        Token::LeftParen => {
            let values_start = tokens.get(pos).ok_or(AstError::eof())?;
            if let Ok((vals, npos)) = parse_expression_parameters(tokens, pos) {
                pos = npos;
                let values_end = tokens.get(pos - 1).ok_or(AstError::eof())?;
                out.values = Some(types::AstValues {
                    range: AstRange::from_position_token(values_start, values_end),
                    values: vals,
                });
                if let Ok((exp, npos)) = parse_expression(tokens, pos, expression_options) {
                    pos = npos;
                    if exp.has_content() {
                        out.next = Some(Box::new(exp));
                    }
                }
            }
        }
        Token::InstanceOf => {
            pos += 1;
            let (jtype, npos) = parse_jtype(tokens, pos)?;
            out.instance_of = Some(jtype);
            pos = npos;
            if let Ok((exp, npos)) = parse_expression(tokens, pos, expression_options) {
                pos = npos;
                if exp.has_content() {
                    out.next = Some(Box::new(exp));
                }
            }
        }
        _ => {
            let mut errors = vec![];
            'others: {
                match parse_value_operator_options(tokens, pos, expression_options) {
                    Ok((op, npos)) => {
                        match op {
                            AstExpressionOperator::Colon(_)
                            | AstExpressionOperator::QuestionMark(_) => {
                                if let Ok((exp, npos)) =
                                    parse_expression(tokens, npos, expression_options)
                                {
                                    pos = npos;
                                    out.operator = op;
                                    if exp.has_content() {
                                        out.next = Some(Box::new(exp));
                                    }
                                }
                            }
                            _ => {
                                pos = npos;
                                out.operator = op;
                                if let Ok((exp, npos)) =
                                    parse_expression(tokens, pos, expression_options)
                                {
                                    pos = npos;
                                    if exp.has_content() {
                                        out.next = Some(Box::new(exp));
                                    }
                                }
                            }
                        }
                        break 'others;
                    }
                    Err(e) => errors.push(("operator".into(), e)),
                }
                match parse_value(tokens, pos) {
                    Ok((value, npos)) => {
                        pos = npos;
                        out.ident = Some(AstExpressionIdentifier::Value(value));
                        if let Ok((exp, npos)) = parse_expression(tokens, pos, expression_options) {
                            pos = npos;
                            if exp.has_content() {
                                out.next = Some(Box::new(exp));
                            }
                        }
                        break 'others;
                    }
                    Err(e) => errors.push(("value".into(), e)),
                }
                return Err(AstError::AllChildrenFailed {
                    parent: "expression".into(),
                    errors,
                });
            }
        }
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    out.range = AstRange::from_position_token(start, end);
    Ok((out, pos))
}

fn parse_expression_lhs(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    match parse_name(tokens, pos) {
        Ok((ident, npos)) => Ok((ident, npos)),
        Err(AstError::InvalidName(e)) => match e.found {
            Token::Class => {
                let start = tokens.get(pos).ok_or(AstError::eof())?;
                Ok((
                    AstIdentifier {
                        range: AstRange::from_position_token(start, start),
                        value: "class".into(),
                    },
                    pos + 1,
                ))
            }
            Token::This => {
                let start = tokens.get(pos).ok_or(AstError::eof())?;
                Ok((
                    AstIdentifier {
                        range: AstRange::from_position_token(start, start),
                        value: "this".into(),
                    },
                    pos + 1,
                ))
            }
            Token::New => {
                let start = tokens.get(pos).ok_or(AstError::eof())?;
                Ok((
                    AstIdentifier {
                        range: AstRange::from_position_token(start, start),
                        value: "new".into(),
                    },
                    pos + 1,
                ))
            }
            _ => Err(AstError::InvalidName(e)),
        },
        Err(e) => Err(e),
    }
}

/// `String value = "a";`
pub fn parse_block_variable(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstBlockVariable>, usize), AstError> {
    parse_block_variable_options(tokens, pos, &BlockEntryOptions::None)
}
fn parse_block_variable_options(
    tokens: &[PositionToken],
    pos: usize,
    block_entry_options: &BlockEntryOptions,
) -> Result<(Vec<AstBlockVariable>, usize), AstError> {
    let (block_variable, pos) = parse_block_variable_no_semicolon(tokens, pos)?;
    let pos = assert_semicolon_options(tokens, pos, block_entry_options)?;
    Ok((block_variable, pos))
}
fn parse_block_variable_no_semicolon(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstBlockVariable>, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut pos = pos;
    let mut fin = false;
    if let Ok(npos) = assert_token(tokens, pos, Token::Final) {
        pos = npos;
        fin = true;
    }
    let mut out = vec![];
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    let (v, pos) = parse_variable_base(tokens, start, &annotated, fin, &jtype, pos)?;
    out.push(v);
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
        let (v, npos) = parse_variable_base(tokens, start, &annotated, fin, &jtype, npos)?;
        pos = npos;
        out.push(v);
    }

    Ok((out, pos))
}

fn parse_variable_base(
    tokens: &[PositionToken],
    start: &PositionToken,
    annotated: &[AstAnnotated],
    fin: bool,
    jtype: &AstJType,
    pos: usize,
) -> Result<(AstBlockVariable, usize), AstError> {
    let mut jtype = jtype.clone();
    let (name, pos) = parse_name(tokens, pos)?;
    let mut pos = parse_array_type_on_name(tokens, pos, &mut jtype);
    let mut value = None;
    if let Ok(npos) = assert_token(tokens, pos, Token::Equal) {
        pos = npos;
        // optional when typing `var = `
        if let Ok((aexpression, npos)) = parse_expression(tokens, npos, &ExpressionOptions::None) {
            pos = npos;
            value = Some(aexpression);
        }
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstBlockVariable {
            range: AstRange::from_position_token(start, end),
            fin,
            annotated: annotated.to_owned(),
            jtype,
            name,
            value,
        },
        pos,
    ))
}

fn parse_block_variable_multi_type_no_semicolon(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockVariableMutliType, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut fin = false;
    if let Ok(npos) = assert_token(tokens, pos, Token::Final) {
        fin = true;
        pos = npos;
    }
    let mut jtypes = vec![];
    loop {
        let (jtype, npos) = parse_jtype(tokens, pos)?;
        jtypes.push(jtype);
        pos = npos;
        if let Ok(npos) = assert_token(tokens, pos, Token::VerticalBar) {
            pos = npos;
            continue;
        } else {
            break;
        }
    }
    let (name, pos) = parse_name(tokens, pos)?;
    let mut expression = None;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Equal) {
        let (aexpression, npos) = parse_expression(tokens, npos, &ExpressionOptions::None)?;
        pos = npos;
        expression = Some(aexpression);
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

    Ok((
        AstBlockVariableMutliType {
            name,
            fin,
            jtypes,
            expression,
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}

/// `return 1;`
pub fn parse_block_return(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockReturn, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Return)?;
    let mut pos = pos;
    let mut expression = AstExpressionOrValue::None;
    if let Ok(npos) = assert_token(tokens, pos, Token::Semicolon) {
        pos = npos;
    } else {
        let (nexpression, npos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
        pos = npos;
        expression = AstExpressionOrValue::Expression(Box::new(nexpression));
    }
    let pos = assert_semicolon(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

    Ok((
        AstBlockReturn {
            range: AstRange::from_position_token(start, end),
            expression,
        },
        pos,
    ))
}
fn parse_block_yield(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockYield, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Yield)?;
    let mut pos = pos;
    let mut expression = AstExpressionOrValue::None;
    if let Ok((nexpression, npos)) = parse_expression(tokens, pos, &ExpressionOptions::None) {
        pos = npos;
        expression = AstExpressionOrValue::Expression(Box::new(nexpression));
    }
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos).ok_or(AstError::eof())?;

    Ok((
        AstBlockYield {
            range: AstRange::from_position_token(start, end),
            expression,
        },
        pos,
    ))
}
fn parse_block_break(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockBreak, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = assert_token(tokens, pos, Token::Break)?;
    let mut label = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        label = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

    Ok((
        AstBlockBreak {
            range: AstRange::from_position_token(start, end),
            label,
        },
        pos,
    ))
}
fn parse_block_continue(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockContinue, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Continue)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

    Ok((
        AstBlockContinue {
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}
fn parse_block_expression_options(
    tokens: &[PositionToken],
    pos: usize,
    block_entry_options: &BlockEntryOptions,
) -> Result<(AstBlockExpression, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (value, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_semicolon_options(tokens, pos, block_entry_options)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

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
    block_entry_options: &BlockEntryOptions,
) -> Result<(AstBlockAssign, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (key, pos) = parse_recursive_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_token(tokens, pos, Token::Equal)?;
    let (expression, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_semicolon_options(tokens, pos, block_entry_options)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;

    Ok((
        AstBlockAssign {
            range: AstRange::from_position_token(start, end),
            key,
            expression,
        },
        pos,
    ))
}

fn parse_method_header(
    tokens: &[PositionToken],
    pos: usize,
    default_availability: AstAvailability,
) -> Result<(AstMethodHeader, usize), AstError> {
    let mut avaliability = default_availability;
    let mut type_parameters = None;
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut pos = pos;
    if let Ok((avav, npos)) = parse_avaliability(tokens, pos) {
        avaliability = avav;
        pos = npos;
    };
    loop {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match t.token {
            Token::Synchronized => avaliability |= AstAvailability::Synchronized,
            Token::Static => avaliability |= AstAvailability::Static,
            Token::Final => avaliability |= AstAvailability::Final,
            Token::Abstract => avaliability |= AstAvailability::Abstract,
            Token::Native => avaliability |= AstAvailability::Native,
            _ => break,
        }
        pos += 1;
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
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstMethodHeader {
            range: AstRange::from_position_token(start, end),
            avaliability,
            type_parameters,
            name,
            annotated,
            jtype,
            parameters,
            throws,
        },
        pos,
    ))
}

/// `byte b[]` is modified to be the correct jtype
fn parse_array_type_on_name(tokens: &[PositionToken], pos: usize, jtype: &mut AstJType) -> usize {
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::LeftParenSquare)
        && let Ok(npos) = assert_token(tokens, npos, Token::RightParenSquare)
    {
        pos = npos;
        let orig = jtype.clone();
        *jtype = AstJType {
            range: jtype.range,
            value: AstJTypeKind::Array(Box::new(orig)),
        };
    }
    pos
}
fn parse_constructor_header(
    tokens: &[PositionToken],
    pos: usize,
    default_availability: AstAvailability,
) -> Result<(AstConstructorHeader, usize), AstError> {
    let mut avaliability = default_availability;
    let mut type_parameters = None;
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut pos = pos;
    if let Ok((avav, npos)) = parse_avaliability(tokens, pos) {
        avaliability = avav;
        pos = npos;
    };
    loop {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        match t.token {
            Token::Synchronized => avaliability |= AstAvailability::Synchronized,
            Token::Static => avaliability |= AstAvailability::Static,
            Token::Final => avaliability |= AstAvailability::Final,
            _ => break,
        }
        pos += 1;
    }

    if let Ok((type_params, npos)) = parse_type_parameters(tokens, pos) {
        type_parameters = Some(type_params);
        pos = npos;
    };

    let (name, pos) = parse_name(tokens, pos)?;
    let (parameters, pos) = parse_method_paramerters(tokens, pos)?;
    let mut pos = pos;
    let mut throws = None;
    if let Ok((nthrows, npos)) = parse_throws_declaration(tokens, pos) {
        throws = Some(nthrows);
        pos = npos;
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstConstructorHeader {
            range: AstRange::from_position_token(start, end),
            avaliability,
            type_parameters,
            name,
            parameters,
            throws,
            annotated,
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
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstThrowsDeclaration {
            range: AstRange::from_position_token(start, end),
            parameters,
        },
        pos,
    ))
}

/// `public void <T> do() { ... }`
///              ^^^
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

        let start_p = tokens.get(pos).ok_or(AstError::eof())?;
        let (name, npos) = parse_name(tokens, pos)?;
        pos = npos;
        let mut supperclass = None;
        if let Ok((s, npos)) = parse_superclass(tokens, pos) {
            supperclass = Some(s);
            pos = npos;
        }
        let end_p = tokens.get(pos).ok_or(AstError::eof())?;
        parameters.push(AstTypeParameter {
            range: AstRange::from_position_token(start_p, end_p),
            name,
            supperclass,
        });
        if let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
            pos = npos;
            continue;
        }
        if let Ok(npos) = assert_token(tokens, pos, Token::Gt) {
            pos = npos;
            break;
        }
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
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
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
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

/// { statements; }
pub fn parse_block(tokens: &[PositionToken], pos: usize) -> Result<(AstBlock, usize), AstError> {
    parse_block_brackets(tokens, pos, Token::LeftParenCurly, Token::RightParenCurly)
}
fn parse_block_brackets(
    tokens: &[PositionToken],
    pos: usize,
    left: Token,
    right: Token,
) -> Result<(AstBlock, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, left)?;
    let mut pos = pos;
    let mut entries = vec![];
    let mut start_pos;
    loop {
        if let Ok(npos) = assert_token(tokens, pos, right.clone()) {
            pos = npos;
            break;
        };
        start_pos = pos;
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                entries.push(entry);
                pos = npos;
            }
            Err(e) => {
                return Err(e);
            }
        }
        if pos == start_pos {
            eprintln!("No block enty was parsed: {:?}", tokens.get(pos));
            break;
        }
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof()).unwrap();
    Ok((
        AstBlock {
            range: AstRange::from_position_token(start, end),
            entries,
        },
        pos,
    ))
}

/// Options for expression parsing
#[derive(Debug, PartialEq)]
pub enum BlockEntryOptions {
    /// Default expression
    None,
    /// Don't parse `;`
    NoSemicolon,
}
fn parse_block_entry(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstBlockEntry, usize), AstError> {
    parse_block_entry_options(tokens, pos, &BlockEntryOptions::None)
}
fn parse_block_entry_options(
    tokens: &[PositionToken],
    pos: usize,
    block_entry_options: &BlockEntryOptions,
) -> Result<(AstBlockEntry, usize), AstError> {
    let mut errors = vec![];
    match parse_block(tokens, pos) {
        Ok((block, pos)) => {
            return Ok((AstBlockEntry::Block(block), pos));
        }
        Err(e) => {
            errors.push(("block block".into(), e));
        }
    }
    match parse_block_variable_options(tokens, pos, block_entry_options) {
        Ok((vars, pos)) => {
            return Ok((AstBlockEntry::Variable(vars), pos));
        }
        Err(e) => {
            errors.push(("block variable".into(), e));
        }
    }
    match parse_block_return(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Return(nret), pos));
        }
        Err(e) => {
            errors.push(("block return".into(), e));
        }
    }
    match parse_block_yield(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Yield(nret), pos));
        }
        Err(e) => {
            errors.push(("block yield".into(), e));
        }
    }
    match parse_block_break(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Break(nret), pos));
        }
        Err(e) => {
            errors.push(("block break".into(), e));
        }
    }
    match parse_block_continue(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Continue(nret), pos));
        }
        Err(e) => {
            errors.push(("block continue".into(), e));
        }
    }
    match parse_if(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::If(nret), pos));
        }
        Err(e) => {
            errors.push(("block if".into(), e));
        }
    }
    match parse_while(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::While(nret), pos));
        }
        Err(e) => {
            errors.push(("block while".into(), e));
        }
    }
    match parse_do_while(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::While(nret), pos));
        }
        Err(e) => {
            errors.push(("block do while".into(), e));
        }
    }
    match parse_for(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::For(Box::new(nret)), pos));
        }
        Err(e) => {
            errors.push(("block for".into(), e));
        }
    }
    match parse_for_enhanced(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::ForEnhanced(Box::new(nret)), pos));
        }
        Err(e) => {
            errors.push(("block for enhanced".into(), e));
        }
    }
    match parse_switch(tokens, pos, &ExpressionOptions::None) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Switch(nret), pos));
        }
        Err(e) => {
            errors.push(("block switch".into(), e));
        }
    }
    match parse_switch_case(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::SwitchCase(nret), pos));
        }
        Err(e) => {
            errors.push(("block switch case".into(), e));
        }
    }
    match parse_switch_case_arrow(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::SwitchCaseArrow(nret), pos));
        }
        Err(e) => {
            errors.push(("block switch case arrow".into(), e));
        }
    }
    match parse_switch_case_arrow_default(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::SwitchCaseArrowDefault(nret), pos));
        }
        Err(e) => {
            errors.push(("block switch case arrow".into(), e));
        }
    }
    match parse_switch_default(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::SwitchDefault(nret), pos));
        }
        Err(e) => {
            errors.push(("block switch default".into(), e));
        }
    }
    match parse_try_catch(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::TryCatch(nret), pos));
        }
        Err(e) => {
            errors.push(("block try catch".into(), e));
        }
    }
    match parse_throw(tokens, pos) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Throw(nret), pos));
        }
        Err(e) => {
            errors.push(("block throw".into(), e));
        }
    }
    match parse_synchronised_block(tokens, pos) {
        Ok((synchronized_block, pos)) => {
            return Ok((AstBlockEntry::SynchronizedBlock(synchronized_block), pos));
        }
        Err(e) => {
            errors.push(("static block".into(), e));
        }
    }
    match parse_block_assign(tokens, pos, block_entry_options) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Assign(Box::new(nret)), pos));
        }
        Err(e) => {
            errors.push(("block assign".into(), e));
        }
    }
    match parse_thing(tokens, pos) {
        Ok((thing, pos)) => {
            return Ok((AstBlockEntry::Thing(Box::new(thing)), pos));
        }
        Err(e) => {
            errors.push(("class thing".into(), e));
        }
    }
    match parse_block_expression_options(tokens, pos, block_entry_options) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Expression(nret), pos));
        }
        Err(e) => {
            errors.push(("block expression".into(), e));
        }
    }
    Err(AstError::AllChildrenFailed {
        parent: "block".into(),
        errors,
    })
}
fn parse_block_entry_minimal_options(
    tokens: &[PositionToken],
    pos: usize,
    block_entry_options: &BlockEntryOptions,
) -> Result<(AstBlockEntry, usize), AstError> {
    let mut errors = vec![];
    match parse_block_variable_options(tokens, pos, block_entry_options) {
        Ok((variable, pos)) => {
            return Ok((AstBlockEntry::Variable(variable), pos));
        }
        Err(e) => {
            errors.push(("block variable".into(), e));
        }
    }
    match parse_block_assign(tokens, pos, block_entry_options) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Assign(Box::new(nret)), pos));
        }
        Err(e) => {
            errors.push(("block assign".into(), e));
        }
    }
    match parse_block_expression_options(tokens, pos, block_entry_options) {
        Ok((nret, pos)) => {
            return Ok((AstBlockEntry::Expression(nret), pos));
        }
        Err(e) => {
            errors.push(("block expression".into(), e));
        }
    }
    Err(AstError::AllChildrenFailed {
        parent: "block".into(),
        errors,
    })
}

fn parse_while(tokens: &[PositionToken], pos: usize) -> Result<(AstWhile, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut label = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        let npos = assert_token(tokens, npos, Token::Colon)?;

        label = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::While)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (control, pos) = parse_recursive_expression(tokens, pos, &ExpressionOptions::None)?;
    let mut pos = assert_token(tokens, pos, Token::RightParen)?;
    let mut content = AstWhileContent::None;
    let mut errors = vec![];
    'while_content: {
        match assert_token(tokens, pos, Token::Semicolon) {
            Ok(npos) => {
                pos = npos;
                break 'while_content;
            }
            Err(e) => errors.push(("semicolon".into(), e)),
        }
        match parse_block(tokens, pos) {
            Ok((block, npos)) => {
                content = AstWhileContent::Block(block);
                pos = npos;
                break 'while_content;
            }
            Err(e) => errors.push(("block".into(), e)),
        }
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                content = AstWhileContent::BlockEntry(Box::new(entry));
                pos = npos;
                break 'while_content;
            }
            Err(e) => errors.push(("recursive expression".into(), e)),
        }
        return Err(AstError::AllChildrenFailed {
            parent: "while".into(),
            errors,
        });
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstWhile {
            range: AstRange::from_position_token(start, end),
            control,
            content,
            label,
        },
        pos,
    ))
}
fn parse_do_while(tokens: &[PositionToken], pos: usize) -> Result<(AstWhile, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut label = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        let npos = assert_token(tokens, npos, Token::Colon)?;

        label = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::Do)?;
    let (block, pos) = parse_block(tokens, pos)?;

    let pos = assert_token(tokens, pos, Token::While)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (control, pos) = parse_recursive_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let pos = assert_token(tokens, pos, Token::Semicolon)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstWhile {
            range: AstRange::from_position_token(start, end),
            control,
            content: AstWhileContent::Block(block),
            label,
        },
        pos,
    ))
}
/// `for(;;) { ... }`
pub fn parse_for(tokens: &[PositionToken], pos: usize) -> Result<(AstFor, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut label = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        let npos = assert_token(tokens, npos, Token::Colon)?;

        label = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::For)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let mut vars = vec![];
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Semicolon) {
        pos = npos
    } else {
        let (v, npos) = parse_comma_separated_block_entry(tokens, pos)?;
        let npos = assert_token(tokens, npos, Token::Semicolon)?;
        vars = v;
        pos = npos;
    }
    let mut check = vec![];
    if let Ok(npos) = assert_token(tokens, pos, Token::Semicolon) {
        pos = npos
    } else {
        let (c, npos) = parse_comma_separated_block_entry(tokens, pos)?;
        let npos = assert_token(tokens, npos, Token::Semicolon)?;
        check = c;
        pos = npos;
    }
    let mut changes = vec![];
    if let Ok(npos) = assert_token(tokens, pos, Token::RightParen) {
        pos = npos
    } else {
        let (c, npos) = parse_comma_separated_block_entry(tokens, pos)?;
        let npos = assert_token(tokens, npos, Token::RightParen)?;
        changes = c;
        pos = npos;
    }
    let content;
    let mut errors = vec![];
    'for_content: {
        match parse_block(tokens, pos) {
            Ok((block, npos)) => {
                content = AstForContent::Block(block);
                pos = npos;
                break 'for_content;
            }
            Err(e) => errors.push(("block".into(), e)),
        }
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                content = AstForContent::BlockEntry(Box::new(entry));
                pos = npos;
                break 'for_content;
            }
            Err(e) => errors.push(("recursive expression".into(), e)),
        }
        return Err(AstError::AllChildrenFailed {
            parent: "for".into(),
            errors,
        });
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstFor {
            range: AstRange::from_position_token(start, end),
            content,
            label,
            vars,
            check,
            changes,
        },
        pos,
    ))
}

fn parse_comma_separated_block_entry(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstBlockEntry>, usize), AstError> {
    let mut vars = vec![];
    let options = BlockEntryOptions::NoSemicolon;
    let (e, pos) = parse_block_entry_minimal_options(tokens, pos, &options)?;
    vars.push(e);
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
        let (e, npos) = parse_block_entry_minimal_options(tokens, npos, &options)?;
        vars.push(e);
        pos = npos;
    }
    Ok((vars, pos))
}

fn parse_switch(
    tokens: &[PositionToken],
    pos: usize,
    expression_options: &ExpressionOptions,
) -> Result<(AstSwitch, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Switch)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (check, pos) = parse_expression(tokens, pos, expression_options)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstSwitch {
            range: AstRange::from_position_token(start, end),
            block,
            check: Box::new(check),
        },
        pos,
    ))
}
fn parse_switch_case(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchCase, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Case)?;
    let (expression, pos) = parse_expression(tokens, pos, &ExpressionOptions::NoInlineIf)?;
    let pos = assert_token(tokens, pos, Token::Colon)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstSwitchCase {
            range: AstRange::from_position_token(start, end),
            expression,
        },
        pos,
    ))
}
fn parse_switch_case_arrow(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchCaseArrow, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = assert_token(tokens, pos, Token::Case)?;
    let mut values = vec![];
    loop {
        let (value, npos) = parse_expression(tokens, pos, &ExpressionOptions::NoLambda)?;
        values.push(value);
        pos = npos;

        match assert_token(tokens, pos, Token::Comma) {
            Ok(npos) => {
                pos = npos;
            }
            Err(_) => break,
        }
    }
    let pos = assert_token(tokens, pos, Token::Arrow)?;
    let (content, pos) = parse_switch_case_arrow_content(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstSwitchCaseArrow {
            range: AstRange::from_position_token(start, end),
            values,
            content: Box::new(content),
        },
        pos,
    ))
}
fn parse_switch_case_arrow_default(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchCaseArrowDefault, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Default)?;
    let pos = assert_token(tokens, pos, Token::Arrow)?;
    let (content, pos) = parse_switch_case_arrow_content(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstSwitchCaseArrowDefault {
            range: AstRange::from_position_token(start, end),
            content: Box::new(content),
        },
        pos,
    ))
}

fn parse_switch_case_arrow_content(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchCaseArrowContent, usize), AstError> {
    let content;
    let mut pos = pos;
    if assert_token(tokens, pos, Token::LeftParenCurly).is_ok() {
        let (block, npos) = parse_block(tokens, pos)?;
        content = AstSwitchCaseArrowContent::Block(block);
        pos = npos
    } else {
        let (entry, npos) = parse_block_entry(tokens, pos)?;
        content = AstSwitchCaseArrowContent::Entry(Box::new(entry));
        pos = npos
    }
    Ok((content, pos))
}
fn parse_switch_default(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSwitchDefault, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Default)?;
    let pos = assert_token(tokens, pos, Token::Colon)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstSwitchDefault {
            range: AstRange::from_position_token(start, end),
        },
        pos,
    ))
}

fn parse_for_enhanced(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstForEnhanced, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut label = None;
    if let Ok((lab, npos)) = parse_name(tokens, pos) {
        let npos = assert_token(tokens, npos, Token::Colon)?;

        label = Some(lab);
        pos = npos;
    }
    let pos = assert_token(tokens, pos, Token::For)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (var, pos) = parse_block_variable_no_semicolon(tokens, pos)?;
    let pos = assert_token(tokens, pos, Token::Colon)?;
    let (rhs, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let content;
    let mut errors = vec![];
    let mut pos = pos;
    'for_content: {
        match parse_block(tokens, pos) {
            Ok((block, npos)) => {
                content = AstForContent::Block(block);
                pos = npos;
                break 'for_content;
            }
            Err(e) => errors.push(("block".into(), e)),
        }
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                content = AstForContent::BlockEntry(Box::new(entry));
                pos = npos;
                break 'for_content;
            }
            Err(e) => errors.push(("recursive expression".into(), e)),
        }
        return Err(AstError::AllChildrenFailed {
            parent: "for".into(),
            errors,
        });
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstForEnhanced {
            range: AstRange::from_position_token(start, end),
            content,
            label,
            var,
            rhs,
        },
        pos,
    ))
}

fn parse_if(tokens: &[PositionToken], pos: usize) -> Result<(AstIf, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::If)?;
    let start_control = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (control, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let end_control = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let mut pos = pos;
    let content;
    let mut errors = vec![];
    'if_content: {
        match parse_block(tokens, pos) {
            Ok((block, npos)) => {
                content = AstIfContent::Block(block);
                pos = npos;
                break 'if_content;
            }
            Err(e) => errors.push(("block".into(), e)),
        }
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                content = AstIfContent::BlockEntry(Box::new(entry));
                pos = npos;
                break 'if_content;
            }
            Err(e) => errors.push(("recursive expression".into(), e)),
        }
        return Err(AstError::AllChildrenFailed {
            parent: "if".into(),
            errors,
        });
    }
    let mut el = None;
    if let Ok(npos) = assert_token(tokens, pos, Token::Else) {
        pos = npos;
        let mut errors = vec![];

        match parse_if(tokens, pos) {
            Ok((new_class, npos)) => {
                el = Some(Box::new(new_class));
                pos = npos;
            }
            Err(e) => errors.push(("if nested".into(), e)),
        }
        match parse_block(tokens, pos) {
            Ok((block, npos)) => {
                el = Some(Box::new(AstIf::Else {
                    range: block.range,
                    content: AstIfContent::Block(block),
                }));
                pos = npos;
            }
            Err(e) => errors.push(("block".into(), e)),
        }
        match parse_block_entry(tokens, pos) {
            Ok((entry, npos)) => {
                el = Some(Box::new(AstIf::Else {
                    range: entry.get_range(),
                    content: AstIfContent::BlockEntry(Box::new(entry)),
                }));
                pos = npos;
            }
            Err(e) => errors.push(("recursive expression".into(), e)),
        }
        if el.is_none() {
            return Err(AstError::AllChildrenFailed {
                parent: "ifelse".into(),
                errors,
            });
        }
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstIf::If {
            range: AstRange::from_position_token(start, end),
            control: Box::new(control),
            control_range: AstRange::from_position_token(start_control, end_control),
            content,
            el,
        },
        pos,
    ))
}
fn parse_throw(tokens: &[PositionToken], pos: usize) -> Result<(AstThrow, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Throw)?;
    let (expression, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_semicolon(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstThrow {
            range: AstRange::from_position_token(start, end),
            expression,
        },
        pos,
    ))
}
fn parse_synchronised_block(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstSynchronizedBlock, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Synchronized)?;
    let pos = assert_token(tokens, pos, Token::LeftParen)?;
    let (expression, pos) = parse_expression(tokens, pos, &ExpressionOptions::None)?;
    let pos = assert_token(tokens, pos, Token::RightParen)?;
    let (block, pos) = parse_block(tokens, pos)?;
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstSynchronizedBlock {
            range: AstRange::from_position_token(start, end),
            expression,
            block,
        },
        pos,
    ))
}

fn parse_try_catch(tokens: &[PositionToken], pos: usize) -> Result<(AstTryCatch, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let pos = assert_token(tokens, pos, Token::Try)?;
    let mut resources_block = None;
    let mut pos = pos;
    if let Ok((res, npos)) = parse_block_brackets(tokens, pos, Token::LeftParen, Token::RightParen)
    {
        resources_block = Some(res);
        pos = npos;
    }
    let (block, pos) = parse_block(tokens, pos)?;
    let mut pos = pos;
    let mut finally_block = None;
    let mut cases = vec![];
    while assert_token(tokens, pos, Token::Catch).is_ok() {
        pos += 1;
        let start = tokens.get(pos).ok_or(AstError::eof())?;
        let npos = assert_token(tokens, pos, Token::LeftParen)?;
        let (variable, npos) = parse_block_variable_multi_type_no_semicolon(tokens, npos)?;
        let npos = assert_token(tokens, npos, Token::RightParen)?;
        let (block, npos) = parse_block(tokens, npos)?;
        pos = npos;
        let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
        cases.push(AstTryCatchCase {
            range: AstRange::from_position_token(start, end),
            variable,
            block,
        });
    }
    if let Ok(npos) = assert_token(tokens, pos, Token::Finally) {
        let (f_block, npos) = parse_block(tokens, npos)?;
        finally_block = Some(f_block);
        pos = npos;
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstTryCatch {
            range: AstRange::from_position_token(start, end),
            resources_block,
            block,
            cases,
            finally_block,
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
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
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
    let mut fin = false;
    let mut variatic = false;
    let (annotated, pos) = parse_annotated_list(tokens, pos)?;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Final) {
        fin = true;
        pos = npos;
    }
    let (mut jtype, pos) = parse_jtype(tokens, pos)?;
    let mut pos = pos;
    if let Ok(npos) = assert_token(tokens, pos, Token::Dot)
        && let Ok(npos) = assert_token(tokens, npos, Token::Dot)
        && let Ok(npos) = assert_token(tokens, npos, Token::Dot)
    {
        pos = npos;
        variatic = true;
    }
    let (name, pos) = parse_name(tokens, pos)?;
    let pos = parse_array_type_on_name(tokens, pos, &mut jtype);
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstMethodParamerter {
            range: AstRange::from_position_token(start, end),
            annotated,
            jtype,
            name,
            fin,
            variatic,
        },
        pos,
    ))
}

/// `thing1_`
pub fn parse_name(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let value;
    let mut pos = pos;
    if let Token::Identifier(i) = &start.token {
        value = i.clone();
        pos += 1;
    } else {
        return Err(AstError::InvalidName(InvalidToken::from(start, pos)));
    }
    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
    Ok((
        AstIdentifier {
            range: AstRange::from_position_token(start, end),
            value,
        },
        pos,
    ))
}

/// `thing.1`
pub fn parse_name_dot(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let init_pos = pos;
    let mut pos = pos;
    let mut ident = SmolStrBuilder::new();
    loop {
        let Ok(t) = tokens.get(pos).ok_or(AstError::eof()) else {
            break;
        };
        match &t.token {
            Token::Identifier(id) => {
                ident.push_str(id);
                pos += 1;
            }
            Token::Dot => {
                ident.push('.');
                pos += 1;
            }
            _ => {
                if pos == init_pos
                    && let Ok(t) = tokens.get(pos).ok_or(AstError::eof())
                {
                    return Err(AstError::InvalidName(InvalidToken::from(t, pos)));
                }
                break;
            }
        }
    }
    let ident = ident.finish();
    if ident.is_empty() {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        return Err(AstError::IdentifierEmpty(InvalidToken::from(t, pos)));
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
/// `thing.1`
pub fn parse_name_dot_logical(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let init_pos = pos;
    let mut pos = pos;
    let mut ident = SmolStrBuilder::new();
    let mut first = true;
    loop {
        let Ok(t) = tokens.get(pos).ok_or(AstError::eof()) else {
            break;
        };
        match &t.token {
            Token::Identifier(id) => {
                if first {
                    first = false;
                    ident.push_str(id);
                    pos += 1;
                } else if tokens.get(pos + 1).ok_or(AstError::eof()).map(|i| &i.token)
                    == Ok(&Token::Dot)
                    || tokens.get(pos - 1).ok_or(AstError::eof()).map(|i| &i.token)
                        == Ok(&Token::Dot)
                {
                    ident.push_str(id);
                    pos += 1;
                    continue;
                } else {
                    break;
                }
            }
            Token::Dot => {
                if let Ok(Token::Identifier(_)) =
                    tokens.get(pos + 1).ok_or(AstError::eof()).map(|i| &i.token)
                {
                    ident.push('.');
                    pos += 1;
                    continue;
                } else {
                    break;
                }
            }
            _ => {
                if pos == init_pos
                    && let Ok(t) = tokens.get(pos).ok_or(AstError::eof())
                {
                    return Err(AstError::InvalidName(InvalidToken::from(t, pos)));
                }
                break;
            }
        }
    }
    let ident = ident.finish();
    if ident.is_empty() {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        return Err(AstError::IdentifierEmpty(InvalidToken::from(t, pos)));
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
/// `thing`
pub fn parse_name_single(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstIdentifier, usize), AstError> {
    let start = tokens.get(pos).ok_or(AstError::eof())?;
    let mut pos = pos;
    let mut ident = None;
    let t = tokens.get(pos).ok_or(AstError::eof())?;
    match &t.token {
        Token::Identifier(id) => {
            ident = Some(id);
            pos += 1;
        }
        _ => {
            if let Ok(t) = tokens.get(pos).ok_or(AstError::eof()) {
                return Err(AstError::InvalidName(InvalidToken::from(t, pos)));
            }
        }
    }
    let Some(ident) = ident else {
        let t = tokens.get(pos).ok_or(AstError::eof())?;
        return Err(AstError::IdentifierEmpty(InvalidToken::from(t, pos)));
    };
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
        return Err(AstError::IdentifierEmpty(InvalidToken::from(t, pos)));
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

fn parse_implements(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstJType>, usize), AstError> {
    let Ok(pos) = assert_token(tokens, pos, Token::Implements) else {
        return Ok((vec![], pos));
    };
    let (out, pos) = parse_comma_seperated_jtype(tokens, pos)?;

    Ok((out, pos))
}

fn parse_permits(tokens: &[PositionToken], pos: usize) -> Result<(Vec<AstJType>, usize), AstError> {
    let Ok(pos) = assert_token(tokens, pos, Token::Permits) else {
        return Ok((vec![], pos));
    };
    let (out, pos) = parse_comma_seperated_jtype(tokens, pos)?;

    Ok((out, pos))
}

fn parse_comma_seperated_jtype(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(Vec<AstJType>, usize), AstError> {
    let mut out = vec![];
    let (jtype, pos) = parse_jtype(tokens, pos)?;
    out.push(jtype);
    let mut pos = pos;
    while let Ok(npos) = assert_token(tokens, pos, Token::Comma) {
        let (jtype, npos) = parse_jtype(tokens, npos)?;
        out.push(jtype);
        pos = npos;
    }
    Ok((out, pos))
}

/// String
/// int
pub fn parse_jtype(tokens: &[PositionToken], pos: usize) -> Result<(AstJType, usize), AstError> {
    let out_pos = pos + 1;
    let current = tokens.get(pos).ok_or(AstError::eof())?;
    let base = match &current.token {
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
        Token::Var => Ok((
            AstJType {
                range: AstRange::from_position_token(current, current),
                value: AstJTypeKind::Var,
            },
            out_pos,
        )),
        Token::Identifier(ident) => {
            let token = tokens.get(pos + 1).ok_or(AstError::eof())?;
            let range = AstRange::from_position_token(current, current);
            let ident = AstIdentifier {
                value: ident.clone(),
                range,
            };
            match token.token {
                Token::Dot => {
                    if let Ok((inner, npos)) = parse_jtype(tokens, pos + 2) {
                        return Ok((
                            AstJType {
                                range: AstRange {
                                    start: range.start,
                                    end: inner.range.end,
                                },
                                value: AstJTypeKind::Access {
                                    ident,
                                    inner: Box::new(inner),
                                },
                            },
                            npos,
                        ));
                    }
                    Ok((
                        AstJType {
                            range,
                            value: AstJTypeKind::Class(ident),
                        },
                        pos + 1,
                    ))
                }
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
                    let end = tokens.get(pos - 1).ok_or(AstError::eof())?;
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
        _ => Err(AstError::InvalidJtype(InvalidToken::from(current, pos))),
    };
    let (base, pos) = base?;

    let mut pos = pos;
    let mut base = base;

    while let Ok(npos) = assert_token(tokens, pos, Token::LeftParenSquare) {
        if let Ok(npos) = assert_token(tokens, npos, Token::RightParenSquare) {
            pos = npos;
            base = AstJType {
                range: AstRange::from_position_token(current, current),
                value: AstJTypeKind::Array(Box::new(base)),
            };
        } else {
            break;
        }
    }
    Ok((base, pos))
}

fn parse_avaliability(
    tokens: &[PositionToken],
    pos: usize,
) -> Result<(AstAvailability, usize), AstError> {
    let Some(token) = tokens.get(pos) else {
        return Err(AstError::eof());
    };
    let mut pos = pos;
    let mut out = AstAvailability::empty();
    match token.token {
        Token::Public => {
            out = AstAvailability::Public;
            pos += 1;
        }
        Token::Protected => {
            out = AstAvailability::Protected;
            pos += 1;
        }
        Token::Private => {
            out = AstAvailability::Private;
            pos += 1;
        }
        _ => (),
    }
    Ok((out, pos))
}
